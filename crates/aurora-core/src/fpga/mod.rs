//! Placas FPGA: compilar um projeto para uma placa e gravar nela.
//!
//! O projeto diz a placa e a ligação das portas do topo aos sinais da placa
//! no `fpga.json` ([`config`]), ao lado do `.spf`; a AURORA não o lê. A AURORA
//! gera um topo da placa ([`wrapper`]) que instancia o topo do projeto com
//! essas ligações (invertendo, cortando e completando larguras), e o
//! compila ([`build()`]) e grava na placa ([`program()`]) pelo Quartus Prime
//! instalado no sistema ([`quartus`]), sem abrir a interface dele. É a segunda exceção à regra do
//! bundle, depois do compilador C++ do Verilator: o Quartus é proprietário,
//! grande e não pode ir no bundle. Um fluxo aberto para o Cyclone V da
//! DE10-Nano (Yosys e `nextpnr-mistral`) ficou para depois.
//!
//! As placas ([`mod@board`]) vêm com o Core, com os pinos tirados dos manuais do
//! fabricante.

pub mod board;
pub mod build;
pub mod config;
mod manifest;
mod pins;
pub mod program;
pub mod qsf;
pub mod quartus;
pub mod summary;
pub mod wrapper;

pub use board::{
    Board, BoardSignal, Cable, Device, IoStandard, Jtag, LedColor, Peripheral, SignalDirection,
    board, boards,
};
pub use build::{FpgaBuildResult, bitstream_path, build, build_dir};
pub use config::{CONFIG_FILE, FpgaConfig, Link, PortChoice, Reconnect, Resolved};
pub use program::{BitstreamState, FpgaProgramResult, bitstream_state, cables, program};
pub use quartus::Quartus;
pub use summary::{ClockTiming, ResourceUsage, TimingSummary};

use camino::Utf8PathBuf;
use schemars::JsonSchema;
use serde::Serialize;

use crate::error::{AuroraError, Result};
use crate::project::Project;
use crate::toolchain::Toolchain;
use crate::verilog::ModuleInterface;

/// O `fpga.json` conferido: a placa, as ligações bit a bit e o topo da
/// placa gerado.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct Prepared {
    /// O `fpga.json`.
    #[schemars(with = "String")]
    pub config: Utf8PathBuf,
    /// As ligações conferidas.
    pub resolved: Resolved,
    /// O Verilog do topo da placa ([`wrapper::generate`]).
    pub board_top: String,
    /// Cada ligação como texto ([`wrapper::connections`]): cada entrada do
    /// topo e cada sinal de saída da placa usado, com o que o dirige.
    pub connections: Vec<Connection>,
}

/// Uma ligação como texto, para mostrar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct Connection {
    /// A entrada do topo (`in[15:0]`) ou o sinal de saída da placa
    /// (`LEDR[17:0]`).
    pub target: String,
    /// O que a dirige (`~KEY[0]`, `{10'b0, out[7:0]}`).
    pub source: String,
}

/// Lê o `fpga.json` do projeto e a placa, acha o topo e as portas dele e
/// confere as ligações.
///
/// O topo é o do `fpga.json`; sem ele, o topo do projeto; sem esse, o
/// processador, se o projeto tiver um só. As portas vêm do Verilog (o Yosys
/// do bundle, se houver): um processador SAPHO precisa estar compilado.
///
/// # Erros
///
/// [`AuroraError::NoFpgaConfig`] e [`AuroraError::InvalidFpgaConfig`] (com um
/// problema por linha), [`AuroraError::BoardNotFound`],
/// [`AuroraError::NoTopLevel`] e [`AuroraError::ModuleNotFound`].
pub fn prepare(toolchain: Option<&Toolchain>, project: &Project) -> Result<Prepared> {
    prepare_with(toolchain, project, FpgaConfig::read(project.root())?)
}

/// [`prepare`] com uma configuração dada, em vez da do `fpga.json` (a placa
/// virtual, num projeto sem `fpga.json`, liga as portas pelo nome com
/// [`FpgaConfig::by_name`]). Os erros citam o caminho do `fpga.json`.
///
/// # Erros
///
/// Os de [`prepare`], menos os de ler o `fpga.json`.
pub fn prepare_with(
    toolchain: Option<&Toolchain>,
    project: &Project,
    config: FpgaConfig,
) -> Result<Prepared> {
    let path = FpgaConfig::path(project.root());
    let board = board::board(&config.board)?;
    let interface = top_interface(toolchain, project, config.top.as_deref())?;
    let resolved =
        config
            .resolve(&board, &interface)
            .map_err(|problems| AuroraError::InvalidFpgaConfig {
                path: path.clone(),
                reason: format!(
                    "{} problem{}\n  {}",
                    problems.len(),
                    if problems.len() == 1 { "" } else { "s" },
                    problems.join("\n  ")
                ),
            })?;
    let board_top = wrapper::generate(&resolved);
    let connections = wrapper::connections(&resolved)
        .into_iter()
        .map(|(target, source)| Connection { target, source })
        .collect();
    Ok(Prepared {
        config: path,
        resolved,
        board_top,
        connections,
    })
}

