//! A placa virtual: o projeto simulado com os periféricos da placa, como se
//! estivesse gravado nela.
//!
//! A placa e as ligações são as do fluxo da placa FPGA ([`crate::fpga`]): o
//! `fpga.json` diz a placa e liga as portas do topo aos sinais dela, e o
//! topo da placa gerado ([`crate::fpga::wrapper`]) é o mesmo que vai para o
//! Quartus. Num projeto sem `fpga.json`, as portas com o nome de um sinal da
//! placa ligam a ele ([`FpgaConfig::by_name`]).
//!
//! [`build`] verila esse topo com um laço de simulação em C++ que simula os
//! periféricos nos pinos ([`crate::fpga::Peripheral`]): chaves, botões,
//! LEDs, displays de 7 segmentos, teclado PS/2, tela VGA e UART. [`run`]
//! roda o executável e conversa com ele: as entradas ([`BoardInput`]) vão
//! enquanto ele roda, e o estado dos LEDs, os quadros de vídeo e os bytes da
//! UART voltam como [`SessionEvent`].

mod harness;
mod keys;
mod session;

pub use session::{EndReason, RunOptions, SessionOutcome, run};

/// Os caracteres de `text` que o teclado PS/2 da placa virtual não digita
/// (o texto vira as teclas de um teclado americano: `ç` e `€` ficam de
/// fora), sem repetir, na ordem.
pub fn untypeable(text: &str) -> Vec<char> {
    keys::untypeable(text)
}

use std::time::Instant;

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::control::Control;
use crate::diagnostics::Diagnostic;
use crate::error::{AuroraError, Result};
use crate::fpga::{FpgaConfig, Peripheral, Prepared};
use crate::pipeline::{PlannedStep, Runner, Status, Step, StepReport};
use crate::project::Project;
use crate::toolchain::{Tool, Toolchain};

/// A pasta da placa virtual, dentro da pasta temporária do projeto.
const WORK_DIR: &str = "vboard";
/// O executável gerado (sem a extensão do sistema).
const EXECUTABLE: &str = "aurora-vboard";
/// A pasta do modelo, dentro de [`WORK_DIR`].
const OBJ_DIR: &str = "obj_dir";
/// O runtime em C++ dos periféricos, gravado ao lado do `main` gerado.
const RUNTIME: &str = include_str!("runtime/aurora_board.hpp");

/// A pasta onde a placa virtual do projeto é compilada e roda.
pub fn work_dir(project: &Project) -> Utf8PathBuf {
    project.temp_dir().join(WORK_DIR)
}

/// O executável da placa virtual do projeto (existindo ou não).
pub fn executable_path(project: &Project) -> Utf8PathBuf {
    work_dir(project)
        .join(OBJ_DIR)
        .join(format!("{EXECUTABLE}{}", std::env::consts::EXE_SUFFIX))
}

// ------------------------------------------------------------ o que a interface vê

/// A placa virtual de um projeto: a placa, o topo e cada periférico, ligado
/// ao projeto ou não, com as larguras. É o que uma interface desenha.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct BoardLayout {
    /// A placa (`de2-115`).
    pub board: String,
    /// O nome da placa (`Terasic DE2-115`).
    pub board_name: String,
    /// O módulo do projeto que vai na placa.
    pub top: String,
    /// Os periféricos, na ordem do JSON da placa.
    pub peripherals: Vec<LayoutPeripheral>,
    /// Os relógios da placa que chegam ao projeto.
    pub clocks: Vec<LayoutClock>,
    /// Os ajustes das ligações (larguras completadas ou cortadas, entradas
    /// soltas), como no fluxo da placa.
    pub notes: Vec<String>,
}

