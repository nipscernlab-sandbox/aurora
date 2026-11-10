//! Os fluxos compostos: o que um comando da CLI ou um botão do Studio faz
//! de ponta a ponta, com as mesmas regras nos dois.
//!
//! Uma operação do core (`build`, `simulate`, `synthesize`) roda uma
//! ferramenta e devolve o resultado. Um fluxo é a composição que o usuário
//! vê como uma coisa só: compilar os processadores antes, parar no primeiro
//! que falhar, registrar no `.spf` o Verilog gerado, simular, ler as portas
//! de saída, gravar o relatório no histórico e abrir a onda. Até a 0.8 essa
//! composição existia duas vezes, na CLI e no Studio, e divergia; aqui ela
//! existe uma vez.
//!
//! O que é da interface vai como parâmetro ([`FlowOptions`]): o texto do
//! comando para o histórico, se a onda abre em janela ou se o cliente a
//! mostra por conta própria, quem abre o RTL viewer. O que o fluxo diz
//! enquanto roda ([`Progress`]) é tudo que a CLI imprime no meio de um
//! comando, na ordem em que acontece; o Studio só repassa a fase e cada
//! build à interface, pelo [`delivery`](crate::delivery).
//!
//! `Err` é a AURORA não conseguir rodar (bundle, projeto, E/S). Um fluxo que
//! rodou e falhou é `Ok` com [`FlowOutcome::status`] diferente de
//! `Succeeded`, e com o que cada fase deixou.

use std::time::SystemTime;

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::fpga::{FpgaBuildResult, FpgaProgramResult};
use crate::{
    AddedFile, AuroraError, BuildResult, CheckResult, ErrorInfo, HierarchyResult, SchematicResult,
    SimulationResult, Simulator, Status, SynthesisResult, WaveProcessor,
};

mod run;
mod wave;

pub use run::{run, run_with};
pub use wave::{choose_signals, open_wave, open_wave_with, wave_file};

