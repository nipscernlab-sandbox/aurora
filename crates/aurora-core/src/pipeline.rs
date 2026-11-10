//! Peças comuns a toda operação que encadeia ferramentas (build, simulação,
//! síntese, esquemático, compilação para a placa): o relatório de cada
//! passo, o status final e o rastreio de artefatos.
//!
//! Todo resultado de operação ([`BuildResult`](crate::BuildResult),
//! [`SimulationResult`](crate::SimulationResult), ...) tem a mesma espinha:
//!
//! | Campo | Conteúdo |
//! |---|---|
//! | `status` | [`Status`]: como a operação terminou |
//! | `failed_step` | o [`Step`] que falhou ou quebrou, se algum |
//! | `steps` | um [`StepReport`] por programa executado, na ordem |
//! | `diagnostics` | as mensagens de todos os passos, já interpretadas |
//! | `artifacts` | os [`Artifact`]s esperados, com `fresh` dizendo se foram gerados agora |
//!
//! Os passos rodam em sequência e param no primeiro que não terminar com
//! código 0: se o `cmmcomp` falhar, `steps` tem um elemento só. Também param
//! quando o [`Control`] da operação pede o cancelamento.

use std::time::{Duration, SystemTime};

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::control::{Control, Event, Stream};
use crate::diagnostics::{self, Diagnostic, Severity};
use crate::error::Result;
use crate::process::{self, Invocation, Termination, Watch};
use crate::toolchain::Tool;

/// Um passo de uma operação. Em JSON, em `snake_case` (`"pre_assemble"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Step {
    /// `cpppp`, do YANC até a 6.x, nos relatórios antigos (o `cppcomp` da
    /// 7.1 pré-processa).
    Preprocess,
    /// `cmmcomp` ou `cppcomp`: fonte para assembly.
    Compile,
    /// `appcomp`, do YANC até a 6.x, nos relatórios antigos (o `asmcomp` da
    /// 7.1 faz as duas passadas).
    PreAssemble,
    /// `asmcomp`: assembly para Verilog e memórias.
    Assemble,
    /// `iverilog -t null`: só confere se o Verilog elabora.
    CheckSyntax,
    /// `verilator --lint-only`: a verificação mais rigorosa (larguras, sinais
    /// sem uso), com `aurora check --lint`.
    Lint,
    /// `ghdl -i`: lê os arquivos VHDL para a biblioteca de trabalho; os
    /// erros de sintaxe aparecem aqui.
    Analyze,
    /// `iverilog`: Verilog para o `.vvp` do Icarus. No VHDL, `ghdl -m`: a
    /// análise de cada unidade na ordem das dependências e a elaboração.
    Elaborate,
    /// `verilator --binary`: Verilog para um executável C++.
    Verilate,
    /// `vvp`, o executável do Verilator ou o GHDL rodando o testbench.
    Simulate,
    /// `yosys`: Verilog para netlist JSON. Numa placa, o `quartus_map`
    /// (Analysis & Synthesis do Quartus).
    Synthesize,
    /// `yosys show`: netlist para o grafo do esquemático (`.dot`).
    Graph,
    /// `dot`: grafo do esquemático para SVG.
    Render,
    /// `quartus_fit`: posiciona e roteia o design na FPGA da placa (Fitter).
    Fit,
    /// `quartus_asm`: os arquivos de gravação (`.sof`, `.rbf`, `.svf`), no
    /// Assembler do Quartus.
    Bitstream,
    /// `quartus_sta`: a análise de tempo (Timing Analyzer), com a folga e a
    /// Fmax de cada clock.
    Timing,
    /// `quartus_pgm`: grava o `.sof` na FPGA da placa pelo cabo JTAG.
    Program,
}

/// O que aconteceu num passo: o comando exato, como terminou e tudo o que
/// escreveu. É o registro de auditoria da operação; os `diagnostics` do
/// resultado são a versão interpretada de `stdout` e `stderr`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct StepReport {
    /// Qual passo.
    pub step: Step,
    /// A ferramenta executada. Para o executável que o Verilator gera, é
    /// [`Tool::Verilator`].
    pub tool: Tool,
    /// O que foi executado, com CWD e ambiente.
    pub command: Invocation,
    /// Como o processo terminou.
    pub termination: Termination,
    /// Tudo que o processo escreveu no stdout, decodificado como UTF-8 (bytes
    /// inválidos viram `U+FFFD`). Numa simulação, inclui o `$display` do
    /// testbench.
    pub stdout: String,
    /// Tudo que o processo escreveu no stderr.
    pub stderr: String,
    /// Duração, do início do processo até o fim, em milissegundos.
    pub duration_ms: u64,
}

