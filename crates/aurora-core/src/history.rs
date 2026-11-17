//! O relatório de cada operação e o histórico deles, no projeto.
//!
//! Depois de uma operação (compilar, verificar, simular, sintetizar), a
//! interface chama [`record`] com o que cada fase devolveu ([`Operation`]).
//! A AURORA grava o relatório em texto, para quem lê, e o que a comparação usa,
//! em JSON, numa pasta nova do histórico:
//!
//! ```text
//! <projeto>/.aurora8/reports/
//!   run-000001/
//!     report.txt     o relatório, como `aurora report show` o mostra
//!     record.json    metadados, estatísticas da síntese e tempos da simulação
//!   run-000002/
//!   sequence         o último número reservado
//! ```
//!
//! O número só cresce: `sequence` guarda o último reservado, e o número de um
//! relatório apagado ([`plan_cleanup`] e [`remove`]) não volta. A pasta é montada com outro nome e renomeada
//! no fim, então quem lê o histórico nunca vê um relatório pela metade. Duas
//! operações gravando ao mesmo tempo no mesmo projeto não colidem: a segunda
//! fica com o número seguinte.
//!
//! [`compare`] compara dois relatórios: as estatísticas genéricas de síntese
//! ([`SynthesisStatistics`]) e os tempos da simulação, cada parte só quando o
//! contexto permite (o mesmo topo; o mesmo simulador e o mesmo testbench). O
//! resto que difere (fontes, entradas, versões, máquina) vira aviso, sem
//! invalidar a comparação. Uma mudança é só descrita: tempo maior não é
//! chamado de regressão, nem menor de melhora.
//!
//! É a função de relatório do Alpha-Solar (`solar report`), com os nomes e os
//! formatos da AURORA: JSON no lugar dos `.dat`; `run-` no lugar de `build-`,
//! porque na AURORA "build" é compilar processador; milissegundos, que é o que
//! os passos medem; e o tempo simulado em femtossegundos, que cabe as
//! escalas do Icarus.

use std::fmt::Write as _;
use std::time::SystemTime;

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::build::BuildResult;
use crate::diagnostics::{Diagnostic, Severity};
use crate::error::{AuroraError, Result};
use crate::files::FileRole;
use crate::pipeline::{Artifact, Status, Step, StepReport};
use crate::process::Termination;
use crate::project::{Language, Project, utc_seconds};
use crate::simulate::{SimulationResult, Simulator};
use crate::stats::{SynthesisMetric, SynthesisStatistics};
use crate::synth::{CheckResult, SchematicResult, SynthesisResult};
use crate::toolchain::{Tool, Toolchain};

/// A pasta do histórico, relativa à raiz do projeto.
pub const REPORTS_DIR: &str = ".aurora8/reports";

/// O formato do `record.json` que esta AURORA grava e entende.
pub const RECORD_SCHEMA: u32 = 1;

const RECORD_FILE: &str = "record.json";
const TEXT_FILE: &str = "report.txt";
const SEQUENCE_FILE: &str = "sequence";
const ID_PREFIX: &str = "run-";
/// O nome de uma pasta que [`remove`] está apagando.
const REMOVING_PREFIX: &str = ".removing-";
/// Linhas do fim da saída do passo que falhou, no relatório.
const FAILURE_TAIL: usize = 20;
const RULE: &str =
    "================================================================================";

/// Uma operação que terminou, para o relatório: o que cada fase devolveu. As
/// fases que não rodaram ficam vazias.
///
/// ```no_run
/// use aurora_core::{history, Control, Project, SimulationOptions, Simulator, Toolchain};
///
/// let toolchain = Toolchain::open("/opt/aurora/toolchain")?;
/// let project = Project::open("/home/eu/projetos/soma")?;
/// let started = std::time::SystemTime::now();
/// let options = SimulationOptions::new(Simulator::Icarus);
/// let sim = aurora_core::simulate_project(&toolchain, &project, &options, &Control::default())?;
/// let operation = history::Operation::new("aurora sim", started).with_simulation(&sim);
/// let record = history::record(&project, &toolchain, &operation)?;
/// println!("relatório {}", record.id);
/// # Ok::<(), aurora_core::AuroraError>(())
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Operation<'a> {
    /// O comando, como o usuário o escreveu (`aurora sim --verilator`).
    pub command: String,
    /// Quando a operação começou.
    pub started: SystemTime,
    /// Os processadores compilados antes, na ordem.
    pub builds: &'a [BuildResult],
    /// A verificação.
    pub check: Option<&'a CheckResult>,
    /// A simulação.
    pub simulation: Option<&'a SimulationResult>,
    /// A síntese.
    pub synthesis: Option<&'a SynthesisResult>,
    /// O esquemático.
    pub schematic: Option<&'a SchematicResult>,
}

impl<'a> Operation<'a> {
    /// Uma operação sem fases.
    pub fn new(command: impl Into<String>, started: SystemTime) -> Self {
        Operation {
            command: command.into(),
            started,
            builds: &[],
            check: None,
            simulation: None,
            synthesis: None,
            schematic: None,
        }
    }

    /// Com os processadores compilados antes.
    pub fn with_builds(mut self, builds: &'a [BuildResult]) -> Self {
        self.builds = builds;
        self
    }

    /// Com a verificação.
    pub fn with_check(mut self, check: &'a CheckResult) -> Self {
        self.check = Some(check);
        self
    }

    /// Com a simulação.
    pub fn with_simulation(mut self, simulation: &'a SimulationResult) -> Self {
        self.simulation = Some(simulation);
        self
    }

    /// Com a síntese.
    pub fn with_synthesis(mut self, synthesis: &'a SynthesisResult) -> Self {
        self.synthesis = Some(synthesis);
        self
    }

    /// Com o esquemático.
    pub fn with_schematic(mut self, schematic: &'a SchematicResult) -> Self {
        self.schematic = Some(schematic);
        self
    }

    /// Como a operação terminou: o status da primeira fase que não deu
    /// certo, ou `Succeeded`.
    pub fn status(&self) -> Status {
        self.phases()
            .iter()
            .map(|p| p.status)
            .find(|s| *s != Status::Succeeded)
            .unwrap_or(Status::Succeeded)
    }

    fn phases(&self) -> Vec<Phase<'a>> {
        let mut phases: Vec<Phase<'a>> = self
            .builds
            .iter()
            .map(|b| Phase {
                label: format!("Build {}", b.processor),
                status: b.status,
                duration_ms: b.duration_ms,
                steps: &b.steps,
                diagnostics: &b.diagnostics,
                artifacts: &b.artifacts,
                failed_step: b.failed_step,
            })
            .collect();
        if let Some(c) = self.check {
            phases.push(Phase {
                label: "Check".into(),
                status: c.status,
                duration_ms: c.duration_ms,
                steps: &c.steps,
                diagnostics: &c.diagnostics,
                artifacts: &[],
                failed_step: c.failed_step,
            });
        }
        if let Some(s) = self.simulation {
            phases.push(Phase {
                label: format!(
                    "{} {} ({})",
                    if s.fast {
                        "Fast simulation"
                    } else {
                        "Simulation"
                    },
                    s.top,
                    simulator_name(s.simulator)
                ),
                status: s.status,
                duration_ms: s.duration_ms,
                steps: &s.steps,
                diagnostics: &s.diagnostics,
                artifacts: &s.artifacts,
                failed_step: s.failed_step,
            });
        }
        if let Some(s) = self.synthesis {
            phases.push(Phase {
                label: format!("Synthesis {}", s.top),
                status: s.status,
                duration_ms: s.duration_ms,
                steps: &s.steps,
                diagnostics: &s.diagnostics,
                artifacts: &s.artifacts,
                failed_step: s.failed_step,
            });
        }
        if let Some(s) = self.schematic {
            phases.push(Phase {
                label: format!("Schematic {}", s.module),
                status: s.status,
                duration_ms: s.duration_ms,
                steps: &s.steps,
                diagnostics: &s.diagnostics,
                artifacts: &s.artifacts,
                failed_step: s.failed_step,
            });
        }
        phases
    }
}

/// Uma fase da operação, vista do relatório.
struct Phase<'a> {
    label: String,
    status: Status,
    duration_ms: u64,
    steps: &'a [StepReport],
    diagnostics: &'a [Diagnostic],
    artifacts: &'a [Artifact],
    failed_step: Option<Step>,
}

/// Um relatório guardado: o que a comparação e a lista usam. O texto fica ao
/// lado, em `report.txt` ([`report_text`]).
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema, Deserialize)]
#[non_exhaustive]
pub struct RunRecord {
    /// O formato ([`RECORD_SCHEMA`]).
    pub schema: u32,
    /// O identificador (`run-000042`).
    pub id: String,
    /// O contexto da operação.
    pub metadata: RunMetadata,
    /// As estatísticas do Yosys, quando houve síntese e o Yosys as gravou.
    pub synthesis: Option<SynthesisStatistics>,
    /// Os tempos da simulação, quando houve simulação.
    pub simulation: Option<SimulationTimings>,
}