/// O que o cliente pede. Os alvos são sempre explícitos: a CLI resolve o
/// processador da pasta atual antes de montar o pedido.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "flow", rename_all = "snake_case")]
pub enum FlowRequest {
    /// Compila os processadores pedidos (vazio: todos), mesmo que um falhe,
    /// para mostrar todos os erros. Registra no `.spf` o que gerou e grava
    /// relatório.
    Build {
        /// Os processadores; vazio é todos os do projeto.
        #[serde(default)]
        processors: Vec<String>,
    },
    /// Verifica o Verilog que está no disco, sem compilar os processadores.
    /// Grava relatório.
    Check {
        /// Só os módulos deste arquivo como raiz.
        #[serde(default)]
        #[schemars(with = "Option<String>")]
        file: Option<Utf8PathBuf>,
        /// Só este processador: o Verilog dele e o testbench que o YANC
        /// gerou, no lugar do projeto.
        #[serde(default)]
        processor: Option<String>,
        /// Acrescenta o `verilator --lint-only`.
        #[serde(default)]
        lint: bool,
    },
    /// A hierarquia do design e de cada testbench, elaborada pelo Icarus,
    /// do que está no disco. Não compila nem grava relatório.
    Hierarchy {
        /// Só este processador.
        #[serde(default)]
        processor: Option<String>,
    },
    /// Compila os processadores que faltam (parando no primeiro que falhar)
    /// e simula o processador ou o projeto. Grava relatório; abre a onda se
    /// pedido.
    Simulate {
        /// Simula este processador com o testbench do YANC; `None`, a
        /// simulação do projeto.
        #[serde(default)]
        processor: Option<String>,
        /// Troca o testbench da simulação do projeto antes de simular
        /// (grava o `.spf`).
        #[serde(default)]
        #[schemars(with = "Option<String>")]
        testbench: Option<Utf8PathBuf>,
        /// Icarus ou Verilator. Na simulação rápida, só vale para um
        /// testbench cocotb.
        simulator: Simulator,
        /// A simulação rápida: sem onda, com o testbench Verilog no
        /// Verilator.
        #[serde(default)]
        fast: bool,
        /// Prazo do passo `simulate`, em segundos.
        #[serde(default)]
        timeout_s: Option<u64>,
        /// Abrir a onda se a simulação a entregar, conforme
        /// [`FlowOptions::wave`]. A simulação rápida não tem onda.
        #[serde(default)]
        open_wave: bool,
    },
    /// Compila os processadores que faltam e sintetiza o processador ou o
    /// topo do projeto com o Yosys. Grava relatório.
    Synthesize {
        /// Sintetiza só este processador; `None`, o topo do projeto.
        #[serde(default)]
        processor: Option<String>,
        /// Desenha o esquemático (Yosys `show` e Graphviz) depois.
        #[serde(default)]
        schematic: bool,
        /// O módulo do esquemático; `None`, o topo da síntese.
        #[serde(default)]
        module: Option<String>,
        /// Grava o HTML do RTL viewer ao lado do netlist e o abre conforme
        /// [`FlowOptions::viewer_opener`].
        #[serde(default)]
        open_viewer: bool,
    },
    /// Desenha outro módulo de um netlist que já existe, sem sintetizar de
    /// novo. Sem relatório.
    Schematic {
        /// O `hierarchy.json` da síntese.
        #[schemars(with = "String")]
        netlist: Utf8PathBuf,
        /// O módulo.
        module: String,
        /// Escrever a largura dos barramentos nas arestas.
        #[serde(default = "yes")]
        bus_widths: bool,
    },
    /// Compila os processadores e depois o projeto para a placa do
    /// `fpga.json`, pelo Quartus. Recurso experimental; sem relatório.
    FpgaBuild,
    /// Grava na placa o `.sof` da última compilação. Recurso experimental;
    /// sem relatório.
    FpgaProgram {
        /// O cabo; `None`, o primeiro que o Quartus lista.
        #[serde(default)]
        cable: Option<String>,
    },
    /// Corrige um exercício do `aurora learn` e grava se ele está resolvido;
    /// a correção vai em [`FlowOutcome::learn`]. Quem roda é o servidor do
    /// AURORA, que linka o aurora-learn: o [`run`] do core responde
    /// `unsupported`. Sem relatório.
    Learn {
        /// A pasta de exercícios (ou uma pasta dentro dela).
        #[schemars(with = "String")]
        root: Utf8PathBuf,
        /// O exercício.
        exercise: String,
    },
}

fn yes() -> bool {
    true
}