/// Como uma operação terminou. Em JSON, em `snake_case` (`"succeeded"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Status {
    /// Todos os passos terminaram com código 0 e os artefatos foram gerados.
    Succeeded,
    /// Uma ferramenta recusou a entrada (código de saída diferente de 0).
    Failed,
    /// Uma ferramenta morreu (sinal no Unix, exceção no Windows). É defeito da
    /// ferramenta, não do código do usuário.
    Crashed,
    /// As ferramentas terminaram bem, mas faltou algum artefato obrigatório.
    Incomplete,
    /// O cancelamento foi pedido ([`CancelToken`](crate::CancelToken)): o
    /// passo que rodava foi encerrado e os seguintes não rodaram.
    Cancelled,
    /// Um passo passou do prazo e a AURORA o encerrou. Numa simulação, é o
    /// testbench que não chega ao `$finish`.
    TimedOut,
}

/// O papel de um arquivo produzido por uma operação. Em JSON, em
/// `snake_case` (`"data_memory"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ArtifactKind {
    /// `<nome>/<nome>/<nome>.asm`, na pasta de saída do processador.
    Assembly,
    /// `<nome>/<nome>/<nome>.v`, o processador sintetizável.
    Verilog,
    /// `<nome>/<nome>/<nome>_data.mif`
    DataMemory,
    /// `<nome>/<nome>/<nome>_inst.mif`
    InstructionMemory,
    /// `<nome>/<nome>/<nome>_tb.v`: o testbench que o `asmcomp` gera, que
    /// roda na pasta de saída.
    Testbench,
    /// `<temp>/pp.cpp`, saída do `cpppp` do YANC até a 6.x, nos relatórios
    /// antigos.
    PreprocessedSource,
    /// `<nome>/<nome>/pc_<nome>_mem.txt`, liga endereço de instrução a linha
    /// do fonte.
    ProgramCounterMap,
    /// `<nome>/<nome>/src_lines.txt`: o texto de cada linha do fonte.
    SourceTranslation,
    /// `<nome>/<nome>/trad_opcode.txt`
    OpcodeTranslation,
    /// `<nome>/<nome>/prog_log.txt`: as variáveis, o número de instruções
    /// e a pasta do fonte.
    CompilerLog,
    /// `<nome>/<nome>/app_log.txt`
    PreAssemblerLog,
    /// O `.vvp` do Icarus.
    IcarusImage,
    /// O executável que o Verilator gerou.
    VerilatedModel,
    /// Onda da simulação (VCD ou FST).
    Waveform,
    /// `<nome>/<nome>/output_<n>.txt`, escrito pelo testbench.
    SimulationOutput,
    /// Netlist JSON do Yosys.
    Netlist,
    /// Grafo do esquemático (`.dot`), gerado pelo Yosys.
    SchematicGraph,
    /// Esquemático SVG, desenhado pelo `dot`.
    Schematic,
    /// As estatísticas do `stat -json` do Yosys (`stat.json`).
    SynthesisStatistics,
    /// O topo da placa que a AURORA gera (`aurora_board_top.v`).
    BoardTop,
    /// As configurações do projeto do Quartus (`.qsf`): a FPGA, os fontes e
    /// os pinos.
    QuartusProject,
    /// O arquivo de gravação do Quartus (`.sof`, SRAM Object File), que o
    /// `quartus_pgm` grava.
    SramObject,
    /// O bitstream cru, sem compressão (`.rbf`, Raw Binary File), que o
    /// openFPGALoader grava.
    RawBinary,
    /// A gravação como vetores JTAG (`.svf`, Serial Vector Format), que o
    /// openFPGALoader também grava.
    SerialVectorFormat,
}

/// Milissegundos desde `started`, para o `duration_ms` dos resultados.
pub(crate) fn elapsed_ms(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Um arquivo que a operação deveria produzir.
///
/// `fresh` é calculado comparando o horário de modificação antes e depois da
/// operação. Um `.v` que sobrou de um build anterior aparece com
/// `fresh = false`, e não como sucesso. Artefatos obrigatórios aparecem
/// sempre; intermediários, só se existirem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Artifact {
    /// O papel do arquivo.
    pub kind: ArtifactKind,
    /// Caminho absoluto.
    #[schemars(with = "String")]
    pub path: Utf8PathBuf,
    /// Sem ele a operação não está completa.
    pub required: bool,
    /// Existe e foi escrito por esta operação (não é sobra de uma anterior).
    /// O executável do Verilator, que o `make` só refaz quando o modelo
    /// muda, conta como escrito quando a compilação terminou bem: ele está
    /// em dia com a entrada.
    pub fresh: bool,
}