/// O contexto de uma operação guardada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[non_exhaustive]
pub struct RunMetadata {
    /// Quando terminou, em UTC (`2026-10-03T21:04:05Z`).
    pub timestamp: String,
    /// O comando.
    pub command: String,
    /// Como terminou.
    pub status: Status,
    /// Quanto levou, do começo ao fim, em milissegundos.
    pub duration_ms: u64,
    /// A versão que gravou. `lace_version` nos relatórios gravados pelo
    /// Lace, que a `.lace` levada para a `.aurora8` traz.
    #[serde(alias = "lace_version")]
    pub aurora_version: String,
    /// O que identifica o projeto: um hash da raiz dele. Relatórios de
    /// projetos diferentes não se comparam.
    pub project: String,
    /// O nome do projeto.
    pub project_name: String,
    /// O bundle de ferramentas (`2026.09.29`) e a plataforma.
    pub bundle: String,
    /// `linux-x64`, `darwin-arm64`, `windows-x64`.
    pub platform: String,
    /// O módulo de topo: o da síntese, ou o testbench simulado, ou o topo do
    /// projeto.
    pub top: Option<String>,
    /// O contexto da síntese, quando houve.
    pub synthesis: Option<SynthesisContext>,
    /// O contexto da simulação, quando houve.
    pub simulation: Option<SimulationContext>,
    /// A máquina.
    pub environment: Environment,
}

/// O que decide se duas sínteses se comparam, e o que vira aviso.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[non_exhaustive]
pub struct SynthesisContext {
    /// O módulo de topo.
    pub top: String,
    /// A versão do componente yosys do bundle.
    pub component_version: Option<String>,
    /// Hash dos fontes sintetizáveis e dos programas dos processadores.
    pub sources: String,
}

/// O que decide se duas simulações se comparam, e o que vira aviso.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[non_exhaustive]
pub struct SimulationContext {
    /// O simulador.
    pub simulator: Simulator,
    /// O testbench (o módulo de topo simulado).
    pub testbench: String,
    /// A versão do componente do simulador no bundle.
    pub component_version: Option<String>,
    /// Hash dos fontes, com os testbenches, e dos programas dos processadores.
    pub sources: String,
    /// Hash das entradas dos processadores (`<proc>/input_*.txt`, ao lado
    /// do fonte).
    pub inputs: String,
    /// A simulação gravou onda.
    pub waveform: bool,
}

/// A máquina onde a operação rodou. O que o sistema não informa sem rodar
/// outro programa fica `null`: a AURORA só lê arquivos do sistema (no Linux,
/// `/proc` e `/etc/os-release`) e o `uname`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[non_exhaustive]
pub struct Environment {
    /// O nome da máquina.
    pub hostname: Option<String>,
    /// `linux`, `macos`, `windows`.
    pub os: String,
    /// O nome da distribuição (Linux).
    pub os_name: Option<String>,
    /// A versão do kernel (Linux e macOS).
    pub kernel: Option<String>,
    /// `x86_64`, `aarch64`.
    pub arch: String,
    /// O modelo do processador (Linux).
    pub cpu_model: Option<String>,
    /// Processadores lógicos disponíveis.
    pub cpus: Option<u64>,
    /// A memória total, em bytes (Linux).
    pub memory_bytes: Option<u64>,
    /// Hash do sistema, da arquitetura, do processador, do número de
    /// processadores e do nome da máquina: muda quando a máquina muda.
    pub fingerprint: String,
}

/// Os tempos de uma simulação, em milissegundos de relógio. O tempo simulado
/// é outra grandeza: o quanto o tempo andou dentro do modelo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[non_exhaustive]
pub struct SimulationTimings {
    /// A simulação terminou com sucesso.
    pub succeeded: bool,
    /// O passo que compila o Verilog (`iverilog` ou `verilator --binary`).
    pub compile_ms: Option<u64>,
    /// O passo que roda o testbench (`vvp` ou o modelo do Verilator).
    pub execution_ms: Option<u64>,
    /// A simulação inteira, medida à parte: com a preparação da AURORA.
    pub total_ms: Option<u64>,
    /// O tempo simulado no `$finish`, em femtossegundos, quando o simulador
    /// o informa (o `vvp` informa; o Verilator, não).
    pub simulated_fs: Option<u64>,
}

impl SimulationTimings {
    fn availability(&self) -> Availability {
        let present = [self.compile_ms, self.execution_ms, self.total_ms]
            .iter()
            .filter(|t| t.is_some())
            .count();
        match present {
            0 => Availability::Unavailable,
            3 => Availability::Available,
            _ => Availability::Partial,
        }
    }
}

/// Quanto de uma parte um relatório tem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Availability {
    /// Tudo.
    Available,
    /// Uma parte (uma simulação que não compilou não tem tempo de execução).
    Partial,
    /// Nada.
    Unavailable,
}

/// Uma linha de [`list`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct RunSummary {
    /// O identificador.
    pub id: String,
    /// Quando terminou; `null` num relatório ilegível.
    pub timestamp: Option<String>,
    /// O comando.
    pub command: Option<String>,
    /// Como terminou.
    pub status: Option<Status>,
    /// O módulo de topo.
    pub top: Option<String>,
    /// Tem estatísticas de síntese.
    pub synthesis: bool,
    /// Quanto dos tempos de simulação tem.
    pub simulation: Availability,
    /// O `record.json` pôde ser lido.
    pub readable: bool,
}

/// O que aconteceu com um número de um relatório para o outro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Change {
    /// Igual.
    Unchanged,
    /// Maior.
    Increased,
    /// Menor.
    Decreased,
    /// Só existe no atual (um tipo de célula novo).
    Added,
    /// Só existe na referência (um tipo de célula que sumiu).
    Removed,
    /// Falta de um dos lados.
    NotComparable,
}

/// Um número comparado.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct MetricComparison {
    /// Na referência.
    pub baseline: Option<u64>,
    /// No atual.
    pub current: Option<u64>,
    /// O que mudou.
    pub change: Change,
    /// Atual menos referência.
    pub delta: Option<i64>,
    /// A variação em porcentagem da referência. `null` quando não se
    /// compara ou quando a referência é zero e o atual não (não há
    /// porcentagem de zero; o texto mostra `new`).
    pub percent: Option<f64>,
}

/// Uma contagem do resumo, comparada.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct SynthesisMetricComparison {
    /// Qual.
    pub metric: SynthesisMetric,
    /// A comparação.
    pub comparison: MetricComparison,
}

/// Um tipo de célula, comparado.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct CellComparison {
    /// O tipo (`$add`).
    pub cell_type: String,
    /// A comparação. Um tipo que só existe de um lado conta como 0 do outro,
    /// com `added` ou `removed`.
    pub usage: MetricComparison,
}

/// As estatísticas de síntese, comparadas.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct SynthesisComparison {
    /// As contagens do resumo, na ordem do relatório.
    pub metrics: Vec<SynthesisMetricComparison>,
    /// Os tipos de célula, da maior mudança para a menor e, no empate, pelo
    /// nome.
    pub cell_types: Vec<CellComparison>,
    /// Tipos que só existem no atual.
    pub added_cell_types: usize,
    /// Tipos que só existem na referência.
    pub removed_cell_types: usize,
    /// Contagens do resumo que aumentaram.
    pub increased: usize,
    /// Que diminuíram.
    pub decreased: usize,
    /// Iguais.
    pub unchanged: usize,
    /// Sem um dos lados.
    pub not_comparable: usize,
}

impl SynthesisComparison {
    /// A comparação de uma contagem do resumo.
    pub fn metric(&self, metric: SynthesisMetric) -> Option<&MetricComparison> {
        self.metrics
            .iter()
            .find(|m| m.metric == metric)
            .map(|m| &m.comparison)
    }
}

/// Os tempos de simulação, comparados.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct TimingComparison {
    /// Compilação, em milissegundos.
    pub compile: MetricComparison,
    /// Execução, em milissegundos.
    pub execution: MetricComparison,
    /// A simulação inteira, em milissegundos.
    pub total: MetricComparison,
    /// O tempo simulado, em femtossegundos.
    pub simulated: MetricComparison,
    /// As duas rodaram em máquinas diferentes.
    pub environment_changed: bool,
}

/// Dois relatórios comparados: o atual contra a referência.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct RunComparison {
    /// O relatório atual.
    pub current_id: String,
    /// O contexto dele.
    pub current: RunMetadata,
    /// A referência.
    pub baseline_id: String,
    /// O contexto dela.
    pub baseline: RunMetadata,
    /// A síntese comparada; `null` se os dois não têm estatísticas do mesmo
    /// topo.
    pub synthesis: Option<SynthesisComparison>,
    /// A simulação comparada; `null` se os dois não têm tempos do mesmo
    /// simulador e testbench.
    pub simulation: Option<TimingComparison>,
    /// O que difere no contexto e pode explicar uma mudança (fontes,
    /// versões, máquina), em inglês, sem ponto final.
    pub warnings: Vec<String>,
}

// ------------------------------------------------------------------ gravar

/// Grava o relatório de `operation` no histórico do projeto e o devolve.
///
/// # Erros
///
/// [`AuroraError::Io`] se a pasta do histórico não pôde ser escrita.
pub fn record(
    project: &Project,
    toolchain: &Toolchain,
    operation: &Operation,
) -> Result<RunRecord> {
    let finished = SystemTime::now();
    let mut record = RunRecord {
        schema: RECORD_SCHEMA,
        id: String::new(),
        metadata: metadata(project, toolchain, operation, finished),
        synthesis: operation.synthesis.and_then(|s| s.statistics.clone()),
        simulation: operation.simulation.map(timings),
    };
    let dir = reports_dir(project);
    store(&dir, |id| {
        record.id = id.to_owned();
        let text = render(project, toolchain, operation, &record, finished);
        let json = serde_json::to_string_pretty(&record).expect("RunRecord serializa") + "\n";
        (text, json)
    })?;
    Ok(record)
}

fn reports_dir(project: &Project) -> Utf8PathBuf {
    project.root().join(REPORTS_DIR)
}