/// Muda uma ligação a partir da placa ([`FpgaConfig::reconnect`]) e grava o
/// `fpga.json`. Num projeto sem `fpga.json`, parte das ligações pelo nome com
/// a placa `board` ([`FpgaConfig::by_name`]), como a placa virtual. Devolve as
/// ligações gravadas.
///
/// # Erros
///
/// [`AuroraError::NoFpgaConfig`] sem `fpga.json` e sem `board`;
/// [`AuroraError::InvalidFpgaConfig`] com os problemas das ligações (as de
/// antes ou as novas), e os de [`prepare`] para a placa e o topo.
pub fn connect(
    toolchain: Option<&Toolchain>,
    project: &Project,
    board: Option<&str>,
    change: &Reconnect,
) -> Result<FpgaConfig> {
    let config = match FpgaConfig::read(project.root()) {
        Ok(config) => config,
        Err(AuroraError::NoFpgaConfig(path)) => {
            let Some(id) = board else {
                return Err(AuroraError::NoFpgaConfig(path));
            };
            let top = top_interface(toolchain, project, None)?;
            FpgaConfig::by_name(&board::board(id)?, &top)
        }
        Err(other) => return Err(other),
    };
    let found = board::board(&config.board)?;
    let top = top_interface(toolchain, project, config.top.as_deref())?;
    let next = config.reconnect(&found, &top, change).map_err(|problems| {
        AuroraError::InvalidFpgaConfig {
            path: FpgaConfig::path(project.root()),
            reason: format!(
                "{} problem{}
  {}",
                problems.len(),
                if problems.len() == 1 { "" } else { "s" },
                problems.join(
                    "
  "
                )
            ),
        }
    })?;
    next.write(project.root())?;
    Ok(next)
}

/// Os módulos que podem ir para a placa: os dos fontes do projeto (sem os
/// testbenches), na ordem dos arquivos. As portas vêm do Verilog, como em
/// [`top_interface`].
///
/// # Erros
///
/// Os de ler os fontes do projeto.
pub fn modules(toolchain: Option<&Toolchain>, project: &Project) -> Result<Vec<String>> {
    let files = crate::synth::project_sources(project)?;
    let mut names: Vec<String> =
        crate::verilog::read_interfaces_in(toolchain, &files, Some(project.root()))?
            .into_iter()
            .map(|i| i.name)
            .collect();
    for file in crate::ghdl::design_files(project)? {
        let text = std::fs::read_to_string(&file).map_err(AuroraError::io("Reading", &file))?;
        names.extend(crate::vhdl::entities_in(&text));
    }
    Ok(names)
}

/// Uma entidade VHDL do design e a interface dela, ou o arquivo e a porta
/// que não vira pinos.
type VhdlInterface = (
    String,
    std::result::Result<ModuleInterface, (Utf8PathBuf, String)>,
);

/// As interfaces das entidades VHDL do design, para a placa. Uma entidade
/// com porta que não vira pinos (um `integer`, um registro) fica com o
/// motivo, em vez da interface.
fn vhdl_interfaces(project: &Project) -> Result<Vec<VhdlInterface>> {
    let mut found = Vec::new();
    for file in crate::ghdl::design_files(project)? {
        let text = std::fs::read_to_string(&file).map_err(AuroraError::io("Reading", &file))?;
        for entity in crate::vhdl::entities(&text) {
            let interface = entity.interface(&file).map_err(|port| (file.clone(), port));
            found.push((entity.name.clone(), interface));
        }
    }
    Ok(found)
}

/// O módulo que vai para a placa e as portas dele: `top`, se dado (o do
/// `fpga.json`); senão o topo do projeto; senão o processador, se o projeto
/// tiver um só. As portas vêm do Verilog (o Yosys do bundle, se houver): um
/// processador SAPHO precisa estar compilado. Serve para quem monta as
/// ligações antes de elas estarem certas.
///
/// # Erros
///
/// [`AuroraError::NoTopLevel`] e [`AuroraError::ModuleNotFound`], e os de ler os
/// fontes do projeto.
pub fn top_interface(
    toolchain: Option<&Toolchain>,
    project: &Project,
    top: Option<&str>,
) -> Result<ModuleInterface> {
    let top = match top {
        Some(top) => top.to_owned(),
        None => match project.top_module()? {
            Some(top) => top,
            None => match project.processors() {
                [only] => only.name.clone(),
                _ => return Err(AuroraError::NoTopLevel(project.spf_path().to_owned())),
            },
        },
    };
    let files = crate::synth::project_sources(project)?;
    let mut interfaces =
        crate::verilog::read_interfaces_in(toolchain, &files, Some(project.root()))?;
    if let Some(at) = interfaces.iter().position(|i| i.name == top) {
        return Ok(interfaces.swap_remove(at));
    }
    // Uma entidade VHDL, sem diferenciar maiúsculas: o Quartus lê o VHDL e
    // o topo da placa (Verilog) a instancia pelo nome.
    let vhdl = vhdl_interfaces(project)?;
    if let Some((_, interface)) = vhdl
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(&top))
    {
        return match interface {
            Ok(interface) => Ok(interface.clone()),
            Err((file, port)) => Err(AuroraError::InvalidProject {
                path: file.clone(),
                reason: format!(
                    "The port {port} of entity {top} has a type that does not become board pins; use std_logic or std_logic_vector with numeric bounds or generics"
                ),
            }),
        };
    }
    Err(AuroraError::ModuleNotFound {
        name: top,
        available: interfaces
            .into_iter()
            .map(|i| i.name)
            .chain(vhdl.into_iter().map(|(name, _)| name))
            .collect(),
    })
}