/// Um passo pronto para rodar.
pub(crate) struct PlannedStep {
    pub step: Step,
    pub tool: Tool,
    pub invocation: Invocation,
    /// Arquivo ao qual se referem as mensagens com linha e sem arquivo.
    pub source: Option<Utf8PathBuf>,
    /// Prazo do processo; `None`, sem prazo.
    pub timeout: Option<Duration>,
}

impl PlannedStep {
    pub fn new(step: Step, tool: Tool, invocation: Invocation) -> Self {
        PlannedStep {
            step,
            tool,
            invocation,
            source: None,
            timeout: None,
        }
    }

    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Executa passos em sequência, acumulando relatórios e diagnósticos, e para
/// no primeiro que não terminar com código 0 ou quando o cancelamento for
/// pedido.
pub(crate) struct Runner<'a> {
    control: &'a Control,
    pub steps: Vec<StepReport>,
    pub diagnostics: Vec<Diagnostic>,
    pub status: Status,
    pub failed_step: Option<Step>,
}

impl<'a> Runner<'a> {
    pub fn new(control: &'a Control) -> Self {
        Runner {
            control,
            steps: Vec::new(),
            diagnostics: Vec::new(),
            status: Status::Succeeded,
            failed_step: None,
        }
    }

    /// Roda o passo e diz se ele terminou bem. Depois de uma falha ou de um
    /// cancelamento, não roda mais nada e devolve `false`.
    pub fn run(&mut self, planned: PlannedStep) -> Result<bool> {
        if self.status != Status::Succeeded {
            return Ok(false);
        }
        let control = self.control;
        if control.is_cancelled() {
            self.status = Status::Cancelled;
            self.failed_step = Some(planned.step);
            return Ok(false);
        }
        tracing::info!(step = ?planned.step, tool = %planned.tool, "Step");
        let (step, tool) = (planned.step, planned.tool);
        if control.has_sink() {
            control.emit(Event::StepStarted {
                step,
                tool,
                command: planned.invocation.clone(),
            });
        }
        let on_line = |stream: Stream, line: &str| {
            control.emit(Event::Output {
                step,
                tool,
                stream,
                line: line.to_owned(),
                diagnostic: diagnostics::is_message(tool, stream, line),
            });
        };
        let watch = Watch {
            cancel: Some(control.cancel_token()),
            timeout: planned.timeout,
            on_line: control
                .has_sink()
                .then_some(&on_line as process::OnLine<'_>),
        };
        let output = process::run(&planned.invocation, &watch)?;
        let duration_ms = u64::try_from(output.duration.as_millis()).unwrap_or(u64::MAX);
        control.emit(Event::StepFinished {
            step,
            tool,
            termination: output.termination,
            duration_ms,
        });
        let first_new = self.diagnostics.len();
        self.diagnostics.extend(diagnostics::parse(
            planned.tool,
            &output.stdout,
            &output.stderr,
            planned.source.as_deref(),
        ));
        resolve_relative_files(
            planned.tool,
            &planned.invocation.cwd,
            &mut self.diagnostics[first_new..],
        );
        // O iverilog sai com 0 depois de um erro que a AURORA promove (macro
        // indefinida): o passo falha do mesmo jeito.
        let promoted = planned.tool == Tool::Iverilog
            && self.diagnostics[first_new..]
                .iter()
                .any(|d| d.severity == Severity::Error);
        let termination = output.termination;
        self.steps.push(StepReport {
            step: planned.step,
            tool: planned.tool,
            command: planned.invocation,
            termination,
            stdout: output.stdout,
            stderr: output.stderr,
            duration_ms,
        });
        let status = match termination {
            Termination::Exited(0) if promoted => Status::Failed,
            Termination::Exited(0) => return Ok(true),
            Termination::Exited(_) => Status::Failed,
            Termination::Cancelled => Status::Cancelled,
            Termination::TimedOut => Status::TimedOut,
            Termination::Signaled(_) | Termination::Exception(_) | Termination::Unknown => {
                Status::Crashed
            }
        };
        self.status = status;
        self.failed_step = Some(planned.step);
        Ok(false)
    }

    /// Algumas ferramentas terminam com 0 mesmo tendo reportado erro. Marca como falha o passo
    /// mais recente se houver diagnóstico de erro dele.
    pub fn fail_on_error_diagnostics(&mut self, tool: Tool) {
        let has_error = self
            .diagnostics
            .iter()
            .any(|d| d.tool == tool && d.severity == Severity::Error);
        if self.status == Status::Succeeded && has_error {
            self.status = Status::Failed;
            self.failed_step = self.steps.last().map(|s| s.step);
        }
    }
}

/// Torna absoluto o arquivo de cada diagnóstico que a ferramenta escreveu
/// relativo ao CWD dela: o Icarus, o `vvp`, o Yosys e o Verilator escrevem
/// o caminho como o receberam, e no Windows um caminho com acento vai
/// relativo ao CWD ([`tool_path`](crate::process::tool_path)). O `vvp`
/// cita o que o `iverilog` recebeu, e os dois rodam no mesmo CWD. Só quando o
/// arquivo existe: o compilador C++ que o Verilator chama cita os fontes do
/// modelo relativos à pasta dele, e não ao CWD do Verilator.
fn resolve_relative_files(tool: Tool, cwd: &Utf8Path, diagnostics: &mut [Diagnostic]) {
    if !matches!(
        tool,
        Tool::Iverilog | Tool::Vvp | Tool::Yosys | Tool::Verilator | Tool::Dot | Tool::Ghdl
    ) {
        return;
    }
    for d in diagnostics {
        let Some(file) = &d.file else {
            continue;
        };
        if !file.is_relative() || file.as_str().starts_with('<') {
            continue;
        }
        let absolute = crate::paths::normalize(&cwd.join(file));
        if absolute.is_file() {
            d.file = Some(absolute);
        }
    }
}

/// Lembra o horário de modificação de cada artefato antes da operação, para
/// saber depois quais foram realmente reescritos. Sem isso, um `.v` que sobrou
/// de um build anterior pareceria sucesso.
pub(crate) struct ArtifactTracker {
    /// O papel, o caminho, se é obrigatório, se basta existir (um build
    /// incremental que o deixa como está) e o horário de antes.
    entries: Vec<(ArtifactKind, Utf8PathBuf, bool, bool, Option<SystemTime>)>,
}

impl ArtifactTracker {
    pub fn new() -> Self {
        ArtifactTracker {
            entries: Vec::new(),
        }
    }