/// Reserva o próximo número, monta a pasta com outro nome e a renomeia. Se
/// outra operação ficou com o número no meio do caminho, tenta o seguinte.
fn store(dir: &Utf8Path, mut contents: impl FnMut(&str) -> (String, String)) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(AuroraError::io("Creating report history", dir))?;
    let sequence = dir.join(SEQUENCE_FILE);
    for _ in 0..64 {
        let last = read_sequence(&sequence).max(highest_id(dir));
        let next = last + 1;
        let id = format!("{ID_PREFIX}{next:06}");
        write_atomically(&sequence, &format!("{next}\n"))?;

        let staging = dir.join(format!(".pending-{id}-{}", unique_suffix()));
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir(&staging)
            .map_err(AuroraError::io("Creating report directory", &staging))?;
        let (text, json) = contents(&id);
        let written = std::fs::write(staging.join(TEXT_FILE), text)
            .and_then(|()| std::fs::write(staging.join(RECORD_FILE), json));
        if let Err(e) = written {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(AuroraError::io("Writing report", &staging)(e));
        }
        let target = dir.join(&id);
        if target.exists() {
            let _ = std::fs::remove_dir_all(&staging);
            continue;
        }
        match std::fs::rename(&staging, &target) {
            Ok(()) => return Ok(()),
            // Outra operação publicou o mesmo número entre a conferência e
            // a renomeação.
            Err(_) if target.exists() => {
                let _ = std::fs::remove_dir_all(&staging);
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&staging);
                return Err(AuroraError::io("Publishing report", &target)(e));
            }
        }
    }
    Err(AuroraError::io("Publishing report", dir)(
        std::io::Error::other("no free report number after 64 attempts"),
    ))
}

fn read_sequence(path: &Utf8Path) -> u64 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or(0)
}

fn highest_id(dir: &Utf8Path) -> u64 {
    ids_in(dir).into_iter().max().unwrap_or(0)
}

/// Os números dos relatórios publicados em `dir`.
fn ids_in(dir: &Utf8Path) -> Vec<u64> {
    let Ok(entries) = dir.read_dir_utf8() else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| id_number(e.file_name()))
        .collect()
}

fn id_number(id: &str) -> Option<u64> {
    let digits = id.strip_prefix(ID_PREFIX)?;
    (digits.len() >= 6 && digits.bytes().all(|b| b.is_ascii_digit()))
        .then(|| digits.parse().ok())
        .flatten()
}

/// `<pid>-<n>`, diferente em cada chamada: duas operações do mesmo processo
/// (duas threads do Studio) não dividem a pasta de preparo nem o temporário.
fn unique_suffix() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{}-{n}", std::process::id())
}

/// Grava num arquivo ao lado e renomeia: quem lê vê o conteúdo velho ou o
/// novo, nunca pela metade.
fn write_atomically(path: &Utf8Path, text: &str) -> Result<()> {
    let temporary = path.with_file_name(format!(
        "{}.tmp-{}",
        path.file_name().unwrap_or("file"),
        unique_suffix()
    ));
    std::fs::write(&temporary, text).map_err(AuroraError::io("Writing", &temporary))?;
    std::fs::rename(&temporary, path).map_err(|e| {
        let _ = std::fs::remove_file(&temporary);
        AuroraError::io("Writing", path)(e)
    })
}

// ------------------------------------------------------------------- ler

/// O identificador de um relatório a partir do que o usuário escreveu:
/// `run-000042` ou só o número (`42`). `None` se não é nenhum dos dois.
pub fn parse_id(text: &str) -> Option<String> {
    let text = text.trim();
    if let Some(n) = id_number(text) {
        return Some(format!("{ID_PREFIX}{n:06}"));
    }
    if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) {
        return text
            .parse::<u64>()
            .ok()
            .map(|n| format!("{ID_PREFIX}{n:06}"));
    }
    None
}

/// Os relatórios do projeto, do mais novo para o mais antigo. Um relatório
/// cujo `record.json` não se lê aparece com `readable` falso.
pub fn list(project: &Project) -> Result<Vec<RunSummary>> {
    let dir = reports_dir(project);
    let mut ids = ids_in(&dir);
    ids.sort_unstable_by(|a, b| b.cmp(a));
    Ok(ids
        .into_iter()
        .map(|n| {
            let id = format!("{ID_PREFIX}{n:06}");
            match read_record(&dir, &id) {
                Ok(r) => RunSummary {
                    timestamp: Some(r.metadata.timestamp.clone()),
                    command: Some(r.metadata.command.clone()),
                    status: Some(r.metadata.status),
                    top: r.metadata.top.clone(),
                    synthesis: r.synthesis.is_some(),
                    simulation: r
                        .simulation
                        .as_ref()
                        .map_or(Availability::Unavailable, SimulationTimings::availability),
                    readable: true,
                    id,
                },
                Err(_) => RunSummary {
                    id,
                    timestamp: None,
                    command: None,
                    status: None,
                    top: None,
                    synthesis: false,
                    simulation: Availability::Unavailable,
                    readable: false,
                },
            }
        })
        .collect())
}

/// Um relatório guardado, pelo identificador (`run-000042` ou `42`).
///
/// # Erros
///
/// [`AuroraError::ReportNotFound`] se não existe; [`AuroraError::InvalidReport`]
/// se o `record.json` não se lê.
pub fn load(project: &Project, id: &str) -> Result<RunRecord> {
    let id = parse_id(id).ok_or_else(|| AuroraError::ReportNotFound(id.to_owned()))?;
    read_record(&reports_dir(project), &id)
}

/// O relatório mais novo.
///
/// # Erros
///
/// [`AuroraError::NoReports`] se o histórico está vazio.
pub fn latest(project: &Project) -> Result<RunRecord> {
    match highest_id(&reports_dir(project)) {
        0 => Err(AuroraError::NoReports(project.spf_path().to_owned())),
        n => read_record(&reports_dir(project), &format!("{ID_PREFIX}{n:06}")),
    }
}

/// O `report.txt` de um relatório, sem mudança.
pub fn report_text(project: &Project, id: &str) -> Result<String> {
    let path = report_path(project, id)?;
    std::fs::read_to_string(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => AuroraError::ReportNotFound(id.to_owned()),
        _ => AuroraError::io("Reading report", &path)(e),
    })
}

/// Onde fica o `report.txt` de um relatório.
pub fn report_path(project: &Project, id: &str) -> Result<Utf8PathBuf> {
    let id = parse_id(id).ok_or_else(|| AuroraError::ReportNotFound(id.to_owned()))?;
    Ok(reports_dir(project).join(id).join(TEXT_FILE))
}

/// `history.show`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct ReportView {
    /// O identificador (`run-000042`).
    pub id: String,
    /// O `report.txt`.
    #[schemars(with = "String")]
    pub path: Utf8PathBuf,
    /// O que a comparação usa.
    pub record: RunRecord,
    /// O `report.txt`, sem mudança.
    pub text: String,
}

/// O relatório `id`, ou o mais novo sem `id`, com o texto e o caminho do
/// `report.txt`: o que o `aurora report show` e o `history.show` mostram.
///
/// # Erros
///
/// [`AuroraError::NoReports`] sem relatório nenhum; os de [`load`] e de
/// [`report_text`].
pub fn view(project: &Project, id: Option<&str>) -> Result<ReportView> {
    let record = match id {
        Some(id) => load(project, id)?,
        None => latest(project)?,
    };
    let text = report_text(project, &record.id)?;
    let path = report_path(project, &record.id)?;
    Ok(ReportView {
        id: record.id.clone(),
        path,
        record,
        text,
    })
}

