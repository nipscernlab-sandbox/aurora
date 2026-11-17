//! Compilar o projeto para a placa do `fpga.json` com o Quartus Prime do
//! sistema, sem abrir a interface dele.
//!
//! ```text
//! <raiz>/.aurora8/fpga/<placa>/
//!   aurora_board_top.v      o topo da placa (wrapper)
//!   aurora_board_top.qsf    a FPGA, os fontes e os pinos (qsf)
//!   aurora_board_top.sdc    os clocks da placa
//!   output_files/         o .sof, o .rbf, o .svf e os relatórios do Quartus
//!   db/, incremental_db/  o banco de dados do Quartus
//!
//! quartus_map --read_settings_files=on --write_settings_files=off aurora_board_top -c aurora_board_top
//! quartus_fit --read_settings_files=off --write_settings_files=off aurora_board_top -c aurora_board_top
//! quartus_asm --read_settings_files=off --write_settings_files=off aurora_board_top -c aurora_board_top
//! quartus_sta aurora_board_top -c aurora_board_top
//! ```
//!
//! Os quatro são os comandos que a interface do Quartus roda ao compilar (o
//! `Flow Log` do relatório dela), um por passo, na pasta da placa. A AURORA
//! nunca os executou: não havia Quartus na máquina em que foi escrito.

use std::time::Instant;

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::Serialize;

use super::Quartus;
use super::qsf::{self, OUTPUT_DIR, REVISION};
use super::summary::{self, ResourceUsage, TimingSummary};
use super::wrapper::TOP_MODULE;
use super::{manifest, pins};
use crate::control::Control;
use crate::diagnostics::{Diagnostic, Severity};
use crate::error::{AuroraError, Result};
use crate::pipeline::{
    Artifact, ArtifactKind, ArtifactTracker, PlannedStep, Runner, Status, Step, StepReport,
    elapsed_ms, final_status,
};
use crate::process::Termination;
use crate::project::Project;
use crate::toolchain::{Tool, Toolchain};

/// A pasta do projeto do Quartus para uma placa: `<raiz>/.aurora8/fpga/<placa>`.
pub fn build_dir(project: &Project, board: &str) -> Utf8PathBuf {
    project.root().join(".aurora8").join("fpga").join(board)
}

/// O resultado de [`build`].
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct FpgaBuildResult {
    /// A placa (o `id`: `de2-115`).
    pub board: String,
    /// O topo do projeto, que o topo da placa instancia.
    pub top: String,
    /// A pasta do projeto do Quartus ([`build_dir`]).
    #[schemars(with = "String")]
    pub dir: Utf8PathBuf,
    /// O Quartus que compilou.
    pub quartus: Quartus,
    /// Como a compilação terminou. Um design que não alcança o clock
    /// compila do mesmo jeito: quem diz é [`FpgaBuildResult::timing`].
    pub status: Status,
    /// O passo que falhou.
    pub failed_step: Option<Step>,
    /// Até quatro passos, todos do Quartus: `synthesize` (`quartus_map`),
    /// `fit`, `bitstream` (`quartus_asm`) e `timing` (`quartus_sta`).
    pub steps: Vec<StepReport>,
    /// Os erros e avisos do Quartus, sem os `Info` e sem repetição (o aviso
    /// de tempo vem uma vez por canto analisado).
    pub diagnostics: Vec<Diagnostic>,
    /// O topo da placa, o `.qsf` e os arquivos de gravação: o `.sof`,
    /// obrigatório, e o `.rbf` e o `.svf`, para o openFPGALoader.
    pub artifacts: Vec<Artifact>,
    /// O `.sof`, quando a compilação terminou.
    #[schemars(with = "Option<String>")]
    pub bitstream: Option<Utf8PathBuf>,
    /// O que o design usa da FPGA, do resumo do Fitter, quando ele
    /// terminou.
    pub resources: Vec<ResourceUsage>,
    /// A Fmax e as folgas de cada clock, quando a análise de tempo
    /// terminou.
    pub timing: Option<TimingSummary>,
    /// Os ajustes das ligações do `fpga.json` (larguras completadas,
    /// entradas soltas), como em `aurora fpga check`.
    pub notes: Vec<String>,
    /// Quanto a compilação levou, do começo ao fim, em milissegundos.
    pub duration_ms: u64,
}