/// Um periférico na placa virtual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct LayoutPeripheral {
    /// O identificador nas entradas e no estado: o nome do primeiro sinal do
    /// periférico (`SW`, `HEX7`, `VGA_R`).
    pub id: String,
    /// O tipo e os sinais, como no JSON da placa.
    pub peripheral: Peripheral,
    /// Algum sinal do periférico chega ao projeto. Um periférico solto
    /// aparece apagado: o projeto não o usa.
    pub connected: bool,
    /// Os sinais, com largura e polaridade.
    pub signals: Vec<LayoutSignal>,
}

/// Um sinal de um periférico.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct LayoutSignal {
    /// O nome (`LEDR`).
    pub name: String,
    /// A largura em bits.
    pub width: u32,
    /// Ativo em nível baixo na placa (os segmentos e os botões da Terasic):
    /// o estado da placa virtual já vem convertido para aceso e apertado.
    pub active_low: bool,
    /// O sinal chega ao projeto.
    pub connected: bool,
    /// A porta do topo em cada bit, do bit 0 em diante (`b[3]`, `!rst`,
    /// `a`); `None` no bit solto. Numa entrada da placa lida por mais de
    /// uma porta, as portas separadas por vírgula.
    pub ports: Vec<Option<String>>,
}

/// Um relógio da placa que chega ao projeto.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct LayoutClock {
    /// O sinal (`CLOCK_50`).
    pub signal: String,
    /// A frequência, em MHz.
    pub mhz: f64,
}

/// Uma entrada para a placa virtual em execução.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum BoardInput {
    /// O valor de chaves (bit 1 = chave para cima) ou dos botões apertados
    /// (bit 1 = apertado; a polaridade da placa é aplicada no pino).
    Set {
        /// O periférico (`SW`, `KEY`).
        device: String,
        /// Os bits, do bit 0 em diante.
        value: u64,
    },
    /// Uma tecla do teclado do computador, apertada ou solta, para o teclado
    /// PS/2. O código é o `KeyboardEvent.code` dos navegadores (`KeyA`,
    /// `Enter`, `ArrowUp`, `IntlRo`).
    Key {
        /// O periférico PS/2.
        device: String,
        /// O código da tecla.
        code: String,
        /// Apertada (`true`) ou solta (`false`).
        pressed: bool,
    },
    /// Texto: no teclado PS/2, cada caractere vira as teclas que o digitam
    /// num teclado americano (letras, números, espaço, Enter e a pontuação
    /// comum); na UART, os bytes em UTF-8.
    Text {
        /// O teclado PS/2 ou a UART.
        device: String,
        /// O texto.
        text: String,
    },
    /// Bytes crus: códigos de tecla do PS/2 ou bytes para o RX da UART.
    Bytes {
        /// O teclado PS/2 ou a UART.
        device: String,
        /// Os bytes.
        bytes: Vec<u8>,
    },
    /// Para a simulação, com o estado atual na tela.
    Pause,
    /// Continua depois de [`BoardInput::Pause`] ou [`BoardInput::Step`].
    Resume,
    /// Roda esse número de flancos de relógio e para.
    Step {
        /// Quantos flancos (dois por ciclo de um relógio).
        edges: u64,
    },
    /// Limita a velocidade: 1 é o tempo real, 0 é o mais rápido possível.
    Speed {
        /// Segundos simulados por segundo de relógio; 0 sem limite.
        limit: f64,
    },
}

/// O estado da placa virtual, cerca de 30 vezes por segundo.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct BoardState {
    /// O tempo simulado, em picossegundos.
    pub time_ps: u64,
    /// Segundos simulados por segundo de relógio desde o estado anterior: 1
    /// é o tempo real.
    pub speed: f64,
    /// A simulação está parada ([`BoardInput::Pause`]).
    pub paused: bool,
    /// Quanto cada LED e cada segmento ficou aceso desde o estado anterior,
    /// de 0 a 255, por sinal, do bit 0 em diante: o PWM aparece como brilho,
    /// e o display multiplexado como persistência.
    pub lights: std::collections::BTreeMap<String, Vec<u8>>,
    /// Os bytes que o projeto transmitiu em cada UART desde o estado
    /// anterior.
    pub uart: std::collections::BTreeMap<String, Vec<u8>>,
    /// A velocidade de cada UART que não está na da placa: o projeto
    /// transmitia em outra, e a placa passou a ler na padrão mais próxima do
    /// bit medido (os primeiros bytes se perdem). Vazio na velocidade da
    /// placa.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub uart_baud: std::collections::BTreeMap<String, u32>,
}

