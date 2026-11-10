//! AURORA-Core: a API que orquestra a toolchain do processador SAPHO.
//!
//! A AURORA substitui a lógica de orquestração da AURORA por uma biblioteca.
//! As interfaces (a CLI `aurora` hoje, uma GUI e um LSP depois) só chamam estas
//! funções e mostram o resultado; nenhuma regra de negócio mora nelas.
//!
//! # O que a API faz
//!
//! | Ferramenta | Função | Resultado |
//! |---|---|---|
//! | arquivos e pastas | [`Project::create`], [`Project::add_processor`], [`Project::add_file`], [`Project::set_top_level`], [`Project::set_testbench`], [`Project::move_path`], [`Processor::write_input`] | `.spf` e diretórios no formato da AURORA |
//! | YANC | [`build`] | Verilog, memórias e testbench de um processador |
//! | Icarus Verilog | [`simulate`], [`simulate_project`], [`check`], [`hierarchy`] | [`SimulationResult`], [`CheckResult`], [`HierarchyResult`] |
//! | cocotb (com o Icarus ou o Verilator) | [`simulate_project`] com um testbench `.py` ([`cocotb`]) | [`SimulationResult`] com [`TestReport`] |
//! | Verilator | [`simulate`], [`simulate_project`]; sem onda, a simulação rápida ([`SimulationOptions::fast`]) | [`SimulationResult`] |
//! | Yosys | [`synthesize`] | [`SynthesisResult`] (netlist JSON) |
//! | Yosys + Graphviz | [`render_schematic`] | [`SchematicResult`] (SVG) |
//! | surfer-aurora | [`open_waveform`] | [`RunningProcess`] |
//! | placas FPGA | [`fpga::boards`], [`fpga::prepare`] | as placas, o `fpga.json` conferido e o topo da placa |
//! | Quartus Prime (do sistema) | [`fpga::build()`] | [`fpga::FpgaBuildResult`]: o `.sof`, o `.rbf` e o `.svf`, os recursos e o tempo |
//!
//! As placas FPGA são um recurso experimental ([`experimental`]), desligado
//! até o usuário ligar: a CLI e o Studio só as oferecem ligadas.
//!
//! Todas as ferramentas saem do bundle versionado instalado com a AURORA
//! ([`Toolchain`]); nenhuma do `PATH`. As exceções são o compilador C++ do
//! Verilator no Linux e no macOS, que vem do sistema ([`SystemCompiler`]; no
//! Windows ele também vem no bundle), e o Quartus Prime, para as placas
//! Intel ([`fpga::Quartus`]).
//!
//! # Fluxo típico
//!
//! ```no_run
//! use aurora_core::*;
//!
//! // 1. O bundle instalado com a AURORA. Nada vem do PATH.
//! let exe = camino::Utf8PathBuf::try_from(std::env::current_exe()?)?;
//! let toolchain = Toolchain::locate(&exe)?;
//!
//! // 2. O projeto: um .spf da AURORA.
//! let project = Project::open("/home/eu/projetos/soma")?;
//! let soma = project.require_processor("soma")?;
//!
//! // 3. Compilar. Erro do usuário não é Err: vem nos diagnósticos. O
//! //    Control cancela e mostra a saída ao vivo; o padrão não faz nada.
//! let control = Control::default();
//! let built = build(&toolchain, soma, &BuildOptions::default(), &control)?;
//! if let Some(e) = built.first_error() {
//!     eprintln!("{:?}:{:?}: {}", e.file, e.line, e.message);
//!     return Ok(());
//! }
//!
//! // 4. Simular, desenhar, abrir a onda.
//! let options = SimulationOptions::new(Simulator::Icarus);
//! let sim = simulate(&toolchain, soma, &options, &control)?;
//! let target = DesignTarget::Processor("soma".into());
//! let synth = synthesize(&toolchain, &project, &target, &control)?;
//! if let Some(netlist) = &synth.netlist {
//!     let schematic = SchematicOptions::default();
//!     render_schematic(&toolchain, netlist, "soma", &schematic, &control)?;
//! }
//! if let Some(wave) = &sim.waveform {
//!     open_waveform(&toolchain, &wave.path, &ViewerOptions::default())?;
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Dois tipos de falha
//!
//! Toda operação devolve `Result<Resultado, AuroraError>`:
//!
//! - `Err(`[`AuroraError`]`)`: a AURORA não conseguiu rodar (toolchain, projeto,
//!   I/O). É problema do ambiente.
//! - `Ok(resultado)` com [`Status`] diferente de `Succeeded`: as ferramentas
//!   rodaram e recusaram a entrada (ou foram canceladas, ou passaram do
//!   prazo). É feedback para o usuário, com arquivo e linha em
//!   [`Diagnostic`].
//!
//! # Anatomia de um resultado
//!
//! Todos os resultados de operação têm `status`, `failed_step`, `steps`
//! ([`StepReport`]: comando exato, CWD, ambiente, stdout, stderr, duração) e
//! `diagnostics`; os que produzem arquivos têm `artifacts` ([`Artifact`], com
//! `fresh` dizendo se o arquivo foi escrito agora). Todos implementam
//! `Serialize`: o JSON da CLI (`aurora ... --json`) é exatamente esse.
//!
//! # Disco
//!
//! ```text
//! <raiz>/
//!   <projeto>.spf
//!   <processador>/            fonte .cmm ou .cpp, input_<n>.txt, arquivos de arrays (usuário)
//!   <processador>/<processador>/  tudo o que o YANC gera (.asm, .v, .mif, testbench,
//!                             tabelas) e o que a simulação grava (output_<n>.txt, a onda)
//!   .aurora8/Temp/<processador>/ o que só a AURORA usa: .vvp, modelo do Verilator, testbench instrumentado
//!   .aurora8/Temp/               simulação do projeto, síntese (synth/<topo>/), hierarquia (hierarchy/)
//!   .aurora8/fpga/<placa>/       o projeto do Quartus e os arquivos de gravação (fpga::build)
//! ```
//!
//! # Garantias
//!
//! - **Sem I/O de console.** O Core nunca escreve no stdout ou no stderr, não
//!   chama `exit` e não entra em pânico em caminho de erro esperado. O log
//!   sai por [`tracing`](https://docs.rs/tracing); quem configura o
//!   subscriber é o cliente.
//! - **Só o bundle.** Todo programa sai do bundle, num caminho fixo por
//!   plataforma; nada do `PATH`, nada configurável. As exceções, o
//!   compilador do Verilator e o Quartus, vêm de pastas fixas ou declaradas,
//!   nunca do `PATH`. O processo filho recebe um ambiente vazio, mais o
//!   mínimo que cada ferramenta exige (ver `docs/BUNDLE.md`).
//! - **Nunca sobrescreve código.** Criar processador ou arquivo com conteúdo
//!   recusa se o arquivo já existir. Artefatos gerados (a pasta de saída de
//!   cada processador, `Temp/`) são reescritos a cada operação.
//! - **Compatível com a AURORA.** O `.spf` é lido e gravado no formato dela,
//!   preservando o que a AURORA não entende.
//! - **Síncrono, cancelável e ao vivo.** Cada operação bloqueia até as
//!   ferramentas terminarem (menos [`open_waveform`]). Numa GUI, chame numa
//!   thread; os tipos são `Send`. O [`Control`] passado a cada operação
//!   cancela ([`CancelToken`]) e avisa cada linha que as ferramentas
//!   escrevem enquanto rodam ([`Event`]). Cancelar encerra o processo e tudo
//!   o que ele iniciou: nenhum processo da AURORA sobra depois da operação. No
//!   Windows a árvore fica num Job Object ([`ProcessJob`]) e termina também
//!   quando o processo da AURORA morre, e nenhuma ferramenta abre janela de
//!   console ([`hide_console`]).
//!
//! # Plataformas
//!
//! Linux x64, macOS arm64 e Windows x64, um bundle para cada. Conferido no
//! Linux com o bundle 2026.09.29; no Windows (pelo Wine) os processos, o build
//! e a regressão do YANC; no macOS, a compilação. O que muda por sistema está
//! em `docs/API.md`, seção 9.1.