fn read_record(dir: &Utf8Path, id: &str) -> Result<RunRecord> {
    let folder = dir.join(id);
    if !folder.is_dir() {
        return Err(AuroraError::ReportNotFound(id.to_owned()));
    }
    let path = folder.join(RECORD_FILE);
    let invalid = |reason: String| AuroraError::InvalidReport {
        path: path.clone(),
        reason,
    };
    let text = std::fs::read_to_string(&path).map_err(|e| invalid(e.to_string()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| invalid(e.to_string()))?;
    let schema = value.get("schema").and_then(|s| s.as_u64());
    if schema != Some(u64::from(RECORD_SCHEMA)) {
        return Err(invalid(format!(
            "format {} (this AURORA reads format {RECORD_SCHEMA})",
            schema.map_or("unknown".into(), |s| s.to_string())
        )));
    }
    serde_json::from_value(value).map_err(|e| invalid(e.to_string()))
}

// ----------------------------------------------------------------- apagar

/// Quais relatórios [`plan_cleanup`] escolhe para apagar. Em JSON:
/// `{"kind": "all"}`, `{"kind": "keep_latest", "value": 5}`,
/// `{"kind": "reports", "value": ["run-000042", "7"]}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
#[non_exhaustive]
pub enum Cleanup {
    /// Todos.
    All,
    /// Todos menos os `n` mais novos.
    KeepLatest(usize),
    /// Estes, como o usuário escreveu (`run-000042` ou `42`).
    Reports(Vec<String>),
}

/// Os relatórios que `cleanup` escolhe, do mais antigo para o mais novo,
/// sem apagar nada. A interface mostra a lista, confirma e passa a mesma
/// lista para [`remove`]: um relatório gravado entre as duas chamadas não
/// sai sem ter sido mostrado.
///
/// # Erros
///
/// [`AuroraError::ReportNotFound`] se um relatório de
/// [`Cleanup::Reports`] não existe; nesse caso nada é escolhido.
pub fn plan_cleanup(project: &Project, cleanup: &Cleanup) -> Result<Vec<String>> {
    let dir = reports_dir(project);
    let mut ids = ids_in(&dir);
    ids.sort_unstable();
    let id = |n: &u64| format!("{ID_PREFIX}{n:06}");
    Ok(match cleanup {
        Cleanup::All => ids.iter().map(id).collect(),
        Cleanup::KeepLatest(keep) => {
            let end = ids.len().saturating_sub(*keep);
            ids[..end].iter().map(id).collect()
        }
        Cleanup::Reports(wanted) => {
            let mut chosen = Vec::new();
            for text in wanted {
                let id = parse_id(text).ok_or_else(|| AuroraError::ReportNotFound(text.clone()))?;
                if !dir.join(&id).is_dir() {
                    return Err(AuroraError::ReportNotFound(id));
                }
                if !chosen.contains(&id) {
                    chosen.push(id);
                }
            }
            chosen.sort();
            chosen
        }
    })
}

/// Apaga os relatórios `ids` (os de [`plan_cleanup`]) e devolve os que
/// saíram. Um que já não existe é pulado.
///
/// O número de um relatório apagado não volta: antes de apagar, `sequence`
/// passa a guardar pelo menos o maior número publicado. Cada pasta é
/// renomeada para um nome oculto antes de ser apagada, então quem lê o
/// histórico nunca vê um relatório pela metade; uma pasta dessas que ficou
/// de uma limpeza interrompida sai na próxima.
///
/// # Erros
///
/// [`AuroraError::ReportNotFound`] se um identificador não é de relatório;
/// [`AuroraError::Io`] se uma pasta não pôde ser apagada (as anteriores já
/// saíram).
pub fn remove(project: &Project, ids: &[String]) -> Result<Vec<String>> {
    let dir = reports_dir(project);
    let ids = ids
        .iter()
        .map(|text| parse_id(text).ok_or_else(|| AuroraError::ReportNotFound(text.clone())))
        .collect::<Result<Vec<_>>>()?;
    if ids.is_empty() || !dir.is_dir() {
        return Ok(Vec::new());
    }
    let sequence = dir.join(SEQUENCE_FILE);
    let last = read_sequence(&sequence).max(highest_id(&dir));
    write_atomically(&sequence, &format!("{last}\n"))?;

    let mut removed = Vec::new();
    for id in ids {
        let folder = dir.join(&id);
        let doomed = dir.join(format!("{REMOVING_PREFIX}{id}-{}", std::process::id()));
        match std::fs::rename(&folder, &doomed) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(AuroraError::io("Removing report", &folder)(e)),
        }
        std::fs::remove_dir_all(&doomed).map_err(AuroraError::io("Removing report", &doomed))?;
        removed.push(id);
    }
    // O que sobrou de uma limpeza interrompida.
    if let Ok(entries) = dir.read_dir_utf8() {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.file_name().starts_with(REMOVING_PREFIX) {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    Ok(removed)
}

/// `history.clean`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CleanOutcome {
    /// Os apagados, do mais antigo para o mais novo.
    pub removed: Vec<String>,
    /// Quantos ficaram.
    pub kept: usize,
}

/// Apaga os relatórios `ids` ([`remove`]) e conta os que ficaram: o
/// `aurora report clean` e o `history.clean`, depois do plano
/// ([`plan_cleanup`]).
///
/// # Erros
///
/// Os de [`remove`] e de [`list`].
pub fn clean(project: &Project, ids: &[String]) -> Result<CleanOutcome> {
    let removed = remove(project, ids)?;
    let kept = list(project)?.len();
    Ok(CleanOutcome { removed, kept })
}

// --------------------------------------------------------------- comparar

/// Compara dois relatórios do projeto, como `aurora report compare`.
///
/// Sem `current`, o atual é o mais novo. Sem `baseline`, a referência é o
/// relatório mais novo, entre os anteriores ao atual, que se compara com ele
/// em pelo menos uma parte e terminou bem ([`previous_comparable`]). Uma
/// referência pedida é usada como está, sem trocar por outra.
///
/// Os dois vêm do histórico deste projeto, então são do mesmo projeto mesmo
/// que a pasta tenha mudado de lugar depois de um deles (o `project` do
/// relatório é um hash da raiz, e [`compare`], com relatórios soltos,
/// recusaria).
///
/// # Erros
///
/// [`AuroraError::NotComparable`] se o atual (ou a referência pedida) não tem
/// estatísticas de síntese nem tempos de simulação, se não há anterior
/// compatível, ou se os dois não têm parte nenhuma em comum (uma simulação
/// contra uma síntese, topos diferentes); os de [`load`] e [`latest`].
pub fn compare_reports(
    project: &Project,
    current: Option<&str>,
    baseline: Option<&str>,
) -> Result<RunComparison> {
    let current = match current {
        Some(id) => load(project, id)?,
        None => latest(project)?,
    };
    if current.synthesis.is_none() && current.simulation.is_none() {
        return Err(AuroraError::NotComparable(format!(
            "Report {} has no synthesis statistics or simulation timings to compare",
            current.id
        )));
    }
    let baseline = match baseline {
        Some(id) => {
            let baseline = load(project, id)?;
            if baseline.synthesis.is_none() && baseline.simulation.is_none() {
                return Err(AuroraError::NotComparable(format!(
                    "Report {} has no synthesis statistics or simulation timings to compare",
                    baseline.id
                )));
            }
            baseline
        }
        None => previous_comparable(project, &current)?,
    };
    let comparison = compare_records(&baseline, &current, false)?;
    if comparison.synthesis.is_none() && comparison.simulation.is_none() {
        let why = if comparison.warnings.is_empty() {
            "one has synthesis statistics and the other simulation timings".to_owned()
        } else {
            comparison.warnings.join("; ")
        };
        return Err(AuroraError::NotComparable(format!(
            "Reports {} and {} have nothing to compare: {why}",
            baseline.id, current.id
        )));
    }
    Ok(comparison)
}

/// O relatório mais novo, entre os anteriores a `current`, que se compara
/// com ele em pelo menos uma parte. Prefere um que terminou bem: uma
/// simulação que estourou o prazo ou falhou só vira referência se não há
/// outra.
///
/// # Erros
///
/// [`AuroraError::NotComparable`] se não há nenhum.
pub fn previous_comparable(project: &Project, current: &RunRecord) -> Result<RunRecord> {
    let dir = reports_dir(project);
    let mine = id_number(&current.id).unwrap_or(u64::MAX);
    let mut older: Vec<u64> = ids_in(&dir).into_iter().filter(|n| *n < mine).collect();
    older.sort_unstable_by(|a, b| b.cmp(a));
    let mut fallback = None;
    for n in older {
        let Ok(candidate) = read_record(&dir, &format!("{ID_PREFIX}{n:06}")) else {
            continue;
        };
        let Ok(c) = compare_records(&candidate, current, false) else {
            continue;
        };
        if c.synthesis.is_none() && c.simulation.is_none() {
            continue;
        }
        let finished = candidate.metadata.status == Status::Succeeded
            && candidate.simulation.as_ref().is_none_or(|t| t.succeeded);
        if finished {
            return Ok(candidate);
        }
        fallback.get_or_insert(candidate);
    }
    fallback.ok_or_else(|| {
        AuroraError::NotComparable(format!(
            "No earlier report is comparable with {}",
            current.id
        ))
    })
}

/// Compara `current` com `baseline`. Não lê nem roda nada.
///
/// A síntese se compara quando os dois têm estatísticas do mesmo topo; a
/// simulação, quando os dois têm tempos do mesmo simulador e testbench. O
/// resto que difere vira aviso.
///
/// # Erros
///
/// [`AuroraError::NotComparable`] se os relatórios são de projetos
/// diferentes.
pub fn compare(baseline: &RunRecord, current: &RunRecord) -> Result<RunComparison> {
    compare_records(baseline, current, true)
}

/// [`compare`], conferindo ou não o projeto: do histórico de um projeto,
/// os relatórios são dele, mesmo com a pasta movida.
fn compare_records(
    baseline: &RunRecord,
    current: &RunRecord,
    check_project: bool,
) -> Result<RunComparison> {
    let (b, c) = (&baseline.metadata, &current.metadata);
    if check_project && b.project != c.project {
        return Err(AuroraError::NotComparable(format!(
            "Reports {} and {} belong to different projects",
            baseline.id, current.id
        )));
    }
    let mut warnings = Vec::new();
    let mut warn_if = |differs: bool, message: &str| {
        if differs {
            warnings.push(message.to_owned());
        }
    };

    let mut synthesis = None;
    if let (Some(bs), Some(cs)) = (&baseline.synthesis, &current.synthesis) {
        if bs.top == cs.top {
            synthesis = Some(compare_synthesis(bs, cs));
            warn_if(bs.tool != cs.tool, "synthesis tool versions differ");
            if let (Some(bc), Some(cc)) = (&b.synthesis, &c.synthesis) {
                warn_if(bc.sources != cc.sources, "synthesis sources differ");
            }
        } else {
            warn_if(
                true,
                "synthesis comparison unavailable because top modules differ",
            );
        }
    }

    let mut simulation = None;
    if let (Some(bt), Some(ct)) = (&baseline.simulation, &current.simulation) {
        match (&b.simulation, &c.simulation) {
            (Some(bc), Some(cc))
                if bc.simulator == cc.simulator && bc.testbench == cc.testbench =>
            {
                let comparison = compare_timings(bt, ct, b, c);
                warn_if(
                    bc.component_version != cc.component_version,
                    "simulator versions differ",
                );
                warn_if(bc.sources != cc.sources, "simulation sources differ");
                warn_if(bc.inputs != cc.inputs, "simulation inputs differ");
                warn_if(
                    bc.waveform != cc.waveform,
                    "waveform generation was enabled in only one run",
                );
                warn_if(
                    !matches!(
                        comparison.simulated.change,
                        Change::Unchanged | Change::NotComparable
                    ),
                    "simulated HDL durations differ",
                );
                warn_if(
                    comparison.environment_changed,
                    "runs were measured in different execution environments",
                );
                warn_if(
                    !bt.succeeded || !ct.succeeded,
                    "one or both simulations did not complete successfully",
                );
                simulation = Some(comparison);
            }
            (Some(bc), Some(cc)) => warn_if(
                true,
                if bc.simulator != cc.simulator {
                    "simulation timing comparison unavailable because simulators differ"
                } else {
                    "simulation timing comparison unavailable because testbenches differ"
                },
            ),
            _ => {}
        }
    }
    warn_if(
        b.aurora_version != c.aurora_version,
        "AURORA versions differ",
    );

    Ok(RunComparison {
        current_id: current.id.clone(),
        current: c.clone(),
        baseline_id: baseline.id.clone(),
        baseline: b.clone(),
        synthesis,
        simulation,
        warnings,
    })
}

/// Compara dois números. Sem um dos lados, `NotComparable`. A porcentagem
/// é da referência; de zero para outro número não há porcentagem.
pub fn compare_metric(baseline: Option<u64>, current: Option<u64>) -> MetricComparison {
    let (Some(b), Some(c)) = (baseline, current) else {
        return MetricComparison {
            baseline,
            current,
            change: Change::NotComparable,
            delta: None,
            percent: None,
        };
    };
    let change = match c.cmp(&b) {
        std::cmp::Ordering::Greater => Change::Increased,
        std::cmp::Ordering::Less => Change::Decreased,
        std::cmp::Ordering::Equal => Change::Unchanged,
    };
    let delta = i128::from(c) - i128::from(b);
    let percent = match b {
        0 if c == 0 => Some(0.0),
        0 => None,
        _ => Some(delta as f64 * 100.0 / b as f64),
    };
    MetricComparison {
        baseline,
        current,
        change,
        delta: Some(i64::try_from(delta).unwrap_or(if delta < 0 { i64::MIN } else { i64::MAX })),
        percent,
    }
}

fn compare_synthesis(b: &SynthesisStatistics, c: &SynthesisStatistics) -> SynthesisComparison {
    let metrics: Vec<SynthesisMetricComparison> = SynthesisMetric::ALL
        .iter()
        .map(|&metric| SynthesisMetricComparison {
            metric,
            comparison: compare_metric(b.get(metric), c.get(metric)),
        })
        .collect();
    let count = |change: Change| {
        metrics
            .iter()
            .filter(|m| m.comparison.change == change)
            .count()
    };
    let mut cell_types: Vec<CellComparison> = Vec::new();
    for cell in &b.cell_types {
        let other = c.cell_types.iter().find(|o| o.cell_type == cell.cell_type);
        let mut usage = compare_metric(Some(cell.count), Some(other.map_or(0, |o| o.count)));
        if other.is_none() {
            usage.change = Change::Removed;
        }
        cell_types.push(CellComparison {
            cell_type: cell.cell_type.clone(),
            usage,
        });
    }
    for cell in &c.cell_types {
        if !b.cell_types.iter().any(|o| o.cell_type == cell.cell_type) {
            let mut usage = compare_metric(Some(0), Some(cell.count));
            usage.change = Change::Added;
            cell_types.push(CellComparison {
                cell_type: cell.cell_type.clone(),
                usage,
            });
        }
    }
    cell_types.sort_by(|x, y| {
        let magnitude = |c: &CellComparison| c.usage.delta.unwrap_or(0).unsigned_abs();
        magnitude(y)
            .cmp(&magnitude(x))
            .then_with(|| x.cell_type.cmp(&y.cell_type))
    });
    SynthesisComparison {
        added_cell_types: cell_types
            .iter()
            .filter(|c| c.usage.change == Change::Added)
            .count(),
        removed_cell_types: cell_types
            .iter()
            .filter(|c| c.usage.change == Change::Removed)
            .count(),
        increased: count(Change::Increased),
        decreased: count(Change::Decreased),
        unchanged: count(Change::Unchanged),
        not_comparable: count(Change::NotComparable),
        metrics,
        cell_types,
    }
}

fn compare_timings(
    b: &SimulationTimings,
    c: &SimulationTimings,
    bm: &RunMetadata,
    cm: &RunMetadata,
) -> TimingComparison {
    TimingComparison {
        compile: compare_metric(b.compile_ms, c.compile_ms),
        execution: compare_metric(b.execution_ms, c.execution_ms),
        total: compare_metric(b.total_ms, c.total_ms),
        simulated: compare_metric(b.simulated_fs, c.simulated_fs),
        environment_changed: bm.environment.fingerprint != cm.environment.fingerprint,
    }
}

// ------------------------------------------------------- montar o registro

fn metadata(
    project: &Project,
    toolchain: &Toolchain,
    operation: &Operation,
    finished: SystemTime,
) -> RunMetadata {
    let version_of = |component: &str| toolchain.component(component).map(|c| c.version.clone());
    let synthesis = operation.synthesis.map(|s| SynthesisContext {
        top: s.top.clone(),
        component_version: version_of(crate::toolchain::component::YOSYS),
        sources: sources_fingerprint(project, false),
    });
    let simulation = operation.simulation.map(|s| SimulationContext {
        simulator: s.simulator,
        testbench: s.top.clone(),
        component_version: version_of(match s.simulator {
            Simulator::Icarus => crate::toolchain::component::ICARUS,
            Simulator::Verilator => crate::toolchain::component::VERILATOR,
            Simulator::Ghdl => crate::toolchain::component::GHDL,
        }),
        sources: sources_fingerprint(project, true),
        inputs: inputs_fingerprint(project),
        waveform: s.waveform.is_some(),
    });
    let top = operation
        .synthesis
        .map(|s| s.top.clone())
        .or_else(|| operation.simulation.map(|s| s.top.clone()))
        .or_else(|| project.top_module().ok().flatten());
    let manifest = toolchain.manifest();
    RunMetadata {
        timestamp: utc_seconds(finished),
        command: operation.command.clone(),
        status: operation.status(),
        duration_ms: finished
            .duration_since(operation.started)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX)),
        aurora_version: env!("CARGO_PKG_VERSION").to_owned(),
        project: Fnv::new().text(project.root().as_str()).hex(),
        project_name: project.name().to_owned(),
        bundle: manifest.bundle.clone(),
        platform: manifest.platform.clone(),
        top,
        synthesis,
        simulation,
        environment: environment(),
    }
}