/// Um quadro de vídeo remontado pela placa virtual. Só sai quando a imagem
/// muda.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct Frame {
    /// O periférico de vídeo.
    pub device: String,
    /// A largura visível, em pixels.
    pub width: u32,
    /// A altura visível, em linhas.
    pub height: u32,
    /// A imagem em PNG, em base64.
    #[serde(serialize_with = "base64_bytes")]
    #[schemars(with = "String")]
    pub png: Vec<u8>,
}

fn base64_bytes<S: serde::Serializer>(
    bytes: &[u8],
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    use base64::Engine;
    serializer.serialize_str(&base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// O que a placa virtual conta enquanto roda.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(tag = "event", rename_all = "snake_case")]
#[non_exhaustive]
pub enum SessionEvent {
    /// Pronta: o executável subiu e os periféricos estão ligados.
    Ready,
    /// O estado dos LEDs, dos displays e da UART.
    State(BoardState),
    /// Um quadro de vídeo novo.
    Frame(Frame),
    /// As linhas que o projeto escreveu (`$display`) desde o último lote,
    /// no máximo [`OUTPUT_BATCH`] por lote, um lote por estado: um
    /// `$display` a cada ciclo chega a centenas de milhares de linhas por
    /// segundo, mais do que uma interface consegue mostrar.
    Output {
        /// As linhas, sem o fim de linha, na ordem.
        lines: Vec<String>,
        /// As que ficaram de fora deste lote, depois de `lines`.
        omitted: u64,
    },
}

/// Linhas de saída do projeto por lote ([`SessionEvent::Output`]).
pub const OUTPUT_BATCH: usize = 100;

// ------------------------------------------------------------ compilação

/// O que [`build`] usa além do projeto.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct BuildOptions {
    /// A placa, num projeto sem `fpga.json`: as portas com o nome de um sinal
    /// dela ligam a ele. Com `fpga.json`, valem a placa e as ligações dele.
    pub board: Option<String>,
}

/// O resultado de [`build`]: a placa e o executável, ou os erros do
/// Verilator e do compilador C++.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct BoardBuild {
    /// Como terminou. `failed` é Verilog (ou ligação) que não compila.
    pub status: Status,
    /// O passo que falhou, quando for o caso (`verilate`).
    pub failed_step: Option<Step>,
    /// Os passos rodados, com o comando e a saída.
    pub steps: Vec<StepReport>,
    /// As mensagens do YANC, do Verilator e do compilador.
    pub diagnostics: Vec<Diagnostic>,
    /// Os processadores SAPHO compilados antes da placa, os desatualizados
    /// ([`crate::Processor::is_out_of_date`]); vazio quando todos estavam em
    /// dia.
    pub processors: Vec<crate::BuildResult>,
    /// A placa virtual do projeto; `None` quando um processador não compilou,
    /// antes de o topo poder ser lido.
    pub layout: Option<BoardLayout>,
    /// O executável, se a compilação deu certo.
    #[schemars(with = "Option<String>")]
    pub executable: Option<Utf8PathBuf>,
    /// Quanto levou, em milissegundos.
    pub duration_ms: u64,
}

/// A placa virtual compilada e rodada: a compilação e, se ela deu certo,
/// como a execução terminou.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct BoardRun {
    /// A compilação.
    pub build: BoardBuild,
    /// Como a execução terminou; `None` se a compilação falhou.
    pub outcome: Option<SessionOutcome>,
}