    pub fn expect(&mut self, kind: ArtifactKind, path: impl Into<Utf8PathBuf>, required: bool) {
        let path = path.into();
        let before = mtime(&path);
        self.entries.push((kind, path, required, false, before));
    }

    /// O artefato em `path`, esperado antes, é de um build incremental (o
    /// `make` do Verilator) que terminou bem e só o refaz quando a entrada
    /// muda: existir basta.
    pub fn mark_current(&mut self, path: &Utf8Path) {
        for entry in self.entries.iter_mut().filter(|e| e.1 == path) {
            entry.3 = true;
        }
    }

    /// Os artefatos, com `fresh` calculado. Intermediário que não existe fica
    /// de fora; obrigatório que não existe aparece com `fresh = false`.
    pub fn finish(self) -> Vec<Artifact> {
        self.entries
            .into_iter()
            .filter_map(|(kind, path, required, current, before)| {
                let after = mtime(&path);
                let fresh = after.is_some() && (current || after != before);
                (required || after.is_some()).then_some(Artifact {
                    kind,
                    path,
                    required,
                    fresh,
                })
            })
            .collect()
    }
}

fn mtime(path: &Utf8Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// `Incomplete` quando todos os passos deram certo mas falta artefato.
pub(crate) fn final_status(status: Status, artifacts: &[Artifact]) -> Status {
    if status == Status::Succeeded && artifacts.iter().any(|a| a.required && !a.fresh) {
        Status::Incomplete
    } else {
        status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(tool: Tool, file: &str) -> Diagnostic {
        Diagnostic {
            tool,
            severity: Severity::Error,
            message: "x".into(),
            file: Some(Utf8PathBuf::from(file)),
            line: Some(1),
            column: None,
            raw: String::new(),
        }
    }

    #[test]
    fn relative_files_become_absolute_from_the_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = crate::paths::canonicalize(Utf8Path::from_path(dir.path()).unwrap()).unwrap();
        std::fs::create_dir_all(cwd.join("rtl")).unwrap();
        std::fs::write(cwd.join("rtl/a.v"), "").unwrap();
        let mut icarus = [
            found(Tool::Iverilog, "rtl/a.v"),
            found(Tool::Iverilog, "rtl/../rtl/a.v"),
            // Um arquivo que não existe a partir do CWD fica como veio.
            found(Tool::Iverilog, "rtl/b.v"),
        ];
        resolve_relative_files(Tool::Iverilog, &cwd, &mut icarus);
        let a = cwd.join("rtl").join("a.v");
        assert_eq!(icarus[0].file.as_deref(), Some(a.as_path()));
        assert_eq!(icarus[1].file.as_deref(), Some(a.as_path()));
        assert_eq!(icarus[2].file.as_deref(), Some(Utf8Path::new("rtl/b.v")));
        // O YANC cita os arquivos de dados a partir de outra pasta.
        let mut yanc = [found(Tool::Asmcomp, "rtl/a.v")];
        resolve_relative_files(Tool::Asmcomp, &cwd, &mut yanc);
        assert_eq!(yanc[0].file.as_deref(), Some(Utf8Path::new("rtl/a.v")));
    }
}