fn timings(s: &SimulationResult) -> SimulationTimings {
    let step = |wanted: &[Step]| {
        s.steps
            .iter()
            .filter(|r| wanted.contains(&r.step))
            .map(|r| r.duration_ms)
            .reduce(u64::saturating_add)
    };
    let simulated_fs = s
        .steps
        .iter()
        .filter(|r| r.step == Step::Simulate)
        .find_map(|r| simulated_time(&r.stdout).or_else(|| simulated_time(&r.stderr)));
    SimulationTimings {
        succeeded: s.status == Status::Succeeded,
        compile_ms: step(&[Step::Elaborate, Step::Verilate]),
        execution_ms: step(&[Step::Simulate]),
        total_ms: Some(s.duration_ms),
        simulated_fs,
    }
}

/// O tempo do `$finish` que o `vvp` informa (`tb.v:61: $finish called at
/// 985000 (1ps)`), em femtossegundos.
fn simulated_time(output: &str) -> Option<u64> {
    let line = output
        .lines()
        .rev()
        .find(|l| l.contains("$finish called at"))?;
    let rest = line.split("$finish called at").nth(1)?.trim();
    let (value, unit) = rest.split_once('(')?;
    let value: u64 = value.trim().parse().ok()?;
    let unit = unit.trim_end_matches(')').trim();
    let digits: String = unit.chars().take_while(char::is_ascii_digit).collect();
    let scale: u64 = if digits.is_empty() {
        1
    } else {
        digits.parse().ok()?
    };
    let fs: u64 = match &unit[digits.len()..] {
        "s" => 1_000_000_000_000_000,
        "ms" => 1_000_000_000_000,
        "us" => 1_000_000_000,
        "ns" => 1_000_000,
        "ps" => 1_000,
        "fs" => 1,
        _ => return None,
    };
    value.checked_mul(scale)?.checked_mul(fs)
}

/// Hash dos fontes do projeto: os Verilog registrados (com os testbenches,
/// para a simulação) e os programas dos processadores.
fn sources_fingerprint(project: &Project, with_testbenches: bool) -> String {
    let mut files: Vec<Utf8PathBuf> = project
        .files(FileRole::Synthesizable)
        .into_iter()
        .map(|f| f.path)
        .collect();
    if with_testbenches {
        files.extend(
            project
                .files(FileRole::Testbench)
                .into_iter()
                .map(|f| f.path),
        );
    }
    files.extend(project.processors().iter().map(|p| p.source.clone()));
    files.sort();
    let mut hash = Fnv::new();
    for file in &files {
        hash = hash.text(file.as_str()).file(file);
    }
    hash.hex()
}

/// Hash das entradas dos processadores (`<proc>/input_*.txt`, ao lado do
/// fonte).
fn inputs_fingerprint(project: &Project) -> String {
    let mut files: Vec<Utf8PathBuf> = project
        .processors()
        .iter()
        .flat_map(|p| {
            p.dir
                .read_dir_utf8()
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .map(|e| e.into_path())
                .filter(|path| {
                    path.file_name()
                        .is_some_and(|n| n.starts_with("input_") && n.ends_with(".txt"))
                })
                .collect::<Vec<_>>()
        })
        .collect();
    files.sort();
    let mut hash = Fnv::new();
    for file in &files {
        hash = hash.text(file.as_str()).file(file);
    }
    hash.hex()
}