impl BoardRun {
    /// A compilação e o fim da execução.
    pub fn new(build: BoardBuild, outcome: Option<SessionOutcome>) -> BoardRun {
        BoardRun { build, outcome }
    }
}

impl BoardBuild {
    /// `status == Succeeded`.
    pub fn succeeded(&self) -> bool {
        self.status == Status::Succeeded
    }
}

/// A placa virtual do projeto sem compilar: a placa, as ligações e os
/// periféricos. Serve para a interface desenhar a placa antes do build.
///
/// # Erros
///
/// Os de [`crate::fpga::prepare`]: [`AuroraError::NoFpgaConfig`] num
/// projeto sem `fpga.json` e sem `options.board`, ligações inválidas, topo
/// ausente.
pub fn layout(
    toolchain: Option<&Toolchain>,
    project: &Project,
    options: &BuildOptions,
) -> Result<BoardLayout> {
    let prepared = prepare(toolchain, project, options)?;
    Ok(layout_of(&prepared))
}

fn prepare(
    toolchain: Option<&Toolchain>,
    project: &Project,
    options: &BuildOptions,
) -> Result<Prepared> {
    match FpgaConfig::read(project.root()) {
        Ok(config) => crate::fpga::prepare_with(toolchain, project, config),
        Err(AuroraError::NoFpgaConfig(path)) => {
            let Some(board) = &options.board else {
                return Err(AuroraError::NoFpgaConfig(path));
            };
            let board = crate::fpga::board(board)?;
            let top = crate::fpga::top_interface(toolchain, project, None)?;
            let config = FpgaConfig::by_name(&board, &top);
            crate::fpga::prepare_with(toolchain, project, config)
        }
        Err(other) => Err(other),
    }
}

fn layout_of(prepared: &Prepared) -> BoardLayout {
    let resolved = &prepared.resolved;
    let board = &resolved.board;
    // O nome de um bit de uma porta: `a` numa porta de 1 bit, `b[3]` nas
    // outras, com `!` quando a ligação inverte.
    let width_of = |port: &str| {
        resolved
            .ports
            .iter()
            .find(|p| p.name == port)
            .map_or(1, |p| p.width)
    };
    let bit_name = |port: &str, bit: u32, invert: bool| {
        let name = if width_of(port) == 1 {
            port.to_owned()
        } else {
            format!("{port}[{bit}]")
        };
        if invert { format!("!{name}") } else { name }
    };
    let mut labels: std::collections::BTreeMap<(String, u32), Vec<String>> =
        std::collections::BTreeMap::new();
    for drive in &resolved.inputs {
        for (bit, source) in drive.bits.iter().enumerate() {
            if let crate::fpga::config::BitSource::Pin {
                signal,
                bit: pin,
                invert,
            } = source
            {
                labels
                    .entry((signal.clone(), *pin))
                    .or_default()
                    .push(bit_name(&drive.port, bit as u32, *invert));
            }
        }
    }
    for drive in &resolved.board_outputs {
        for (bit, port) in drive.bits.iter().enumerate() {
            if let Some(port) = port {
                labels
                    .entry((drive.signal.clone(), bit as u32))
                    .or_default()
                    .push(bit_name(&port.port, port.bit, port.invert));
            }
        }
    }
    let ports_of = |signal: &str, width: u32| -> Vec<Option<String>> {
        (0..width)
            .map(|bit| {
                labels
                    .get(&(signal.to_owned(), bit))
                    .map(|names| names.join(", "))
            })
            .collect()
    };
    let used = |name: &str| {
        resolved.board_inputs.iter().any(|s| s == name)
            || resolved.board_outputs.iter().any(|d| d.signal == name)
    };
    let peripherals = board
        .peripherals
        .iter()
        .map(|peripheral| {
            let signals: Vec<LayoutSignal> = peripheral
                .signals()
                .into_iter()
                .filter_map(|(name, _, _)| board.signal(name))
                .map(|signal| LayoutSignal {
                    name: signal.name.clone(),
                    width: signal.width(),
                    active_low: signal.active_low,
                    connected: used(&signal.name),
                    ports: ports_of(&signal.name, signal.width()),
                })
                .collect();
            LayoutPeripheral {
                id: signals.first().map(|s| s.name.clone()).unwrap_or_default(),
                peripheral: peripheral.clone(),
                connected: signals.iter().any(|s| s.connected),
                signals,
            }
        })
        .collect();
    BoardLayout {
        board: board.id.clone(),
        board_name: board.name.clone(),
        top: resolved.top.clone(),
        peripherals,
        clocks: resolved
            .clocks
            .iter()
            .map(|c| LayoutClock {
                signal: c.signal.clone(),
                mhz: c.mhz,
            })
            .collect(),
        notes: resolved.notes.clone(),
    }
}