#![deny(clippy::print_stdout, clippy::print_stderr)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod build;
pub mod cocotb;
mod control;
pub mod delivery;
mod diagnostics;
mod error;
pub mod experimental;
mod files;
pub mod flows;
pub mod fpga;
mod ghdl;
mod hdl;
mod hierarchy;
pub mod history;
mod layout;
pub mod locate;
mod paths;
mod pipeline;
mod process;
mod project;
pub mod rtl_viewer;
mod simulate;
mod source;
mod spf;
mod stats;
mod status;
mod synth;
mod toolchain;
pub mod verilog;
pub mod vhdl;
pub mod virtual_board;
mod wave;
mod wave_layout;
mod wave_signals;
mod yanc;

pub use build::{BuildOptions, BuildResult, OnFailure, build, build_processors, build_steps};
pub use cocotb::{TestCase, TestReport, TestStatus};
pub use control::{CancelToken, Control, Event, Stream};
pub use diagnostics::{Diagnostic, Severity};
pub use error::{AuroraError, ErrorInfo, Result};
pub use files::{
    AddedFile, FileRole, ListPosition, MovedPath, ProjectFile, RemoveOutcome, read_data_file,
};
pub use hdl::SourceLanguage;
pub use hierarchy::{Elaboration, HierarchyOptions, HierarchyResult, ModuleInstance, hierarchy};
pub use layout::{ConvertedPath, LayoutConversion, RemovedNameLine, SkippedPath};
pub use paths::YANC_PATH_LIMIT;
pub use pipeline::{Artifact, ArtifactKind, Status, Step, StepReport};
pub use process::{
    Invocation, ProcessJob, RunningProcess, Termination, hide_console, open_with_system,
};
pub use project::{
    DEFAULT_CLOCKS, DEFAULT_FREQUENCY_MHZ, IssueKind, Language, MAX_CLOCKS, MAX_FREQUENCY_MHZ,
    MAX_PORTS, MAX_PROCESSOR_NAME, MAX_PROJECT_NAME, NewProcessor, NewProcessorFields, Processor,
    ProcessorConfig, Project, ProjectIssue, RemovedProcessor, validate_new_processor_name,
    validate_processor_name, validate_project_name,
};
pub use simulate::{
    SimulationOptions, SimulationResult, Simulator, Waveform, WaveformFormat, missing_inputs,
    simulate, simulate_project, waveform_path,
};
pub use stats::{CellUsage, SynthesisMetric, SynthesisStatistics};
pub use status::{GeneratedFiles, PortFile, ProcessorStatus, ProjectStatus};
pub use synth::{
    CheckOptions, CheckResult, DesignTarget, SCHEMATIC_TIMEOUT, SchematicOptions, SchematicResult,
    SynthesisResult, check, render_schematic, synthesize,
};
pub use toolchain::{
    BUNDLE_SCHEMA, BundleComponent, BundleManifest, COMPONENTS_DIR, FileMismatch, MANIFEST_FILE,
    Platform, SystemCompiler, Tool, Toolchain, component,
};
pub use wave::{ViewerOptions, open_waveform};
pub use wave_layout::{
    MappingTranslator, PreparedLayout, SavedLayout, WaveLayout, WaveProcessor, prepare_wave_layout,
    reset_saved_layout, saved_layout, wave_layout, wave_testbench_of,
};
pub use wave_signals::{
    PortDirection, ScopeKind, SignalKind, SignalScope, WAVE_DIR, WaveSignal, WaveSignals,
    layout_file, read_selection, selection_file, wave_signals, wave_testbench, write_selection,
};