/// FNV-1a de 64 bits, como o Alpha-Solar: só identifica mudança, não é
/// criptográfico.
#[derive(Clone, Copy)]
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }

    fn bytes(mut self, data: &[u8]) -> Self {
        for &b in data {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
        self
    }

    /// Um texto e um separador, para `"ab" + "c"` não dar o mesmo que
    /// `"a" + "bc"`.
    fn text(self, text: &str) -> Self {
        self.bytes(text.as_bytes()).bytes(&[0xff])
    }

    fn file(self, path: &Utf8Path) -> Self {
        match std::fs::read(path) {
            Ok(data) => self.bytes(&data).bytes(&[0xff]),
            Err(_) => self.text("<missing>"),
        }
    }

    fn hex(self) -> String {
        format!("{:016x}", self.0)
    }
}

/// O que o sistema informa da máquina sem rodar outro programa.
fn environment() -> Environment {
    let os = std::env::consts::OS.to_owned();
    let arch = std::env::consts::ARCH.to_owned();
    let (hostname, kernel) = host_and_kernel();
    let cpus = std::thread::available_parallelism()
        .ok()
        .map(|n| n.get() as u64);
    let cpu_model = cpu_model();
    let fingerprint = Fnv::new()
        .text(&os)
        .text(&arch)
        .text(cpu_model.as_deref().unwrap_or(""))
        .text(&cpus.map_or(String::new(), |n| n.to_string()))
        .text(hostname.as_deref().unwrap_or(""))
        .hex();
    Environment {
        hostname,
        os_name: os_name(),
        os,
        kernel,
        arch,
        cpu_model,
        cpus,
        memory_bytes: memory_bytes(),
        fingerprint,
    }
}

#[cfg(unix)]
fn host_and_kernel() -> (Option<String>, Option<String>) {
    let uname = rustix::system::uname();
    let text =
        |c: &std::ffi::CStr| Some(c.to_string_lossy().into_owned()).filter(|s| !s.is_empty());
    (text(uname.nodename()), text(uname.release()))
}

#[cfg(not(unix))]
fn host_and_kernel() -> (Option<String>, Option<String>) {
    (
        std::env::var("COMPUTERNAME").ok().filter(|s| !s.is_empty()),
        None,
    )
}

fn os_name() -> Option<String> {
    let text = std::fs::read_to_string("/etc/os-release").ok()?;
    let line = text.lines().find(|l| l.starts_with("PRETTY_NAME="))?;
    Some(line["PRETTY_NAME=".len()..].trim_matches('"').to_owned()).filter(|s| !s.is_empty())
}

fn cpu_model() -> Option<String> {
    let text = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    text.lines()
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| matches!(k.trim(), "model name" | "Model" | "Hardware"))
        .map(|(_, v)| v.trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn memory_bytes() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text.lines().find(|l| l.starts_with("MemTotal:"))?;
    let kib: u64 = line
        .trim_start_matches("MemTotal:")
        .trim()
        .trim_end_matches("kB")
        .trim()
        .parse()
        .ok()?;
    kib.checked_mul(1024)
}

// ------------------------------------------------------------- o texto

/// O `report.txt`: o que um humano lê de uma operação.
fn render(
    project: &Project,
    toolchain: &Toolchain,
    operation: &Operation,
    record: &RunRecord,
    finished: SystemTime,
) -> String {
    let phases = operation.phases();
    let m = &record.metadata;
    let mut out = String::new();
    let _ = writeln!(out, "{RULE}\nAURORA OPERATION REPORT\n{RULE}");

    section(&mut out, "OVERVIEW");
    field(&mut out, "Status", status_word(m.status));
    field(&mut out, "Command", &m.command);
    field(&mut out, "Report", &record.id);
    let total = if m.duration_ms < 1000 {
        duration(m.duration_ms)
    } else {
        format!("{} ({} ms)", duration(m.duration_ms), m.duration_ms)
    };
    field(&mut out, "Total wall time", &total);
    field(&mut out, "AURORA version", &m.aurora_version);
    field(&mut out, "Started (UTC)", &utc_seconds(operation.started));
    field(&mut out, "Finished (UTC)", &utc_seconds(finished));

    section(&mut out, "HOST ENVIRONMENT");
    let e = &m.environment;
    let not = |v: &Option<String>| v.clone().unwrap_or_else(|| "not reported".into());
    field(&mut out, "Hostname", &not(&e.hostname));
    field(
        &mut out,
        "Operating system",
        &e.os_name
            .as_ref()
            .map_or_else(|| e.os.clone(), |n| format!("{n} ({})", e.os)),
    );
    field(&mut out, "Kernel", &not(&e.kernel));
    field(&mut out, "Architecture", &e.arch);
    field(&mut out, "CPU model", &not(&e.cpu_model));
    field(
        &mut out,
        "Logical CPUs",
        &e.cpus
            .map_or("not reported".into(), |n| format!("{n} available")),
    );
    field(
        &mut out,
        "Memory",
        &e.memory_bytes.map_or("not reported".into(), |b| {
            format!("{:.2} GiB ({b} bytes)", b as f64 / 1_073_741_824.0)
        }),
    );
    let _ = writeln!(
        out,
        "  Host data is a snapshot collected when this report was written."
    );

    section(&mut out, "EDA TOOLCHAIN");
    toolchain_section(&mut out, toolchain, &phases);

    section(&mut out, "PROJECT");
    project_section(&mut out, project);

    section(&mut out, "TIMING BREAKDOWN");
    timing_section(&mut out, &phases, m.duration_ms);

    statistics_section(&mut out, operation, record);

    section(&mut out, "ARTIFACTS");
    artifacts_section(&mut out, project, &phases);

    if m.status != Status::Succeeded {
        section(&mut out, "FAILURE DIAGNOSTIC");
        failure_section(&mut out, project, &phases);
    }

    let _ = writeln!(
        out,
        "\n{RULE}\nReport file: {REPORTS_DIR}/{}/{TEXT_FILE}\n\
         Use repeated, controlled runs before treating these timings as a benchmark.",
        record.id
    );
    out
}

fn section(out: &mut String, title: &str) {
    let _ = writeln!(out, "\n{title}");
}

fn field(out: &mut String, label: &str, value: &str) {
    let _ = writeln!(out, "  {label:<20} {value}");
}

fn toolchain_section(out: &mut String, toolchain: &Toolchain, phases: &[Phase]) {
    let manifest = toolchain.manifest();
    field(
        out,
        "Bundle",
        &format!("{} ({})", manifest.bundle, manifest.platform),
    );
    field(out, "Location", toolchain.root().as_str());
    // O executável da ferramenta no bundle, e não o programa do passo: no
    // Linux e no macOS o passo roda o lançador pelo `/bin/bash`, e o
    // Verilator roda pelo Perl.
    let mut seen: Vec<(Tool, Utf8PathBuf)> = Vec::new();
    for step in phases.iter().flat_map(|p| p.steps) {
        let program = toolchain
            .tool(step.tool)
            .unwrap_or_else(|_| step.command.program.clone());
        let entry = (step.tool, program);
        if !seen.contains(&entry) {
            seen.push(entry);
        }
    }
    if seen.is_empty() {
        let _ = writeln!(out, "  No tool ran.");
        return;
    }
    let _ = writeln!(out, "  {:<14} {:<24} Executable", "Tool", "Component");
    for (tool, program) in &seen {
        let version = tool
            .component()
            .and_then(|c| toolchain.component(c))
            .map_or_else(
                || "system".to_owned(),
                |c| format!("{} {}", c.name, c.version),
            );
        let _ = writeln!(out, "  {:<14} {version:<24} {program}", tool.to_string());
    }
    let verilator = phases
        .iter()
        .flat_map(|p| p.steps)
        .any(|s| s.tool == Tool::Verilator);
    if verilator && let Some(c) = toolchain.system_compiler() {
        field(
            out,
            "Verilator compiler",
            &format!(
                "{} ({})",
                c.cxx,
                if c.bundled { "bundle" } else { "system" }
            ),
        );
    }
}

fn project_section(out: &mut String, project: &Project) {
    field(out, "Name", project.name());
    field(out, "Root", project.root().as_str());
    field(out, "Project file", project.spf_path().as_str());
    let relative = |p: &Utf8Path| p.strip_prefix(project.root()).unwrap_or(p).to_string();
    let module = |m: Result<Option<String>>| m.ok().flatten().unwrap_or_else(|| "-".into());
    field(
        out,
        "Top level",
        &project.top_level().map_or("none".into(), |p| {
            format!("{} ({})", relative(&p), module(project.top_module()))
        }),
    );
    field(
        out,
        "Testbench",
        &project.testbench().map_or("none".into(), |p| {
            format!("{} ({})", relative(&p), module(project.testbench_module()))
        }),
    );
    for (role, label) in [
        (FileRole::Synthesizable, "Modules"),
        (FileRole::Testbench, "Testbenches"),
    ] {
        let files = project.files(role);
        let _ = writeln!(out, "  {label:<20} {} registered", files.len());
        for f in files {
            let _ = writeln!(out, "      - {}", relative(&f.path));
        }
    }
    let processors = project.processors();
    let _ = writeln!(out, "  {:<20} {}", "Processors", processors.len());
    for p in processors {
        let language = match p.language {
            Language::Cmm => "C±",
            _ => "C",
        };
        let _ = writeln!(
            out,
            "      - {} | {language} | {} MHz | {} clocks | {}",
            p.name,
            p.frequency_mhz,
            p.clocks,
            relative(&p.source)
        );
    }
}