/// Compila a placa virtual do projeto: o topo da placa (o mesmo do fluxo da
/// placa FPGA), os fontes do projeto e o laço de simulação dos periféricos,
/// pelo Verilator do bundle, em `.aurora8/Temp/vboard`.
///
/// Um processador SAPHO entra compilado (o Verilog e as memórias que o build
/// dele gerou), como na simulação do projeto. O que estiver desatualizado
/// ([`crate::Processor::is_out_of_date`]) é compilado antes, para a placa
/// rodar o programa que está no fonte, e não o do último build; os em dia
/// ficam como estão, o que deixa o modelo do Verilator sem recompilar.
///
/// # Erros
///
/// Os de [`layout`], [`AuroraError::SystemCompilerMissing`] e os de
/// ferramenta e I/O. Verilog (ou C±) que não compila é `Ok` com
/// `status: failed` e os diagnósticos.
pub fn build(
    toolchain: &Toolchain,
    project: &Project,
    options: &BuildOptions,
    control: &Control,
) -> Result<BoardBuild> {
    let started = Instant::now();
    let elapsed = || u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let stale = project
        .buildable_processors()
        .into_iter()
        .filter(|p| p.needs_conversion() || p.is_out_of_date());
    let processors = crate::build_processors(
        toolchain,
        stale,
        &crate::BuildOptions::default(),
        crate::OnFailure::Stop,
        control,
        |_| {},
    )?;
    if let Some(failed) = processors.iter().find(|b| !b.succeeded()) {
        return Ok(BoardBuild {
            status: failed.status,
            failed_step: failed.failed_step,
            steps: failed.steps.clone(),
            diagnostics: failed.diagnostics.clone(),
            layout: None,
            executable: None,
            duration_ms: elapsed(),
            processors,
        });
    }
    let prepared = prepare(Some(toolchain), project, options)?;
    let layout = layout_of(&prepared);
    let work = work_dir(project);
    let _span = tracing::info_span!("virtual_board_build", top = %layout.top).entered();
    // Copia as memórias dos processadores para a raiz: não corre com o build
    // de um processador.
    let _locks = crate::simulate::staging_locks(project, &work.join(".vboard"), project.name())?;
    std::fs::create_dir_all(&work).map_err(AuroraError::io("Creating directory", &work))?;
    write(
        &work.join(format!("{}.v", crate::fpga::wrapper::TOP_MODULE)),
        &prepared.board_top,
    )?;
    write(
        &work.join("board_main.cpp"),
        &harness::generate(&prepared.resolved),
    )?;
    write(&work.join("aurora_board.hpp"), RUNTIME)?;
    // A placa roda na raiz do projeto, como a simulação e o Quartus: um
    // `$readmemb("program.txt")` do projeto acha o arquivo, e o `.v` de cada
    // processador, as memórias que o build dele gerou, copiadas para lá.
    crate::simulate::stage_processor_files(project, project.root())?;

    let mut files = vec![work.join(format!("{}.v", crate::fpga::wrapper::TOP_MODULE))];
    files.extend(crate::synth::project_sources(project)?);
    let hdl = toolchain.sapho_library(!project.processors().is_empty())?;

    let mut runner = Runner::new(control);
    // O Verilator só lê Verilog: o VHDL do design entra pelo Verilog que o
    // `ghdl --synth` gera, com cada entidade num módulo de nome em
    // minúsculas, o nome com que o topo da placa a instancia.
    let vhdl = crate::ghdl::design_files(project)?;
    if !vhdl.is_empty() {
        files.extend(crate::ghdl::to_verilog(
            toolchain,
            &vhdl,
            &work.join("vhdl"),
            &mut runner,
        )?);
    }
    let invocation = crate::simulate::board_verilator(toolchain, &work, OBJ_DIR, EXECUTABLE)?;
    let mut invocation = invocation
        .arg("--top-module")
        .arg(crate::fpga::wrapper::TOP_MODULE)
        .arg("-Mdir")
        .arg(OBJ_DIR);
    if let Some(hdl) = &hdl {
        invocation = invocation.arg("-y").tool_path_arg(hdl);
    }
    invocation = crate::simulate::verilator_include_paths(invocation, None, project.root());
    invocation = invocation.arg("board_main.cpp");
    for file in &files {
        invocation = invocation.tool_path_arg(file);
    }
    let obj_dir = work.join(OBJ_DIR);
    crate::simulate::model_build_started(&obj_dir, &crate::simulate::model_stamp(toolchain))?;
    set_aside_running_executable(&obj_dir);
    let built = runner.run(PlannedStep::new(
        Step::Verilate,
        Tool::Verilator,
        invocation,
    ))?;
    crate::simulate::model_build_finished(&obj_dir, &runner);
    crate::simulate::quiet_runtime_warnings(&mut runner.diagnostics, toolchain.root());
    let executable = executable_path(project);
    let executable = (built && executable.is_file()).then_some(executable);
    let status = if built && executable.is_none() {
        Status::Incomplete
    } else {
        runner.status
    };
    let mut steps: Vec<StepReport> = processors.iter().flat_map(|b| b.steps.clone()).collect();
    steps.extend(runner.steps);
    Ok(BoardBuild {
        status,
        failed_step: runner.failed_step,
        steps,
        diagnostics: runner.diagnostics,
        layout: Some(layout),
        executable,
        duration_ms: elapsed(),
        processors,
    })
}