impl FlowRequest {
    /// O nome curto do fluxo, como vai em [`FlowOutcome::flow`].
    pub fn name(&self) -> &'static str {
        match self {
            FlowRequest::Build { .. } => "build",
            FlowRequest::Check { .. } => "check",
            FlowRequest::Hierarchy { .. } => "hierarchy",
            FlowRequest::Simulate { .. } => "simulate",
            FlowRequest::Synthesize { .. } => "synthesize",
            FlowRequest::Schematic { .. } => "schematic",
            FlowRequest::FpgaBuild => "fpga_build",
            FlowRequest::FpgaProgram { .. } => "fpga_program",
            FlowRequest::Learn { .. } => "learn",
        }
    }

    /// O recurso experimental de que o fluxo faz parte, se algum. O
    /// [`run`] confere ([`experimental::require`](crate::experimental::require))
    /// antes de começar; um cliente pode conferir antes, para nem reservar
    /// a operação.
    pub fn feature(&self) -> Option<crate::experimental::Feature> {
        match self {
            FlowRequest::FpgaBuild | FlowRequest::FpgaProgram { .. } => {
                Some(crate::experimental::Feature::Fpga)
            }
            _ => None,
        }
    }

    /// Os processadores que o pedido nomeia. Quem atende confere cada um no
    /// projeto antes de procurar o bundle: um nome errado é o erro do
    /// pedido, e não um bundle que falta.
    pub fn processors(&self) -> Vec<&str> {
        match self {
            FlowRequest::Build { processors } => processors.iter().map(String::as_str).collect(),
            FlowRequest::Check { processor, .. }
            | FlowRequest::Hierarchy { processor }
            | FlowRequest::Simulate { processor, .. }
            | FlowRequest::Synthesize { processor, .. } => {
                processor.as_deref().into_iter().collect()
            }
            FlowRequest::Schematic { .. }
            | FlowRequest::FpgaBuild
            | FlowRequest::FpgaProgram { .. }
            | FlowRequest::Learn { .. } => Vec::new(),
        }
    }

    /// O comando da CLI equivalente ao pedido, começando por `program`
    /// (`aurora`, ou `aurora-ide` quando quem rodou foi o Studio), para o
    /// relatório e para o console: `aurora sim -p soma --verilator`. Um
    /// argumento vazio ou com espaço vai entre aspas duplas, como na CLI.
    pub fn command_line(&self, program: &str) -> String {
        let mut parts = vec![program.to_owned()];
        match self {
            FlowRequest::Build { processors } => {
                parts.push("build".into());
                for p in processors {
                    parts.extend(["-p".into(), p.clone()]);
                }
            }
            FlowRequest::Check {
                file,
                processor,
                lint,
            } => {
                parts.push("check".into());
                parts.extend(file.iter().map(|f| quoted(f.as_str())));
                if let Some(p) = processor {
                    parts.extend(["-p".into(), p.clone()]);
                }
                if *lint {
                    parts.push("--lint".into());
                }
            }
            FlowRequest::Hierarchy { processor } => {
                parts.push("hierarchy".into());
                if let Some(p) = processor {
                    parts.extend(["-p".into(), p.clone()]);
                }
            }
            FlowRequest::Simulate {
                processor,
                testbench,
                simulator,
                fast,
                timeout_s,
                open_wave,
            } => {
                parts.push("sim".into());
                parts.extend(testbench.iter().map(|t| quoted(t.as_str())));
                if let Some(p) = processor {
                    parts.extend(["-p".into(), p.clone()]);
                }
                if *simulator == Simulator::Verilator {
                    parts.push("--verilator".into());
                }
                if *fast {
                    parts.push("--fast".into());
                }
                if let Some(t) = timeout_s {
                    parts.extend(["--timeout".into(), t.to_string()]);
                }
                // O `--open` e o `--fast` não andam juntos na CLI.
                if *open_wave && !*fast {
                    parts.push("--open".into());
                }
            }
            FlowRequest::Synthesize {
                processor,
                schematic,
                module,
                open_viewer,
            } => {
                parts.push("synth".into());
                if let Some(p) = processor {
                    parts.extend(["-p".into(), p.clone()]);
                }
                if *schematic {
                    parts.push("--svg".into());
                }
                if let Some(m) = module {
                    parts.extend(["--module".into(), m.clone()]);
                }
                if *open_viewer {
                    parts.push("--open".into());
                }
            }
            FlowRequest::Schematic { module, .. } => {
                parts.extend(["schematic".into(), quoted(module)]);
            }
            FlowRequest::FpgaBuild => parts.extend(["fpga".into(), "build".into()]),
            FlowRequest::FpgaProgram { cable } => {
                parts.extend(["fpga".into(), "program".into()]);
                if let Some(cable) = cable {
                    parts.extend(["--cable".into(), quoted(cable)]);
                }
            }
            FlowRequest::Learn { exercise, .. } => {
                parts.extend(["learn".into(), "check".into(), quoted(exercise)]);
            }
        }
        parts.join(" ")
    }
}

/// `text` entre aspas duplas quando é vazio ou tem espaço, como a CLI grava
/// a linha de comando no relatório.
pub(crate) fn quoted(text: &str) -> String {
    if text.is_empty() || text.contains(char::is_whitespace) {
        format!("\"{text}\"")
    } else {
        text.to_owned()
    }
}