fn timing_section(out: &mut String, phases: &[Phase], total_ms: u64) {
    let _ = writeln!(out, "  Phases");
    for phase in phases {
        timing_row(
            out,
            status_word(phase.status),
            &phase.label,
            phase.duration_ms,
        );
    }
    timing_row(out, "TOTAL", "AURORA operation", total_ms);
    let _ = writeln!(out, "\n  Tool runs");
    let mut any = false;
    for phase in phases {
        let last = phase.steps.len().saturating_sub(1);
        for (i, step) in phase.steps.iter().enumerate() {
            any = true;
            // FAIL só para o passo que saiu com 0 e reprovou a fase (o `vvp`
            // depois de um `$error`); o resto diz como terminou (`TIMEOUT`,
            // `EXIT 1`).
            let state = if i == last
                && Some(step.step) == phase.failed_step
                && step.termination == Termination::Exited(0)
                && phase.status != Status::Succeeded
            {
                "FAIL".to_owned()
            } else {
                termination_word(step.termination)
            };
            timing_row(
                out,
                &state,
                &format!("{}: {}", step_name(step.step), step.tool),
                step.duration_ms,
            );
        }
    }
    if !any {
        let _ = writeln!(out, "  No tool ran.");
    }
    let _ = writeln!(
        out,
        "\n  Scope: monotonic wall time. Tool runs include process launch, wait and output drain.\n  \
         Phase times also include AURORA's preparation and artifact checks.\n  \
         A single run is not a statistical benchmark; repeat under controlled load before publishing numbers."
    );
}

fn timing_row(out: &mut String, state: &str, label: &str, ms: u64) {
    let _ = writeln!(out, "  {state:<9} {label:<44} {:>14}", duration(ms));
}

fn statistics_section(out: &mut String, operation: &Operation, record: &RunRecord) {
    section(out, "GENERIC SYNTHESIS STATISTICS");
    let synthesis = operation.synthesis;
    let Some(statistics) = &record.synthesis else {
        field(out, "Status", "NOT AVAILABLE");
        let reason = match synthesis {
            None => "synthesis was not run",
            Some(s) if s.status != Status::Succeeded => "synthesis failed",
            Some(_) => "Yosys did not write its statistics",
        };
        field(out, "Reason", reason);
        return;
    };
    field(
        out,
        "Status",
        if statistics.is_complete() {
            "COMPLETE"
        } else {
            "PARTIAL"
        },
    );
    field(out, "Tool", &statistics.tool);
    field(out, "Top", &statistics.top);
    if let Some(stat) = synthesis.and_then(|s| {
        s.artifacts
            .iter()
            .find(|a| a.kind == crate::pipeline::ArtifactKind::SynthesisStatistics)
    }) {
        field(out, "Source", stat.path.as_str());
    }
    field(out, "Data schema", "aurora-generic-synthesis-statistics/1");
    let _ = writeln!(
        out,
        "\n  DESIGN SUMMARY\n  {:<32} {:>12}\n  {} {}",
        "Metric",
        "Value",
        "-".repeat(32),
        "-".repeat(12)
    );
    for metric in SynthesisMetric::ALL {
        let value = statistics
            .get(metric)
            .map_or("not reported".to_owned(), |v| v.to_string());
        let _ = writeln!(out, "  {:<32} {value:>12}", metric.label());
    }
    let _ = writeln!(
        out,
        "\n  CELL USAGE\n  {:<32} {:>12}\n  {} {}",
        "Cell type",
        "Count",
        "-".repeat(32),
        "-".repeat(12)
    );
    if statistics.cell_types.is_empty() {
        let _ = writeln!(out, "  {:<32} {:>12}", "-", "not reported");
    }
    for cell in &statistics.cell_types {
        let _ = writeln!(out, "  {:<32} {:>12}", cell.cell_type, cell.count);
    }
    let _ = writeln!(
        out,
        "\n  Generic Yosys cells after proc and opt_clean, not mapped to an FPGA: no LUT,\n  \
         DSP, utilization or timing estimate."
    );
}

fn artifacts_section(out: &mut String, project: &Project, phases: &[Phase]) {
    let mut any = false;
    for artifact in phases.iter().flat_map(|p| p.artifacts) {
        any = true;
        let path = artifact
            .path
            .strip_prefix(project.root())
            .map_or_else(|_| artifact.path.to_string(), |p| p.to_string());
        let state = if artifact.fresh {
            ""
        } else if artifact.path.exists() {
            "  (left from an earlier run)"
        } else {
            "  (missing)"
        };
        let kind = serde_json::to_value(artifact.kind)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        let _ = writeln!(out, "  {kind:<22} {path}{state}");
    }
    if !any {
        let _ = writeln!(out, "  None.");
    }
}

fn failure_section(out: &mut String, project: &Project, phases: &[Phase]) {
    let Some(phase) = phases.iter().find(|p| p.status != Status::Succeeded) else {
        return;
    };
    field(out, "Phase", &phase.label);
    field(out, "Status", status_word(phase.status));
    if let Some(error) = phase
        .diagnostics
        .iter()
        .find(|d| d.severity == Severity::Error)
    {
        let location = match (&error.file, error.line) {
            (Some(file), Some(line)) => format!(
                "{}:{line}: ",
                file.strip_prefix(project.root()).unwrap_or(file)
            ),
            (Some(file), None) => {
                format!("{}: ", file.strip_prefix(project.root()).unwrap_or(file))
            }
            _ => String::new(),
        };
        field(out, "Error", &format!("{location}{}", error.message));
    }
    let failed = phase
        .failed_step
        .and_then(|s| phase.steps.iter().rev().find(|r| r.step == s))
        .or_else(|| phase.steps.last());
    if let Some(step) = failed {
        field(
            out,
            "Step",
            &format!(
                "{}: {} ({})",
                step_name(step.step),
                step.tool,
                termination_word(step.termination)
            ),
        );
        let text = if step.stderr.trim().is_empty() {
            &step.stdout
        } else {
            &step.stderr
        };
        let lines: Vec<&str> = text.lines().collect();
        if !lines.is_empty() {
            let _ = writeln!(out, "  Last lines of its output:");
            for line in &lines[lines.len().saturating_sub(FAILURE_TAIL)..] {
                let _ = writeln!(out, "    {line}");
            }
        }
    }
}

fn status_word(status: Status) -> &'static str {
    match status {
        Status::Succeeded => "PASS",
        Status::Failed => "FAIL",
        Status::Crashed => "CRASH",
        Status::Incomplete => "INCOMPLETE",
        Status::Cancelled => "CANCELLED",
        Status::TimedOut => "TIMEOUT",
    }
}

fn termination_word(termination: Termination) -> String {
    match termination {
        Termination::Exited(0) => "PASS".into(),
        Termination::Exited(code) => format!("EXIT {code}"),
        Termination::Signaled(signal) => format!("SIGNAL {signal}"),
        Termination::Exception(code) => format!("0x{code:08X}"),
        Termination::Cancelled => "CANCELLED".into(),
        Termination::TimedOut => "TIMEOUT".into(),
        Termination::Unknown => "UNKNOWN".into(),
    }
}

fn step_name(step: Step) -> String {
    serde_json::to_value(step)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn simulator_name(simulator: Simulator) -> &'static str {
    match simulator {
        Simulator::Icarus => "Icarus",
        Simulator::Verilator => "Verilator",
        Simulator::Ghdl => "GHDL",
    }
}