impl FpgaBuildResult {
    /// `status == Succeeded`.
    pub fn succeeded(&self) -> bool {
        self.status == Status::Succeeded
    }
}

/// Compila o projeto para a placa do `fpga.json`: confere as ligações
/// ([`super::prepare`]), grava o topo da placa, o `.qsf` e o `.sdc` na pasta
/// da placa ([`build_dir`]) e roda o Quartus, um programa por passo. Os
/// processadores precisam estar compilados ([`crate::build`]). O Verilog
/// deles cita as memórias pelo nome, relativo ao diretório de quem o
/// compila: as memórias e a tabela do PC vão para a pasta da placa, a do
/// projeto do Quartus, antes da compilação. Que o Quartus as procura ali é
/// suposição, ainda não conferida numa compilação.
///
/// Os fontes são os da síntese ([`crate::synthesize`]): a biblioteca SAPHO,
/// se o projeto tem processadores, e os do projeto, mais o topo da placa.
///
/// # Erros
///
/// - [`AuroraError::QuartusMissing`] sem Quartus ([`Toolchain::with_quartus`]),
///   e [`AuroraError::ToolchainIncomplete`] se falta um programa dele;
/// - os de [`super::prepare`]: sem `fpga.json`, placa desconhecida,
///   ligações inválidas, topo inexistente;
/// - [`AuroraError::InvalidName`] se um caminho não cabe no `.qsf`;
/// - [`AuroraError::OperationInProgress`]: outra compilação para a mesma
///   placa, ou o build de um processador do projeto, está rodando.
pub fn build(
    toolchain: &Toolchain,
    project: &Project,
    control: &Control,
) -> Result<FpgaBuildResult> {
    let started = Instant::now();
    let quartus = toolchain
        .quartus()
        .ok_or(AuroraError::QuartusMissing)?
        .clone();
    let prepared = super::prepare(Some(toolchain), project)?;
    let resolved = &prepared.resolved;
    let _span = tracing::info_span!("fpga_build", board = %resolved.board.id, top = %resolved.top)
        .entered();
    let sources = crate::synth::board_sources(toolchain, project)?;

    // Os quatro programas antes de rodar o primeiro: faltar o último depois
    // de um minuto de síntese seria pior.
    let dir = build_dir(project, &resolved.board.id);
    let stages = [
        (Step::Synthesize, "quartus_map", Some(true)),
        (Step::Fit, "quartus_fit", Some(false)),
        (Step::Bitstream, "quartus_asm", Some(false)),
        (Step::Timing, "quartus_sta", None),
    ];
    let mut planned = Vec::new();
    for (step, program, read_settings) in stages {
        let mut invocation = quartus.invocation(program, &dir)?;
        if let Some(read) = read_settings {
            let read = if read { "on" } else { "off" };
            invocation = invocation
                .arg(format!("--read_settings_files={read}"))
                .arg("--write_settings_files=off");
        }
        let invocation = invocation.arg(REVISION).arg("-c").arg(REVISION);
        planned.push(PlannedStep::new(step, Tool::Quartus, invocation));
    }

    // A pasta da placa só para esta compilação, e os processadores sem build
    // no meio da cópia das memórias.
    let _locks = crate::simulate::staging_locks(project, &dir.join(".build"), &resolved.board.id)?;
    std::fs::create_dir_all(&dir).map_err(AuroraError::io("Creating directory", &dir))?;
    let top_file = dir.join(format!("{TOP_MODULE}.v"));
    let settings = dir.join(format!("{REVISION}.qsf"));
    let clocks = dir.join(format!("{REVISION}.sdc"));
    let output = dir.join(OUTPUT_DIR);
    let output_file = |extension: &str| output.join(format!("{REVISION}.{extension}"));
    let sof = bitstream_path(&dir);
    let mut tracker = ArtifactTracker::new();
    tracker.expect(ArtifactKind::BoardTop, &top_file, false);
    tracker.expect(ArtifactKind::QuartusProject, &settings, false);
    tracker.expect(ArtifactKind::SramObject, &sof, true);
    tracker.expect(ArtifactKind::RawBinary, output_file("rbf"), false);
    tracker.expect(ArtifactKind::SerialVectorFormat, output_file("svf"), false);
    write(&top_file, &prepared.board_top)?;
    write(
        &settings,
        &qsf::settings(resolved, &sources, project.root())?,
    )?;
    write(&clocks, &qsf::constraints(resolved))?;
    crate::simulate::stage_processor_files(project, &dir)?;
    // A compilação anterior deixa de valer para a gravação já agora, e as
    // entradas são lidas antes do Quartus: o registro descreve o que ele leu.
    manifest::remove(&dir);
    let inputs = manifest::inputs(toolchain, project, &resolved.board)?;

    let mut runner = Runner::new(control);
    for step in planned {
        let which = step.step;
        if !runner.run(step)? {
            break;
        }
        // Os pinos como o Fitter os deixou, antes de gerar o arquivo de
        // gravação: um pino fora do lugar não chega à placa.
        if which == Step::Fit {
            let report = read_lossy(&output_file("pin")).unwrap_or_default();
            let problems = pins::check(&report, resolved);
            if !problems.is_empty() {
                runner.status = Status::Failed;
                runner.failed_step = Some(Step::Fit);
                runner
                    .diagnostics
                    .extend(problems.into_iter().map(|message| Diagnostic {
                        tool: Tool::Quartus,
                        severity: Severity::Error,
                        message,
                        file: Some(output_file("pin")),
                        line: None,
                        column: None,
                        raw: String::new(),
                    }));
                break;
            }
        }
    }

    let artifacts = tracker.finish();
    let status = final_status(runner.status, &artifacts);
    if status == Status::Succeeded {
        manifest::write(
            &dir,
            &manifest::Manifest {
                board: resolved.board.id.clone(),
                device: resolved.board.device.part.clone(),
                top: resolved.top.clone(),
                bitstream: manifest::hash_file(&sof)?,
                inputs,
            },
        )?;
    }
    let finished = |step: Step| {
        runner
            .steps
            .iter()
            .any(|s| s.step == step && s.termination == Termination::Exited(0))
    };
    let resources = if finished(Step::Fit) {
        read_lossy(&output_file("fit.summary"))
            .map(|text| summary::resources(&text))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let timing = if finished(Step::Timing) {
        let targets: Vec<(&str, f64)> = resolved
            .clocks
            .iter()
            .map(|c| (c.signal.as_str(), c.mhz))
            .collect();
        read_lossy(&output_file("sta.rpt")).and_then(|text| summary::timing(&text, &targets))
    } else {
        None
    };
    // O mesmo aviso vem uma vez por canto analisado ("Timing requirements
    // not met", três vezes no Cyclone IV): fica o primeiro.
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    for d in runner.diagnostics {
        let repeated = diagnostics.iter().any(|seen| {
            seen.severity == d.severity
                && seen.message == d.message
                && seen.file == d.file
                && seen.line == d.line
        });
        if !repeated {
            diagnostics.push(d);
        }
    }
    Ok(FpgaBuildResult {
        board: resolved.board.id.clone(),
        top: resolved.top.clone(),
        dir,
        quartus,
        status,
        failed_step: runner.failed_step,
        steps: runner.steps,
        diagnostics,
        artifacts,
        bitstream: (status == Status::Succeeded).then_some(sof),
        resources,
        timing,
        notes: resolved.notes.clone(),
        duration_ms: elapsed_ms(started),
    })
}

fn write(path: &Utf8Path, text: &str) -> Result<()> {
    std::fs::write(path, text).map_err(AuroraError::io("Writing", path))
}

/// Um relatório do Quartus como texto; um caminho na página de código do
/// Windows sai certo ([`crate::process::decode_text`]), e um byte fora dela
/// não perde o resto.
fn read_lossy(path: &Utf8Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|bytes| crate::process::decode_text(&bytes))
}

/// O `.sof` da compilação na pasta da placa `dir` ([`build_dir`]):
/// `output_files/aurora_board_top.sof`.
pub fn bitstream_path(dir: &Utf8Path) -> Utf8PathBuf {
    dir.join(OUTPUT_DIR).join(format!("{REVISION}.sof"))
}