/// O que é da interface, não do fluxo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FlowOptions {
    /// O texto do comando para o histórico e para o console (`aurora sim -p
    /// soma`, `aurora-ide sim -p soma`). A CLI passa a linha digitada; um
    /// cliente sem linha de comando monta com [`FlowRequest::command_line`].
    pub command: String,
    /// Quando a operação começou, para o relatório: a CLI toma antes de
    /// abrir o projeto. `None`: quando o fluxo começa.
    #[serde(skip)]
    pub started: Option<SystemTime>,
    /// Como abrir a onda quando o pedido tem `open_wave`.
    #[serde(default)]
    pub wave: WavePolicy,
    /// Quem abre o RTL viewer do `synth --open`: `None` abre no navegador do
    /// sistema; `Some(nome)` só grava o HTML, para quem chamou abrir (o
    /// `AURORA_OPENER=vscode` do terminal da extensão do VS Code).
    #[serde(default)]
    pub viewer_opener: Option<String>,
    /// Quantos valores de cada porta de saída o resultado leva
    /// ([`FlowOutcome::outputs`]): `None`, todos, como a CLI grava no
    /// `sim.json`; `Some(n)`, só os `n` primeiros, e [`PortValues::count`]
    /// diz quantos a porta tem. A simulação rápida da AURORA IDE pede 0,
    /// porque não mostra os valores: um processador simulado por 25 milhões
    /// de clocks escrevia 4 milhões deles, 30 MiB de JSON.
    #[serde(default)]
    pub port_values_limit: Option<usize>,
}

impl FlowOptions {
    /// As opções de um cliente que abre a onda em janela e o RTL viewer no
    /// navegador, com o comando `command`, e recebe todos os valores das
    /// portas.
    pub fn new(command: impl Into<String>) -> Self {
        FlowOptions {
            command: command.into(),
            started: None,
            wave: WavePolicy::Window,
            viewer_opener: None,
            port_values_limit: None,
        }
    }
}

/// Como um fluxo abre a onda que a simulação entregou.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WavePolicy {
    /// Não abre: o cliente mostra a onda por conta própria (a aba do
    /// Studio). O caminho fica em [`SimulationResult::waveform`].
    None,
    /// Abre no surfer-aurora, em janela, com o layout dos processadores
    /// SAPHO ([`open_wave`]).
    #[default]
    Window,
}

/// A fase de um fluxo que começou. Os nomes são os que o Studio mostra;
/// `Learn` é a correção de um exercício do aurora-learn, que o Studio nomeia
/// com o mesmo tipo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Phase {
    /// Compilando processadores (YANC).
    Build,
    /// Verificando o Verilog.
    Check,
    /// Elaborando a hierarquia.
    Hierarchy,
    /// Simulando.
    Simulate,
    /// Sintetizando.
    Synthesize,
    /// Desenhando o esquemático.
    Schematic,
    /// Gravando e abrindo o RTL viewer.
    RtlViewer,
    /// Abrindo a onda.
    Wave,
    /// Corrigindo um exercício do `aurora learn`.
    Learn,
    /// Compilando para a placa (Quartus).
    Fpga,
    /// Gravando na placa (Quartus Programmer).
    Program,
}