/// Milissegundos para o texto: `850 ms`, `1.234 s`.
pub fn duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms} ms")
    } else {
        format!("{}.{:03} s", ms / 1000, ms % 1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::CellUsage;

    fn statistics(top: &str, cells: u64, types: &[(&str, u64)]) -> SynthesisStatistics {
        SynthesisStatistics {
            tool: "Yosys 0.69".into(),
            top: top.into(),
            modules: Some(1),
            wires: Some(10),
            wire_bits: Some(40),
            public_wires: Some(4),
            public_wire_bits: Some(16),
            memories: Some(0),
            memory_bits: Some(0),
            processes: None,
            cells: Some(cells),
            cell_types: types
                .iter()
                .map(|(t, c)| CellUsage {
                    cell_type: (*t).into(),
                    count: *c,
                })
                .collect(),
        }
    }

    fn metadata(project: &str) -> RunMetadata {
        RunMetadata {
            timestamp: "2026-10-03T00:00:00Z".into(),
            command: "aurora synth".into(),
            status: Status::Succeeded,
            duration_ms: 1,
            aurora_version: "0.2.0".into(),
            project: project.into(),
            project_name: "soma".into(),
            bundle: "2026.09.29".into(),
            platform: "linux-x64".into(),
            top: Some("soma".into()),
            synthesis: Some(SynthesisContext {
                top: "soma".into(),
                component_version: Some("2026-09-29".into()),
                sources: "a".into(),
            }),
            simulation: Some(SimulationContext {
                simulator: Simulator::Icarus,
                testbench: "soma_tb".into(),
                component_version: Some("2026-09-29".into()),
                sources: "a".into(),
                inputs: "b".into(),
                waveform: true,
            }),
            environment: Environment {
                hostname: Some("h".into()),
                os: "linux".into(),
                os_name: None,
                kernel: None,
                arch: "x86_64".into(),
                cpu_model: None,
                cpus: Some(8),
                memory_bytes: None,
                fingerprint: "e".into(),
            },
        }
    }

    fn run(id: &str, synthesis: Option<SynthesisStatistics>, total: Option<u64>) -> RunRecord {
        RunRecord {
            schema: RECORD_SCHEMA,
            id: id.into(),
            metadata: metadata("p"),
            synthesis,
            simulation: total.map(|t| SimulationTimings {
                succeeded: true,
                compile_ms: Some(10),
                execution_ms: Some(t - 10),
                total_ms: Some(t),
                simulated_fs: Some(985_000_000),
            }),
        }
    }

    #[test]
    fn a_metric_compares_with_sign_and_percent() {
        let up = compare_metric(Some(200), Some(250));
        assert_eq!(up.change, Change::Increased);
        assert_eq!(up.delta, Some(50));
        assert_eq!(up.percent, Some(25.0));
        let down = compare_metric(Some(200), Some(150));
        assert_eq!(down.change, Change::Decreased);
        assert_eq!(down.delta, Some(-50));
        assert_eq!(down.percent, Some(-25.0));
        assert_eq!(compare_metric(Some(7), Some(7)).change, Change::Unchanged);
        // De zero para outro número não há porcentagem; de zero para zero, 0%.
        assert_eq!(compare_metric(Some(0), Some(5)).percent, None);
        assert_eq!(compare_metric(Some(0), Some(0)).percent, Some(0.0));
        // Ausente não é zero.
        let missing = compare_metric(None, Some(5));
        assert_eq!(missing.change, Change::NotComparable);
        assert_eq!(missing.delta, None);
    }

    #[test]
    fn cells_are_compared_by_largest_change_with_added_and_removed() {
        let baseline = run(
            "run-000001",
            Some(statistics(
                "soma",
                6,
                &[("$add", 2), ("$dff", 3), ("$mux", 1)],
            )),
            None,
        );
        let current = run(
            "run-000002",
            Some(statistics(
                "soma",
                9,
                &[("$add", 2), ("$dff", 6), ("$xor", 1)],
            )),
            None,
        );
        let c = compare(&baseline, &current).unwrap();
        let s = c.synthesis.unwrap();
        let order: Vec<_> = s
            .cell_types
            .iter()
            .map(|c| (c.cell_type.as_str(), c.usage.change))
            .collect();
        assert_eq!(
            order,
            [
                ("$dff", Change::Increased),
                ("$mux", Change::Removed),
                ("$xor", Change::Added),
                ("$add", Change::Unchanged),
            ]
        );
        assert_eq!((s.added_cell_types, s.removed_cell_types), (1, 1));
        let cells = s.metric(SynthesisMetric::Cells).unwrap();
        assert_eq!(cells.delta, Some(3));
        // Os processos não foram informados nos dois: não se comparam.
        assert_eq!(s.not_comparable, 1);
        assert_eq!(s.increased, 1);
        assert!(c.simulation.is_none());
    }

    #[test]
    fn context_differences_become_warnings() {
        let baseline = run("run-000001", Some(statistics("soma", 6, &[])), Some(100));
        let mut current = run("run-000002", Some(statistics("outro", 6, &[])), Some(120));
        current.metadata.environment.fingerprint = "f".into();
        current.metadata.simulation.as_mut().unwrap().sources = "z".into();
        let c = compare(&baseline, &current).unwrap();
        assert!(c.synthesis.is_none());
        let t = c.simulation.unwrap();
        assert_eq!(t.total.delta, Some(20));
        assert!(t.environment_changed);
        for w in [
            "synthesis comparison unavailable because top modules differ",
            "simulation sources differ",
            "runs were measured in different execution environments",
        ] {
            assert!(c.warnings.iter().any(|x| x == w), "{w}: {:?}", c.warnings);
        }

        // Testbench diferente: a simulação não se compara.
        current.metadata.simulation.as_mut().unwrap().testbench = "outro_tb".into();
        let c = compare(&baseline, &current).unwrap();
        assert!(c.simulation.is_none());
        assert!(c.warnings.iter().any(|w| w.contains("testbenches differ")));

        // Projetos diferentes não se comparam.
        current.metadata.project = "q".into();
        assert!(matches!(
            compare(&baseline, &current),
            Err(AuroraError::NotComparable(_))
        ));
    }

    #[test]
    fn the_finish_time_of_vvp_becomes_femtoseconds() {
        assert_eq!(
            simulated_time("x\ntb.v:61: $finish called at 985000 (1ps)\n"),
            Some(985_000_000)
        );
        assert_eq!(
            simulated_time("$finish called at 12 (10ns)"),
            Some(120_000_000)
        );
        assert_eq!(
            simulated_time("$finish called at 3 (1s)"),
            Some(3_000_000_000_000_000)
        );
        assert_eq!(simulated_time("- tb.v:20: Verilog $finish"), None);
        assert_eq!(simulated_time("$finish called at 3 (1xs)"), None);
    }

    #[test]
    fn ids_accept_the_number_alone() {
        assert_eq!(parse_id("42").as_deref(), Some("run-000042"));
        assert_eq!(parse_id("run-000042").as_deref(), Some("run-000042"));
        assert_eq!(parse_id("run-1234567").as_deref(), Some("run-1234567"));
        assert_eq!(parse_id("run-42"), None);
        assert_eq!(parse_id("build-000042"), None);
        assert_eq!(parse_id(""), None);
        assert_eq!(duration(850), "850 ms");
        assert_eq!(duration(1234), "1.234 s");
    }

    #[test]
    fn numbers_only_grow_and_a_record_is_never_half_written() {
        let dir = tempfile::tempdir().unwrap();
        let dir = Utf8Path::from_path(dir.path()).unwrap();
        for expected in ["run-000001", "run-000002"] {
            let mut seen = String::new();
            store(dir, |id| {
                seen = id.to_owned();
                ("text".into(), "{}".into())
            })
            .unwrap();
            assert_eq!(seen, expected);
            assert!(dir.join(expected).join(TEXT_FILE).is_file());
        }
        // Apagar o mais novo não devolve o número dele.
        std::fs::remove_dir_all(dir.join("run-000002")).unwrap();
        let mut seen = String::new();
        store(dir, |id| {
            seen = id.to_owned();
            ("text".into(), "{}".into())
        })
        .unwrap();
        assert_eq!(seen, "run-000003");
        // Nenhuma pasta provisória sobra.
        let leftovers: Vec<_> = dir
            .read_dir_utf8()
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().starts_with(".pending"))
            .collect();
        assert!(leftovers.is_empty());
    }

    /// Grava um relatório no histórico do projeto, como `record` faria.
    fn stored(project: &Project, record: &RunRecord) {
        let dir = reports_dir(project).join(&record.id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(RECORD_FILE),
            serde_json::to_string(record).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join(TEXT_FILE), "").unwrap();
    }

    fn project() -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dunce::canonicalize(dir.path()).unwrap()).unwrap();
        let project = Project::create(&root, "p").unwrap();
        (dir, project)
    }

    #[test]
    fn the_automatic_reference_prefers_a_run_that_finished() {
        let (_guard, project) = project();
        stored(&project, &run("run-000001", None, Some(100)));
        let mut expired = run("run-000002", None, Some(2300));
        expired.metadata.status = Status::TimedOut;
        expired.simulation.as_mut().unwrap().succeeded = false;
        stored(&project, &expired);
        stored(&project, &run("run-000003", None, Some(120)));
        let c = compare_reports(&project, None, None).unwrap();
        assert_eq!(c.baseline_id, "run-000001");
        // Sem outro, o que estourou o prazo ainda serve.
        let (_guard2, alone) = self::project();
        stored(&alone, &expired);
        stored(&alone, &run("run-000003", None, Some(120)));
        assert_eq!(
            compare_reports(&alone, None, None).unwrap().baseline_id,
            "run-000002"
        );
    }

    #[test]
    fn reports_of_a_moved_project_still_compare() {
        let (_guard, project) = project();
        let mut before = run("run-000001", None, Some(100));
        before.metadata.project = "raiz-antiga".into();
        stored(&project, &before);
        stored(&project, &run("run-000002", None, Some(110)));
        let c = compare_reports(&project, Some("2"), Some("1")).unwrap();
        assert!(c.simulation.is_some());
        // Relatórios soltos de projetos diferentes continuam recusados.
        assert!(compare(&before, &run("run-000002", None, Some(110))).is_err());
    }

    #[test]
    fn a_simulation_against_a_synthesis_is_not_comparable() {
        let (_guard, project) = project();
        stored(
            &project,
            &run("run-000001", Some(statistics("soma", 6, &[])), None),
        );
        stored(&project, &run("run-000002", None, Some(110)));
        assert!(matches!(
            compare_reports(&project, Some("2"), Some("1")),
            Err(AuroraError::NotComparable(_))
        ));
    }

    #[test]
    fn a_timed_out_tool_run_is_not_shown_as_fail() {
        let step = |termination| StepReport {
            step: Step::Simulate,
            tool: crate::toolchain::Tool::Vvp,
            command: crate::process::Invocation::new("/bin/vvp", "/"),
            termination,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 3500,
        };
        let steps = [step(Termination::TimedOut)];
        let phase = Phase {
            label: "Simulation".into(),
            status: Status::TimedOut,
            failed_step: Some(Step::Simulate),
            duration_ms: 3500,
            steps: &steps,
            diagnostics: &[],
            artifacts: &[],
        };
        let mut out = String::new();
        timing_section(&mut out, &[phase], 3500);
        assert!(out.contains("TIMEOUT"), "{out}");
        assert!(!out.contains("FAIL"), "{out}");
    }
}