/// O executável de uma placa do mesmo projeto que ainda roda (ligada na IDE
/// e pela CLI, ou em duas janelas) não pode ser regravado no Windows: o link
/// falhava com "cannot open output file aurora-vboard.exe: Permission
/// denied". Um executável em uso pode ser renomeado, e a placa que roda
/// segue com ele: vai para o lado, com um nome que a próxima compilação
/// apaga quando ele não estiver mais em uso. Fora do Windows, o `ld` troca
/// o arquivo sem precisar disso.
fn set_aside_running_executable(obj_dir: &Utf8Path) {
    let aside = format!("{EXECUTABLE}.old-");
    if let Ok(entries) = obj_dir.read_dir_utf8() {
        for entry in entries.flatten() {
            if entry.file_name().starts_with(&aside) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    let executable = obj_dir.join(format!("{EXECUTABLE}{}", std::env::consts::EXE_SUFFIX));
    if !cfg!(windows)
        || !executable.is_file()
        || std::fs::OpenOptions::new()
            .write(true)
            .open(&executable)
            .is_ok()
    {
        return;
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let to = obj_dir.join(format!("{aside}{stamp}{}", std::env::consts::EXE_SUFFIX));
    if let Err(error) = std::fs::rename(&executable, &to) {
        tracing::warn!(%executable, %error, "The running virtual board could not be set aside");
    }
}

fn write(path: &Utf8Path, text: &str) -> Result<()> {
    // Só regrava o que mudou: o `make` do Verilator não refaz o que não
    // mudou de data.
    if std::fs::read_to_string(path).is_ok_and(|old| old == text) {
        return Ok(());
    }
    std::fs::write(path, text).map_err(AuroraError::io("Writing", path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verilog::{ModuleInterface, Port, PortDirection};

    fn port(name: &str, direction: PortDirection, width: u32) -> Port {
        Port {
            name: name.into(),
            direction,
            width,
            signed: false,
        }
    }

    fn layout() -> BoardLayout {
        let board = crate::fpga::board("de2-115").unwrap();
        let top = ModuleInterface {
            name: "placa".into(),
            file: "placa.v".into(),
            ports: vec![
                port("CLOCK_50", PortDirection::Input, 1),
                port("SW", PortDirection::Input, 4),
                port("KEY", PortDirection::Input, 1),
                port("LEDR", PortDirection::Output, 4),
                port("HEX0", PortDirection::Output, 7),
            ],
        };
        let config = FpgaConfig::by_name(&board, &top);
        let resolved = config.resolve(&board, &top).unwrap();
        layout_of(&Prepared {
            config: "fpga.json".into(),
            board_top: crate::fpga::wrapper::generate(&resolved),
            connections: Vec::new(),
            resolved,
        })
    }

    #[test]
    fn the_layout_marks_what_the_project_uses() {
        let layout = layout();
        let connected: Vec<&str> = layout
            .peripherals
            .iter()
            .filter(|p| p.connected)
            .map(|p| p.id.as_str())
            .collect();
        // Os displays são um periférico só, com o id do primeiro dígito, e
        // aparecem ligados porque o HEX0 está.
        assert_eq!(connected, ["SW", "KEY", "LEDR", "HEX7"]);
        let sw = &layout.peripherals[0].signals[0];
        assert_eq!(
            (sw.name.as_str(), sw.width, sw.active_low),
            ("SW", 18, false)
        );
        let key = &layout.peripherals[1].signals[0];
        assert!(key.active_low);
        // A porta em cada bit: o SW de 4 bits do topo nas chaves de baixo, o
        // KEY de 1 bit no botão 0, e as outras chaves soltas.
        assert_eq!(sw.ports[0].as_deref(), Some("SW[0]"));
        assert_eq!(sw.ports[3].as_deref(), Some("SW[3]"));
        assert_eq!(sw.ports[4], None);
        assert_eq!(key.ports[0].as_deref(), Some("KEY"));
        let ledr = &layout.peripherals[2].signals[0];
        assert_eq!(ledr.ports[2].as_deref(), Some("LEDR[2]"));
    }

    #[test]
    fn inputs_go_only_to_peripherals_that_take_them() {
        let layout = layout();
        let set = BoardInput::Set {
            device: "SW".into(),
            value: 0x2A,
        };
        assert_eq!(layout.command(&set).as_deref(), Some("set SW 2a"));
        // As 18 chaves da DE2-115: o bit 18 não cabe.
        let wide = BoardInput::Set {
            device: "SW".into(),
            value: 1 << 18,
        };
        let error = layout.check(&wide).unwrap_err();
        assert_eq!(error.code(), "virtual_board_value");
        assert!(error.to_string().contains("0x3ffff"), "{error}");
        assert_eq!(layout.command(&wide), None);
        // Um LED não recebe valor; a placa não tem teclado ligado.
        let led = BoardInput::Set {
            device: "LEDR".into(),
            value: 1,
        };
        let error = layout.check(&led).unwrap_err();
        assert_eq!(error.code(), "virtual_board_device");
        assert!(error.to_string().contains("SW, KEY"), "{error}");
        assert_eq!(layout.command(&led), None);
        let key = BoardInput::Key {
            device: "PS2_CLK".into(),
            code: "KeyA".into(),
            pressed: true,
        };
        assert!(layout.check(&key).is_err());
        assert_eq!(layout.command(&BoardInput::Pause).as_deref(), Some("pause"));
        assert_eq!(
            layout.command(&BoardInput::Step { edges: 0 }).as_deref(),
            Some("step 1")
        );
    }
}