/// O que o fluxo diz enquanto roda, além dos eventos do
/// [`Control`](crate::Control): tudo que a CLI imprime no meio de um
/// comando, na ordem em que acontece. Carrega referências ao que o fluxo
/// tem na mão; quem precisa guardar, clona. O [`FlowOutcome`] traz tudo
/// isso de novo no fim.
#[derive(Debug)]
pub enum Progress<'a> {
    /// Uma fase começou.
    Phase(Phase),
    /// Um processador terminou de compilar.
    Built(&'a BuildResult),
    /// O que [`Project::register_generated`](crate::Project::register_generated)
    /// pôs no `.spf` depois dos builds.
    Registered(&'a [AddedFile]),
    /// O registro no `.spf` falhou; o build vale do mesmo jeito.
    RegisterFailed(&'a AuroraError),
    /// A fase `what` não rodou porque um build falhou ou foi cancelado.
    NotRun {
        /// A fase que não rodou.
        what: Phase,
        /// Os builds feitos, com o que falhou.
        builds: &'a [BuildResult],
    },
    /// A verificação terminou.
    Checked(&'a CheckResult),
    /// A hierarquia foi elaborada.
    Elaborated {
        /// O resultado.
        result: &'a HierarchyResult,
        /// Os processadores com fonte e sem build, que ficaram de fora.
        not_built: &'a [String],
    },
    /// A simulação terminou.
    Simulated {
        /// O resultado.
        result: &'a SimulationResult,
        /// Os valores das portas de saída (só com processador e sucesso).
        outputs: &'a [PortValues],
    },
    /// A síntese terminou.
    Synthesized(&'a SynthesisResult),
    /// O esquemático foi desenhado.
    Drawn(&'a SchematicResult),
    /// A compilação para a placa terminou.
    FpgaBuilt(&'a FpgaBuildResult),
    /// A gravação na placa terminou.
    Programmed(&'a FpgaProgramResult),
    /// O relatório foi gravado no histórico (`run-000042`).
    Recorded(&'a str),
    /// O relatório não pôde ser gravado; o fluxo vale do mesmo jeito.
    ReportNotSaved(&'a AuroraError),
    /// A onda abriu no surfer-aurora.
    WaveOpened(&'a WaveOpened),
    /// O HTML do RTL viewer foi gravado (e aberto, sem `opener`).
    RtlViewer {
        /// O HTML.
        path: &'a Utf8Path,
        /// Quem o abre no lugar do navegador, se alguém.
        opener: Option<&'a str>,
    },
}

/// A função de progresso de um fluxo.
pub type ProgressFn<'a> = &'a (dyn Fn(Progress<'_>) + Sync);

/// O resultado de um fluxo. Os campos de uma fase que não rodou ficam `None`
/// ou vazios.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FlowOutcome {
    /// O fluxo ([`FlowRequest::name`]).
    pub flow: &'static str,
    /// O comando equivalente ([`FlowOptions::command`]).
    pub command: String,
    /// Como terminou: o pior entre as fases que rodaram. `Cancelled` se o
    /// cancelamento foi pedido; `TimedOut` se a simulação estourou o prazo;
    /// `Failed` se alguma fase falhou (inclusive um build de `Build`, que
    /// compila todos); senão `Succeeded`.
    pub status: Status,
    /// `status == Succeeded`.
    pub succeeded: bool,
    /// Os builds feitos antes (ou o próprio build).
    pub builds: Vec<BuildResult>,
    /// O que esses builds geraram e entrou no `.spf` agora.
    pub registered: Vec<AddedFile>,
    /// O resultado da verificação.
    pub check: Option<CheckResult>,
    /// Com `lint`, o componente do Verilator que falta quando nenhum passo
    /// dele rodou: para a interface sugerir a instalação.
    pub lint_missing: Option<String>,
    /// A hierarquia.
    pub hierarchy: Option<HierarchyResult>,
    /// Na hierarquia sem processador: os processadores com fonte e sem
    /// build, que ficaram de fora.
    pub not_built: Vec<String>,
    /// O resultado da simulação.
    pub simulation: Option<SimulationResult>,
    /// Os valores de cada porta de saída do processador simulado (só com
    /// processador e sucesso), até [`FlowOptions::port_values_limit`] por
    /// porta.
    pub outputs: Vec<PortValues>,
    /// O resultado da síntese.
    pub synthesis: Option<SynthesisResult>,
    /// O resultado do esquemático.
    pub schematic: Option<SchematicResult>,
    /// Por que o esquemático não foi desenhado depois da síntese (o módulo
    /// pedido não está no netlist, `module_not_found`); a síntese vale.
    pub schematic_error: Option<ErrorInfo>,
    /// O HTML do RTL viewer, gravado ao lado do netlist.
    #[schemars(with = "Option<String>")]
    pub rtl_viewer: Option<Utf8PathBuf>,
    /// Por que o RTL viewer não saiu (a síntese vale).
    pub rtl_viewer_error: Option<ErrorInfo>,
    /// A onda aberta no surfer-aurora.
    pub wave: Option<WaveOpened>,
    /// Por que a onda não abriu (a simulação vale).
    pub wave_error: Option<ErrorInfo>,
    /// O relatório gravado no histórico (`run-000042`).
    pub report: Option<String>,
    /// Por que o relatório não foi gravado (o fluxo não falha por isso).
    pub report_error: Option<String>,
    /// A compilação para a placa.
    pub fpga: Option<FpgaBuildResult>,
    /// A gravação na placa.
    pub fpga_program: Option<FpgaProgramResult>,
    /// A correção de um exercício (`learn`): o `Grade` do aurora-learn, em
    /// JSON (o core não conhece o tipo).
    pub learn: Option<serde_json::Value>,
}

impl FlowOutcome {
    /// Um resultado vazio, bem-sucedido, do fluxo `flow` com o comando
    /// `command`: o ponto de partida de [`run`], e de um cliente que compõe
    /// um fluxo próprio sobre este (o `learn` do Studio).
    pub fn new(flow: &'static str, command: &str) -> Self {
        FlowOutcome {
            flow,
            command: command.to_owned(),
            status: Status::Succeeded,
            succeeded: true,
            builds: Vec::new(),
            registered: Vec::new(),
            check: None,
            lint_missing: None,
            hierarchy: None,
            not_built: Vec::new(),
            simulation: None,
            outputs: Vec::new(),
            synthesis: None,
            schematic: None,
            schematic_error: None,
            rtl_viewer: None,
            rtl_viewer_error: None,
            wave: None,
            wave_error: None,
            report: None,
            report_error: None,
            fpga: None,
            fpga_program: None,
            learn: None,
        }
    }

    /// Rebaixa o status para `status` se ele for pior que o atual
    /// (`Succeeded` < `Failed`, `Crashed`, `Incomplete` < `TimedOut` <
    /// `Cancelled`), e acerta `succeeded`.
    pub fn worsen(&mut self, status: Status) {
        if rank(status) > rank(self.status) {
            self.status = status;
        }
        self.succeeded = self.status == Status::Succeeded;
    }
}

fn rank(status: Status) -> u8 {
    match status {
        Status::Succeeded => 0,
        Status::Failed | Status::Crashed | Status::Incomplete => 1,
        Status::TimedOut => 2,
        Status::Cancelled => 3,
    }
}

/// Os valores de uma porta de saída de um processador simulado.
///
/// No schema chama-se `FlowPortValues`: o `PortValues` do relatório `sim`
/// da CLI tem o erro em texto, e os dois convivem no mesmo conjunto de
/// tipos gerados.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(rename = "FlowPortValues")]
pub struct PortValues {
    /// O número da porta.
    pub port: u32,
    /// `<proc>/<proc>/output_<porta>.txt`, na pasta de saída.
    #[schemars(with = "String")]
    pub path: Utf8PathBuf,
    /// Os valores, na ordem em que saíram: todos, ou os primeiros
    /// [`FlowOptions::port_values_limit`]. Vazio quando a porta não pôde ser
    /// lida.
    pub values: Vec<i64>,
    /// Quantos valores a porta tem no arquivo, mesmo quando `values` leva
    /// só os primeiros. 0 quando a porta não pôde ser lida.
    pub count: usize,
    /// Por que a porta não pôde ser lida: uma linha que não é inteiro, como
    /// o `x` que o simulador escreve numa divisão por zero. `None` quando
    /// leu.
    pub error: Option<ErrorInfo>,
}

/// A onda aberta no surfer-aurora, que continua aberto depois.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct WaveOpened {
    /// O arquivo da onda.
    #[schemars(with = "String")]
    pub waveform: Utf8PathBuf,
    /// O PID do surfer-aurora.
    pub pid: u32,
    /// Onde vai o stdout e o stderr do surfer-aurora.
    #[schemars(with = "String")]
    pub log: Utf8PathBuf,
    /// O estado do Surfer com que a onda abriu (`.surf.ron` ou `.sucl`): o
    /// layout salvo no projeto, o gerado, ou o de um exercício; `None` sem
    /// layout.
    #[schemars(with = "Option<String>")]
    pub layout: Option<Utf8PathBuf>,
    /// O layout salvo no projeto (`wave/<testbench>.surf.ron`), onde o
    /// Surfer salva (Ctrl+S); `None` numa onda fora de projeto ou sem layout
    /// gerado.
    pub saved_layout: Option<SavedLayoutInfo>,
    /// Os processadores do layout gerado.
    pub processors: Vec<WaveProcessor>,
    /// A escolha de sinais que o layout gerado seguiu (`wave/<testbench>.json`);
    /// vazia sem escolha ou sem layout gerado.
    pub selection: Vec<String>,
    /// Os processadores compilados de novo depois desta simulação: o PC e a
    /// linha aparecem como números.
    pub outdated: Vec<String>,
    /// O layout gerado falhou e a onda abriu sem ele: o motivo.
    pub layout_error: Option<String>,
    /// Com [`LayoutPolicy::Reset`]: o testbench cujo layout salvo foi
    /// apagado antes de abrir.
    pub reset_deleted: Option<String>,
}

/// O layout salvo no projeto para a onda aberta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct SavedLayoutInfo {
    /// O arquivo.
    #[schemars(with = "String")]
    pub path: Utf8PathBuf,
    /// O usuário salvou depois que a AURORA o gerou: a AURORA não o refaz.
    pub customized: bool,
}

/// Qual onda abrir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WaveSource {
    /// Um arquivo VCD, FST ou GHW.
    File {
        /// O arquivo.
        #[schemars(with = "String")]
        path: Utf8PathBuf,
    },
    /// A onda da simulação de um processador do projeto, ou, sem
    /// processador, a da simulação do projeto
    /// ([`waveform_path`](crate::waveform_path)).
    Project {
        /// O processador; `None`, o projeto.
        #[serde(default)]
        processor: Option<String>,
    },
}

/// O que fazer com o layout dos processadores SAPHO ao abrir a onda.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LayoutPolicy {
    /// O layout gerado (ou o salvo no projeto, se o usuário salvou).
    #[default]
    Generated,
    /// Nenhum: a onda crua.
    None,
    /// Apaga o layout salvo no projeto e abre com o gerado.
    Reset,
    /// Um arquivo de estado ou de comandos do Surfer (o layout de um
    /// exercício do aurora-learn).
    File {
        /// O arquivo (`.surf.ron` ou `.sucl`).
        #[schemars(with = "String")]
        path: Utf8PathBuf,
    },
}

/// O pedido de [`open_wave`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WaveRequest {
    /// Qual onda.
    pub source: WaveSource,
    /// O layout.
    #[serde(default)]
    pub layout: LayoutPolicy,
}

/// Uma mudança na escolha de sinais da onda do testbench do projeto
/// ([`choose_signals`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "paths", rename_all = "snake_case")]
pub enum WaveChoice {
    /// Todos os sinais de novo: a escolha fica vazia e o arquivo sai.
    All,
    /// Acrescenta sinais e escopos (caminhos como `tb.dut.q`).
    Add(Vec<String>),
    /// Tira sinais e escopos.
    Remove(Vec<String>),
    /// Troca a escolha inteira por estes sinais e escopos; vazia, como
    /// `All`.
    Set(Vec<String>),
}

/// A escolha de sinais depois da mudança.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct WaveChoiceOutcome {
    /// O módulo do testbench do projeto.
    pub testbench: String,
    /// O arquivo da escolha (`wave/<testbench>.json`); `None` quando ela
    /// ficou vazia e o arquivo saiu: a onda grava todos os sinais.
    #[schemars(with = "Option<String>")]
    pub file: Option<Utf8PathBuf>,
    /// A escolha; vazia, todos os sinais.
    pub selection: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_reproduces_the_cli_flags() {
        let request = FlowRequest::Simulate {
            processor: Some("soma".into()),
            testbench: None,
            simulator: Simulator::Verilator,
            fast: false,
            timeout_s: Some(30),
            open_wave: true,
        };
        assert_eq!(
            request.command_line("aurora-ide"),
            "aurora-ide sim -p soma --verilator --timeout 30 --open"
        );
        let fast = FlowRequest::Simulate {
            processor: None,
            testbench: Some("tb dir/t.v".into()),
            simulator: Simulator::Icarus,
            fast: true,
            timeout_s: None,
            open_wave: true,
        };
        assert_eq!(
            fast.command_line("aurora"),
            "aurora sim \"tb dir/t.v\" --fast"
        );
        assert_eq!(
            FlowRequest::Check {
                file: None,
                processor: Some("p".into()),
                lint: true
            }
            .command_line("aurora"),
            "aurora check -p p --lint"
        );
        assert_eq!(
            FlowRequest::Synthesize {
                processor: None,
                schematic: true,
                module: Some("ula".into()),
                open_viewer: true
            }
            .command_line("aurora"),
            "aurora synth --svg --module ula --open"
        );
        assert_eq!(
            FlowRequest::FpgaProgram {
                cable: Some("USB-Blaster [1-2]".into())
            }
            .command_line("aurora"),
            "aurora fpga program --cable \"USB-Blaster [1-2]\""
        );
        assert_eq!(
            FlowRequest::Build { processors: vec![] }.command_line("aurora"),
            "aurora build"
        );
    }

    #[test]
    fn requests_round_trip_through_json_with_defaults() {
        let request: FlowRequest =
            serde_json::from_str(r#"{"flow":"simulate","simulator":"icarus"}"#).unwrap();
        assert_eq!(
            request,
            FlowRequest::Simulate {
                processor: None,
                testbench: None,
                simulator: Simulator::Icarus,
                fast: false,
                timeout_s: None,
                open_wave: false,
            }
        );
        let request: FlowRequest = serde_json::from_str(r#"{"flow":"fpga_build"}"#).unwrap();
        assert_eq!(request, FlowRequest::FpgaBuild);
        assert_eq!(request.feature(), Some(crate::experimental::Feature::Fpga));
        let schematic: FlowRequest =
            serde_json::from_str(r#"{"flow":"schematic","netlist":"/n.json","module":"m"}"#)
                .unwrap();
        assert!(matches!(
            schematic,
            FlowRequest::Schematic {
                bus_widths: true,
                ..
            }
        ));
        let json = serde_json::to_value(&FlowRequest::Hierarchy { processor: None }).unwrap();
        assert_eq!(json["flow"], "hierarchy");
    }

    #[test]
    fn the_outcome_status_only_gets_worse() {
        let request = FlowRequest::Build { processors: vec![] };
        let mut outcome = FlowOutcome::new(request.name(), "aurora build");
        assert!(outcome.succeeded);
        outcome.worsen(Status::Failed);
        assert_eq!(outcome.status, Status::Failed);
        assert!(!outcome.succeeded);
        outcome.worsen(Status::Succeeded);
        assert_eq!(outcome.status, Status::Failed);
        outcome.worsen(Status::Cancelled);
        outcome.worsen(Status::TimedOut);
        assert_eq!(outcome.status, Status::Cancelled);
    }
}
