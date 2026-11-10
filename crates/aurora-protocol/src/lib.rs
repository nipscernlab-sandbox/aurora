//! O contrato da AURORA: o que um cliente pede, o que recebe de volta e o que
//! ouve enquanto espera, em tipos com schema.
//!
//! Quem fala com a AURORA por um processo usa este contrato em JSON: o
//! `aurora serve` (JSON-RPC 2.0 por linhas sobre stdio) e o `aurora mcp`, para
//! um agente. Aqui moram só os tipos; quem despacha um [`Request`] é o
//! servidor, na CLI. O crate fica acima do `aurora-core`, do `aurora-installer`
//! e do `aurora-learn`, de quem vêm os tipos dos resultados (os de `update.*`
//! e `install.*` são do instalador, os de `learn.*` do aurora-learn), e
//! nenhum deles conhece o protocolo.
//!
//! Os nomes dos métodos têm um grupo e um ponto (`project.status`,
//! `flow.run`); os campos são os das operações do core. O schema sai de
//! [`request_schema`] e [`response_schema`], gerados dos tipos: o primeiro
//! na vista de quem desserializa (os campos com padrão são opcionais), o
//! segundo na de quem serializa.

#![deny(clippy::print_stdout, clippy::print_stderr)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use aurora_core::experimental::ExperimentalInfo;
use aurora_core::flows::{
    FlowOptions, FlowOutcome, FlowRequest, WaveChoice, WaveChoiceOutcome, WaveOpened, WaveRequest,
};
use aurora_core::fpga::{BitstreamState, Board, FpgaConfig, Prepared, Reconnect};
use aurora_core::history::{CleanOutcome, Cleanup, ReportView, RunComparison, RunSummary};
use aurora_core::locate::ToolchainInfo;
use aurora_core::verilog::ModuleInterface;
use aurora_core::virtual_board::{BoardInput, BoardLayout, BoardRun};
use aurora_core::{
    AddedFile, ErrorInfo, FileMismatch, LayoutConversion, ListPosition, MovedPath, NewProcessor,
    Processor, ProcessorConfig, ProjectFile, ProjectStatus, RemoveOutcome, RemovedProcessor,
    SavedLayout, WaveLayout, WaveSignals,
};
use aurora_git::github::{Account, DeviceCode, LoginPoll};
use aurora_git::{
    Branches, CommitFiles, CommitResult, DiscardResult, FileVersion, GitLog, GitStatus, InitResult,
    Revision, SyncResult,
};
use aurora_installer::add::InstallReport;
use aurora_installer::update::UpdateReport;
use aurora_learn::flows::LearnCheckOutcome;
use aurora_learn::{LearnTracks, WorkspaceSnapshot};

pub use aurora_core::delivery::Notification;

/// A versão do protocolo. Sobe quando um método muda de forma de um jeito
/// que um cliente antigo não entenderia; um método novo não a sobe. A 2 é a
/// da renomeação para AURORA: os campos `lace` viraram `aurora`.
pub const PROTOCOL_VERSION: u32 = 2;

fn yes() -> bool {
    true
}

/// Como um método aparece para um agente de IA, no `aurora mcp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agent {
    /// Não vira ferramenta.
    No,
    /// Vira ferramenta (o `flow.run`, uma por fluxo).
    Tool,
    /// Vira ferramenta só com `--allow-update`: muda a instalação da AURORA.
    AllowUpdate,
}

/// O que se declara de um método além do nome e do tipo do resultado, na
/// linha dele em `methods!`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MethodInfo {
    /// O nome em JSON (`project.status`).
    pub name: &'static str,
    /// Só lê (o disco, o bundle) e não roda ferramenta: um servidor pode
    /// atendê-lo em paralelo com um fluxo longo do mesmo projeto, sem fila.
    pub read_only: bool,
    /// Mexe na instalação da AURORA (instalar, atualizar): um servidor o
    /// enfileira sozinho, com a trava da instalação.
    pub installation: bool,
    /// Como aparece para um agente.
    pub agent: Agent,
}

/// A marcação `$want` está na lista de uma linha de `methods!`.
macro_rules! flag {
    ($want:ident;) => {
        false
    };
    (read; read $($rest:ident)*) => {
        true
    };
    (install; install $($rest:ident)*) => {
        true
    };
    (agent; agent $($rest:ident)*) => {
        true
    };
    (agent_update; agent_update $($rest:ident)*) => {
        true
    };
    ($want:ident; $other:ident $($rest:ident)*) => {
        flag!($want; $($rest)*)
    };
}

/// Declara os métodos num lugar só, um por linha: a variante de
/// [`Request`], o nome em JSON, o tipo do resultado (o `result` do
/// JSON-RPC) e, entre colchetes, as marcações: `read` (só lê), `install`
/// (mexe na instalação), `agent` (vira ferramenta de agente) e
/// `agent_update` (vira ferramenta só com `--allow-update`). Daqui saem
/// [`METHODS`], [`METHOD_INFO`], [`result_schema`],
/// [`Request::is_read_only`] e [`Request::is_installation`]. Uma variante de
/// [`Request`] que falte aqui não compila: o `match` de `declared` não tem
/// `_`.
macro_rules! methods {
    ($($variant:ident = $name:literal => $result:ty $([$($flag:ident),*])?,)*) => {
        /// Os métodos do protocolo, como vão em JSON: o que um servidor anuncia
        /// em [`ServerInfo::methods`] e usa para distinguir um método
        /// desconhecido de um com parâmetros inválidos. Mais `$/cancel`, a
        /// notificação que não é pedido.
        pub const METHODS: &[&str] = &[$($name),*];

        /// As marcações de cada método, na ordem de [`METHODS`].
        pub const METHOD_INFO: &[MethodInfo] = &[$(
            MethodInfo {
                name: $name,
                read_only: flag!(read; $($($flag)*)?),
                installation: flag!(install; $($($flag)*)?),
                agent: if flag!(agent; $($($flag)*)?) {
                    Agent::Tool
                } else if flag!(agent_update; $($($flag)*)?) {
                    Agent::AllowUpdate
                } else {
                    Agent::No
                },
            },
        )*];

        /// O resultado de cada método, uma propriedade por método.
        #[derive(JsonSchema)]
        #[allow(dead_code, non_snake_case)]
        struct Results {
            $(
                #[serde(rename = $name)]
                $variant: $result,
            )*
        }

        #[allow(dead_code)]
        fn declared(request: &Request) {
            match request {
                $(Request::$variant { .. } => {})*
            }
        }

        impl Request {
            /// O pedido só lê e não roda ferramenta (a marcação `read`): um
            /// servidor pode atendê-lo em paralelo com um fluxo longo do mesmo
            /// projeto, sem fila.
            pub fn is_read_only(&self) -> bool {
                match self {
                    $(Request::$variant { .. } => flag!(read; $($($flag)*)?),)*
                }
            }

            /// O pedido mexe na instalação da AURORA (a marcação `install`): um
            /// servidor o enfileira sozinho, com a trava da instalação.
            pub fn is_installation(&self) -> bool {
                match self {
                    $(Request::$variant { .. } => flag!(install; $($($flag)*)?),)*
                }
            }
        }
    };
}

/// As marcações do método `name`, ou `None` se ele não existe.
pub fn method_info(name: &str) -> Option<&'static MethodInfo> {
    METHOD_INFO.iter().find(|info| info.name == name)
}

methods! {
    Initialize = "initialize" => ServerInfo [read],
    Shutdown = "shutdown" => (),
    ProjectCreate = "project.create" => ProjectStatus [agent],
    ProjectStatus = "project.status" => ProjectStatus [read, agent],
    ProjectAdd = "project.add" => Vec<AddedFile> [agent],
    ProjectRemove = "project.remove" => RemoveOutcome [agent],
    ProjectTop = "project.top" => TopOutcome [agent],
    ProjectTestbench = "project.testbench" => ProjectStatus [agent],
    ProjectOrder = "project.order" => Vec<ProjectFile> [agent],
    ProjectMove = "project.move" => MovedPath [agent],
    ProjectConvert = "project.convert" => LayoutConversion [agent],
    ProjectCheckName = "project.check_name" => NameCheck [read],
    ProcessorCheckName = "processor.check_name" => NameCheck [read],
    ProcessorAdd = "processor.add" => Processor [agent],
    ProcessorSet = "processor.set" => Processor [agent],
    ProcessorRemove = "processor.remove" => ProcessorRemoval [agent],
    ProcessorOutputs = "processor.outputs" => Vec<i64> [read, agent],
    ProcessorDefaults = "processor.defaults" => NewProcessor [read],
    FlowRun = "flow.run" => FlowOutcome [agent],
    FlowCommand = "flow.command" => String [read],
    WaveOpen = "wave.open" => WaveOpened,
    WaveSignals = "wave.signals" => WaveSignals [agent],
    WaveChoose = "wave.choose" => WaveChoiceOutcome [agent],
    WaveResetLayout = "wave.reset_layout" => bool,
    WaveLayoutGet = "wave.layout" => WaveLayoutInfo,
    VerilogModules = "verilog.modules" => Vec<String> [read],
    VerilogTestbenchNames = "verilog.testbench_names" => Vec<bool> [read],
    HistoryList = "history.list" => Vec<RunSummary> [read, agent],
    HistoryShow = "history.show" => ReportView [read, agent],
    HistoryCompare = "history.compare" => RunComparison [read, agent],
    HistoryPlanCleanup = "history.plan_cleanup" => Vec<String> [read],
    HistoryClean = "history.clean" => CleanOutcome,
    ToolchainInfo = "toolchain.info" => ToolchainInfo [read, agent],
    ToolchainVerify = "toolchain.verify" => Vec<FileMismatch> [read, agent],
    ExperimentalList = "experimental.list" => ExperimentalInfo [read, agent],
    ExperimentalSet = "experimental.set" => ExperimentalInfo,
    FpgaBoards = "fpga.boards" => Vec<Board> [read, agent],
    FpgaConfigGet = "fpga.config" => Option<FpgaConfig> [read, agent],
    FpgaConfigSet = "fpga.config_set" => () [agent],
    FpgaConnect = "fpga.connect" => FpgaConfig [agent],
    FpgaCheck = "fpga.check" => Prepared [agent],
    FpgaModules = "fpga.modules" => Vec<String> [read, agent],
    FpgaStatus = "fpga.status" => BitstreamState [read, agent],
    FpgaCables = "fpga.cables" => Vec<String>,
    FpgaTop = "fpga.top" => ModuleInterface,
    VboardLayout = "vboard.layout" => BoardLayout [read],
    VboardRun = "vboard.run" => BoardRun,
    VboardInput = "vboard.input" => (),
    UpdateCheck = "update.check" => UpdateReport [install, agent],
    UpdateApply = "update.apply" => UpdateReport [install, agent_update],
    InstallAdd = "install.add" => InstallReport [install, agent_update],
    LearnCheck = "learn.check" => LearnCheckOutcome [agent],
    LearnTracks = "learn.tracks" => LearnTracks [read],
    LearnStatus = "learn.status" => WorkspaceSnapshot,
    LearnInit = "learn.init" => WorkspaceSnapshot,
    LearnSetCurrent = "learn.set_current" => WorkspaceSnapshot,
    LearnReset = "learn.reset" => (),
    GitStatus = "git.status" => GitStatus [read, agent],
    GitDiff = "git.diff" => String [read, agent],
    GitReadFile = "git.read_file" => FileVersion [read],
    GitCommit = "git.commit" => CommitResult [agent],
    GitDiscard = "git.discard" => DiscardResult,
    GitInit = "git.init" => InitResult,
    GitLog = "git.log" => GitLog [read, agent],
    GitCommitFiles = "git.commit_files" => CommitFiles [read, agent],
    GitBranches = "git.branches" => Branches [read, agent],
    GitSwitch = "git.switch" => String,
    GitFetch = "git.fetch" => SyncResult,
    GitPull = "git.pull" => SyncResult,
    GitPush = "git.push" => SyncResult,
    GitSetIdentity = "git.set_identity" => (),
    GitSetRemote = "git.set_remote" => (),
    GitRemoveRemote = "git.remove_remote" => (),
    GithubAccount = "github.account" => Option<Account> [read],
    GithubLoginStart = "github.login_start" => DeviceCode,
    GithubLoginPoll = "github.login_poll" => LoginPoll,
    GithubLogout = "github.logout" => bool,
}

/// O id de um pedido JSON-RPC, como o cliente o mandou: número ou texto.
/// Só para o schema de `vboard.input`.
#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum RequestId {
    /// Um número.
    Number(i64),
    /// Um texto.
    Text(String),
}

/// O que um cliente pede. Em JSON, `{"method": "...", "params": {...}}`;
/// um método sem parâmetros vai sem `params`. Os caminhos são absolutos.
// Sem `non_exhaustive`: o despacho do servidor faz `match` sem `_`, e um
// método novo sem braço no despacho não compila.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum Request {
    /// O primeiro pedido de uma sessão: quem é o cliente. Resposta:
    /// [`ServerInfo`].
    Initialize {
        /// O cliente e a versão dele (`aurora-vscode 0.1.0`).
        #[serde(default)]
        client: Option<String>,
    },
    /// Encerrar a sessão: os pedidos em andamento são cancelados.
    Shutdown,

    /// Cria `<parent>/<name>/<name>.spf`. Resposta: o [`ProjectStatus`] do
    /// projeto novo.
    #[serde(rename = "project.create")]
    ProjectCreate {
        /// A pasta de cima, que precisa existir.
        #[schemars(with = "String")]
        parent: Utf8PathBuf,
        /// O nome do projeto.
        name: String,
    },
    /// O retrato do projeto. Resposta: [`ProjectStatus`].
    #[serde(rename = "project.status")]
    ProjectStatus {
        /// O `.spf` ou a pasta do projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
    },
    /// Registra arquivos Verilog e VHDL, ou testbenches cocotb (criando os
    /// que não existem, pelos modelos). Resposta: um [`AddedFile`] por
    /// arquivo.
    #[serde(rename = "project.add")]
    ProjectAdd {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// Os arquivos.
        #[schemars(with = "Vec<String>")]
        files: Vec<Utf8PathBuf>,
        /// Registrar como testbench em vez de decidir pelo conteúdo.
        #[serde(default)]
        testbench: bool,
    },
    /// Tira arquivos do projeto, sem apagar. Resposta: [`RemoveOutcome`].
    #[serde(rename = "project.remove")]
    ProjectRemove {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// Os arquivos.
        #[schemars(with = "Vec<String>")]
        files: Vec<Utf8PathBuf>,
    },
    /// Escolhe o topo por arquivo ou por nome de módulo; sem alvo, só
    /// consulta. Resposta: [`TopOutcome`].
    #[serde(rename = "project.top")]
    ProjectTop {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// O arquivo ou o módulo.
        #[serde(default)]
        target: Option<String>,
    },
    /// Escolhe o testbench da simulação do projeto. Resposta:
    /// [`ProjectStatus`].
    #[serde(rename = "project.testbench")]
    ProjectTestbench {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// O testbench.
        #[schemars(with = "String")]
        path: Utf8PathBuf,
    },
    /// Muda a posição de um arquivo na lista dele. Resposta: a lista, na
    /// ordem nova ([`ProjectFile`]).
    #[serde(rename = "project.order")]
    ProjectOrder {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// O arquivo.
        #[schemars(with = "String")]
        file: Utf8PathBuf,
        /// Para onde.
        position: ListPosition,
    },
    /// Move ou renomeia um arquivo ou pasta do projeto, mantendo o `.spf` em
    /// dia; com `after`, só registra um movimento já feito. Resposta:
    /// [`MovedPath`].
    #[serde(rename = "project.move")]
    ProjectMove {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// De onde.
        #[schemars(with = "String")]
        from: Utf8PathBuf,
        /// Para onde (o caminho final).
        #[schemars(with = "String")]
        to: Utf8PathBuf,
        /// O movimento já aconteceu fora da AURORA.
        #[serde(default)]
        after: bool,
        /// Se o destino existe, ele vai para a lixeira do sistema e o
        /// movimento acontece; sem isso, `path_exists`. O que a AURORA mantém
        /// no lugar, a pasta do projeto e uma pasta que contém a origem não
        /// são substituídos (`cannot_replace`). Não vale com `after`.
        #[serde(default)]
        overwrite: bool,
    },
    /// Converte um projeto da estrutura do YANC 6.x. Resposta:
    /// [`LayoutConversion`].
    #[serde(rename = "project.convert")]
    ProjectConvert {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
    },
    /// Um nome serve para um projeto novo? Resposta: [`NameCheck`].
    #[serde(rename = "project.check_name")]
    ProjectCheckName {
        /// O nome.
        name: String,
    },
    /// Um nome serve para um processador novo? Resposta: [`NameCheck`].
    #[serde(rename = "processor.check_name")]
    ProcessorCheckName {
        /// O nome.
        name: String,
    },
    /// Cria um processador. Resposta: o [`Processor`].
    #[serde(rename = "processor.add")]
    ProcessorAdd {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// O processador; só `name` e `language` são obrigatórios.
        spec: NewProcessor,
    },
    /// Muda os parâmetros de simulação de um processador. Resposta: o
    /// [`Processor`].
    #[serde(rename = "processor.set")]
    ProcessorSet {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// O processador.
        name: String,
        /// O que muda (`None` mantém).
        config: ProcessorConfig,
    },
    /// Tira um processador do projeto. Resposta: [`ProcessorRemoval`].
    #[serde(rename = "processor.remove")]
    ProcessorRemove {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// O processador.
        name: String,
        /// Mandar a pasta dele para a lixeira do sistema.
        #[serde(default)]
        to_trash: bool,
    },
    /// Os valores de uma porta de saída da última simulação do processador.
    /// Resposta: os valores.
    #[serde(rename = "processor.outputs")]
    ProcessorOutputs {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// O processador.
        processor: String,
        /// A porta.
        port: u32,
    },
    /// Os padrões de um processador novo (os da AURORA), para um formulário
    /// começar preenchido. Resposta: o [`NewProcessor`], com o nome vazio.
    #[serde(rename = "processor.defaults")]
    ProcessorDefaults,

    /// Roda um fluxo ([`FlowRequest`]). Enquanto roda, as notificações
    /// (`phase`, `events`, `output_omitted`, `build`); no fim, o
    /// [`FlowOutcome`].
    #[serde(rename = "flow.run")]
    FlowRun {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// O fluxo.
        request: FlowRequest,
        /// As opções da interface; sem elas, o comando é o da CLI
        /// equivalente, a onda abre em janela e o RTL viewer no navegador.
        #[serde(default)]
        options: Option<FlowOptions>,
        /// Receber a saída das ferramentas ao vivo (`events`). Sem isso, o
        /// `vvp` roda com buffer, que é mais rápido.
        #[serde(default = "yes")]
        output: bool,
        /// Quantos bytes do `stdout` e do `stderr` de cada passo voltam no
        /// resultado, do fim; sem isso, tudo. Com `output`, cada linha já
        /// foi pelas notificações `events`.
        #[serde(default)]
        step_output_limit: Option<usize>,
    },
    /// O comando equivalente de um fluxo, como a CLI o escreveria e como ele
    /// vai para o histórico (`aurora-ide sim -p soma`), para uma interface
    /// mostrar antes de rodar. Resposta: o texto.
    #[serde(rename = "flow.command")]
    FlowCommand {
        /// O fluxo.
        request: FlowRequest,
        /// O programa que começa o comando; sem ele, `aurora`.
        #[serde(default)]
        program: Option<String>,
    },

    /// Abre uma onda no surfer-aurora. Resposta: [`WaveOpened`].
    #[serde(rename = "wave.open")]
    WaveOpen {
        /// O projeto, para uma onda do projeto; `None` para um arquivo.
        #[serde(default)]
        #[schemars(with = "Option<String>")]
        project: Option<Utf8PathBuf>,
        /// Qual onda e com que layout.
        request: WaveRequest,
    },
    /// A árvore de sinais do testbench do projeto, com a escolha gravada.
    /// Resposta: [`WaveSignals`].
    #[serde(rename = "wave.signals")]
    WaveSignals {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
    },
    /// Muda a escolha de sinais. Resposta: [`WaveChoiceOutcome`].
    #[serde(rename = "wave.choose")]
    WaveChoose {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// A mudança.
        choice: WaveChoice,
    },
    /// Apaga o layout salvo de uma onda. Resposta: se havia.
    #[serde(rename = "wave.reset_layout")]
    WaveResetLayout {
        /// A onda.
        #[schemars(with = "String")]
        waveform: Utf8PathBuf,
    },
    /// O layout da AURORA para uma onda, para um visualizador próprio (a aba de
    /// onda do Studio): o gerado dos processadores SAPHO, o salvo no projeto
    /// da onda (que o gerado atualiza enquanto o usuário não salvar o dele)
    /// e o de um exercício do `aurora learn`. Resposta: [`WaveLayoutInfo`].
    #[serde(rename = "wave.layout")]
    WaveLayoutGet {
        /// A onda.
        #[schemars(with = "String")]
        waveform: Utf8PathBuf,
    },

    /// Os módulos declarados num texto Verilog, sem os comentados, na ordem
    /// (o contorno de um editor). Resposta: os nomes.
    #[serde(rename = "verilog.modules")]
    VerilogModules {
        /// O texto.
        text: String,
    },
    /// Quais nomes de arquivo são de testbench pela regra da AURORA
    /// (`tb_<nome>.v`, `<nome>_tb.v`). Resposta: um sim ou não por nome, na
    /// ordem.
    #[serde(rename = "verilog.testbench_names")]
    VerilogTestbenchNames {
        /// Os nomes, sem pasta.
        names: Vec<String>,
    },

    /// Os relatórios do projeto, do mais novo para o mais antigo. Resposta:
    /// [`RunSummary`].
    #[serde(rename = "history.list")]
    HistoryList {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// Quantos, no máximo.
        #[serde(default)]
        limit: Option<usize>,
    },
    /// Um relatório (o mais novo sem `id`). Resposta: [`ReportView`].
    #[serde(rename = "history.show")]
    HistoryShow {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// `run-000042` ou `42`.
        #[serde(default)]
        id: Option<String>,
    },
    /// Compara dois relatórios. Resposta: [`RunComparison`].
    #[serde(rename = "history.compare")]
    HistoryCompare {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// O atual (o mais novo sem ele).
        #[serde(default)]
        id: Option<String>,
        /// A referência (o anterior comparável sem ela).
        #[serde(default)]
        against: Option<String>,
    },
    /// Quais relatórios uma limpeza apagaria, sem apagar. Resposta: os
    /// identificadores.
    #[serde(rename = "history.plan_cleanup")]
    HistoryPlanCleanup {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// A regra.
        cleanup: Cleanup,
    },
    /// Apaga exatamente estes relatórios. Resposta: [`CleanOutcome`].
    #[serde(rename = "history.clean")]
    HistoryClean {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// Os identificadores (do plano).
        ids: Vec<String>,
    },

    /// O bundle e as ferramentas. Resposta: [`ToolchainInfo`].
    #[serde(rename = "toolchain.info")]
    ToolchainInfo,
    /// Confere o SHA-256 dos executáveis. Resposta: os que não conferem
    /// ([`FileMismatch`]; vazio, todos conferem).
    #[serde(rename = "toolchain.verify")]
    ToolchainVerify,

    /// Os recursos experimentais e se estão ligados. Resposta:
    /// [`ExperimentalInfo`].
    #[serde(rename = "experimental.list")]
    ExperimentalList,
    /// Liga ou desliga um recurso experimental. Resposta:
    /// [`ExperimentalInfo`].
    #[serde(rename = "experimental.set")]
    ExperimentalSet {
        /// O recurso (`fpga`).
        feature: String,
        /// Ligado.
        enabled: bool,
    },

    /// As placas conhecidas, ou uma. Resposta: [`Board`].
    #[serde(rename = "fpga.boards")]
    FpgaBoards {
        /// Uma placa; `None`, todas.
        #[serde(default)]
        board: Option<String>,
    },
    /// O `fpga.json` do projeto. Resposta: o [`FpgaConfig`], ou `null`.
    #[serde(rename = "fpga.config")]
    FpgaConfigGet {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
    },
    /// Grava o `fpga.json`. Resposta: nada.
    #[serde(rename = "fpga.config_set")]
    FpgaConfigSet {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// A configuração.
        config: FpgaConfig,
    },
    /// Muda uma ligação a partir da placa: um bit de um sinal (ou o sinal
    /// inteiro) passa a ir numa porta do topo, ou fica solto; o que estava ali
    /// sai, e as outras ligações continuam. Grava o `fpga.json`; sem ele,
    /// parte das portas ligadas pelo nome com a placa `board`. Resposta: o
    /// [`FpgaConfig`] gravado.
    #[serde(rename = "fpga.connect")]
    FpgaConnect {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// A placa, num projeto sem `fpga.json`.
        #[serde(default)]
        board: Option<String>,
        /// A mudança.
        change: Reconnect,
    },
    /// Confere o `fpga.json` contra a placa e o topo. Resposta:
    /// [`Prepared`].
    #[serde(rename = "fpga.check")]
    FpgaCheck {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
    },
    /// Os módulos do projeto que podem ir para a placa. Resposta: os
    /// nomes, na ordem dos arquivos.
    #[serde(rename = "fpga.modules")]
    FpgaModules {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
    },
    /// O arquivo de gravação da última compilação e se serve. Resposta:
    /// [`BitstreamState`].
    #[serde(rename = "fpga.status")]
    FpgaStatus {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
    },
    /// Os cabos de gravação ligados. Resposta: os nomes.
    #[serde(rename = "fpga.cables")]
    FpgaCables,
    /// O módulo que vai para a placa e as portas dele, para montar as
    /// ligações antes de estarem certas. Resposta: o [`ModuleInterface`].
    #[serde(rename = "fpga.top")]
    FpgaTop {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// O módulo; `None`, o `top` do `fpga.json`, senão o do projeto.
        #[serde(default)]
        top: Option<String>,
    },

    /// A placa virtual do projeto sem compilar: a placa, o topo e os
    /// periféricos, ligados ou não. Resposta: [`BoardLayout`].
    #[serde(rename = "vboard.layout")]
    VboardLayout {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// A placa, num projeto sem `fpga.json`: as portas com o nome de um
        /// sinal dela ligam a ele.
        #[serde(default)]
        board: Option<String>,
    },
    /// Compila a placa virtual e a roda até `$/cancel` (ou o `$finish` do
    /// projeto). Enquanto compila, `events`; rodando, `vboard_ready` com a
    /// placa, `vboard_state` cerca de 30 vezes por segundo, `vboard_frame`
    /// quando a imagem muda e `vboard_output` a cada linha do projeto. As
    /// entradas vão por `vboard.input` com o id deste pedido. Resposta:
    /// [`BoardRun`]; com o Verilog que não compila, sem `outcome`.
    #[serde(rename = "vboard.run")]
    VboardRun {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// A placa, num projeto sem `fpga.json` (como em `vboard.layout`).
        #[serde(default)]
        board: Option<String>,
        /// Segundos simulados por segundo de relógio, no máximo: 1 é o
        /// tempo real (o padrão), 0 é o mais rápido possível.
        #[serde(default)]
        limit: Option<f64>,
        /// Quadros de vídeo por segundo, no máximo (padrão 30).
        #[serde(default)]
        fps: Option<f64>,
    },
    /// Uma entrada para a placa virtual de um `vboard.run` que está rodando:
    /// chaves, botões, teclas, texto, pausa, velocidade. Resposta: nada; um
    /// periférico que a placa não tem é erro.
    #[serde(rename = "vboard.input")]
    VboardInput {
        /// O id do pedido `vboard.run`, como o cliente o mandou.
        #[schemars(with = "RequestId")]
        run: serde_json::Value,
        /// A entrada.
        input: BoardInput,
    },

    /// As versões: esta AURORA, a última release, o bundle, cada componente.
    /// Resposta: o relatório do instalador.
    #[serde(rename = "update.check")]
    UpdateCheck,
    /// Atualiza a instalação. Enquanto roda, `install`; no fim, o
    /// relatório do instalador. Com `updated`, o servidor encerra em
    /// seguida: o binário dele foi trocado.
    #[serde(rename = "update.apply")]
    UpdateApply {
        /// O instalador inteiro, sem tentar por componentes.
        #[serde(default)]
        full: bool,
        /// Um instalador no disco, sem rede.
        #[serde(default)]
        #[schemars(with = "Option<String>")]
        from: Option<Utf8PathBuf>,
        /// No Windows, quando o assistente de instalação roda: abrir a AURORA
        /// Studio quando ele terminar (o `--reopen-ide` da CLI). O Studio
        /// pede isso e fecha, para o assistente trocar os arquivos dele.
        #[serde(default)]
        reopen_ide: bool,
    },
    /// Instala componentes do bundle. Enquanto roda, `install`; no fim, o
    /// relatório do instalador. Um nome que não é do bundle é recusado
    /// antes de qualquer download.
    #[serde(rename = "install.add")]
    InstallAdd {
        /// Os componentes (pelo menos um).
        components: Vec<String>,
    },

    /// Corrige um exercício do `aurora learn` (ou todos). Resposta: o
    /// relatório da correção do aurora-learn.
    #[serde(rename = "learn.check")]
    LearnCheck {
        /// A pasta de exercícios, ou uma pasta dentro dela.
        #[schemars(with = "String")]
        root: Utf8PathBuf,
        /// O exercício; `None`, o atual.
        #[serde(default)]
        exercise: Option<String>,
        /// Todos os exercícios.
        #[serde(default)]
        all: bool,
    },
    /// De onde vêm as trilhas do `aurora learn` e quais há. Resposta: `{dir,
    /// tracks}`.
    #[serde(rename = "learn.tracks")]
    LearnTracks,
    /// Abre a pasta de exercícios que contém `root` e a põe em dia com a
    /// trilha. Resposta: o retrato da pasta (o `WorkspaceSnapshot` do
    /// aurora-learn).
    #[serde(rename = "learn.status")]
    LearnStatus {
        /// A pasta de exercícios, ou uma pasta dentro dela.
        #[schemars(with = "String")]
        root: Utf8PathBuf,
        /// O idioma dos enunciados, quando a trilha tem tradução.
        #[serde(default)]
        lang: Option<String>,
    },
    /// Cria a pasta de exercícios (`aurora learn init`). Resposta: o retrato
    /// da pasta.
    #[serde(rename = "learn.init")]
    LearnInit {
        /// A pasta, que pode existir vazia.
        #[schemars(with = "String")]
        dir: Utf8PathBuf,
        /// A trilha; `None`, a padrão.
        #[serde(default)]
        track: Option<String>,
        /// O idioma dos enunciados.
        #[serde(default)]
        lang: Option<String>,
    },
    /// Torna atual um exercício. Resposta: o retrato da pasta.
    #[serde(rename = "learn.set_current")]
    LearnSetCurrent {
        /// A pasta de exercícios.
        #[schemars(with = "String")]
        root: Utf8PathBuf,
        /// O exercício.
        name: String,
        /// O idioma dos enunciados.
        #[serde(default)]
        lang: Option<String>,
    },
    /// Volta o arquivo de um exercício ao começo (`aurora learn reset`).
    /// Resposta: nada.
    #[serde(rename = "learn.reset")]
    LearnReset {
        /// A pasta de exercícios.
        #[schemars(with = "String")]
        root: Utf8PathBuf,
        /// O exercício.
        name: String,
    },

    /// O git da máquina e o repositório que contém o projeto: ramo,
    /// arquivos mudados, identidade e remotos. Fora de repositório,
    /// `repository` vem `null`. Resposta: [`GitStatus`].
    #[serde(rename = "git.status")]
    GitStatus {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
    },
    /// O diff unificado do que mudou desde o último commit, num arquivo ou
    /// em todos. Resposta: o texto.
    #[serde(rename = "git.diff")]
    GitDiff {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// Um arquivo, relativo ao topo do repositório, com `/`; `None`,
        /// todos.
        #[serde(default)]
        path: Option<String>,
    },
    /// O conteúdo de um arquivo no índice, no último commit ou num commit,
    /// para comparar com o do disco. Resposta: [`FileVersion`].
    #[serde(rename = "git.read_file")]
    GitReadFile {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// A versão.
        revision: Revision,
        /// O arquivo, relativo ao topo do repositório, com `/`.
        path: String,
    },
    /// Faz o commit dos arquivos escolhidos, no estado em que estão no
    /// disco. Resposta: [`CommitResult`].
    #[serde(rename = "git.commit")]
    GitCommit {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// A primeira linha da mensagem.
        summary: String,
        /// O resto da mensagem.
        #[serde(default)]
        description: Option<String>,
        /// Os arquivos, relativos ao topo, com `/`; `None`, todos os mudados.
        #[serde(default)]
        paths: Option<Vec<String>>,
        /// Refaz o último commit.
        #[serde(default)]
        amend: bool,
    },
    /// Descarta as mudanças de arquivos: voltam ao último commit, e os novos
    /// saem; a versão descartada vai para a lixeira do sistema. Resposta:
    /// [`DiscardResult`].
    #[serde(rename = "git.discard")]
    GitDiscard {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// Os arquivos, relativos ao topo, com `/`.
        paths: Vec<String>,
    },
    /// Cria um repositório na pasta do projeto, com um `.gitignore` para o
    /// que a AURORA gera. Resposta: [`InitResult`].
    #[serde(rename = "git.init")]
    GitInit {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
    },
    /// Uma página do histórico do ramo atual. Resposta: [`GitLog`].
    #[serde(rename = "git.log")]
    GitLog {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// Quantos commits pular, dos mais novos.
        #[serde(default)]
        skip: Option<usize>,
        /// Quantos devolver (até 500; 50 sem o campo).
        #[serde(default)]
        limit: Option<usize>,
    },
    /// Os arquivos que um commit mudou. Resposta: [`CommitFiles`].
    #[serde(rename = "git.commit_files")]
    GitCommitFiles {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// O identificador do commit.
        oid: String,
    },
    /// Os ramos locais e remotos. Resposta: [`Branches`].
    #[serde(rename = "git.branches")]
    GitBranches {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
    },
    /// Troca de ramo, ou cria um. Resposta: o ramo atual depois da troca.
    #[serde(rename = "git.switch")]
    GitSwitch {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// Um ramo local, um remoto (`origin/x`) ou, com `create`, o novo.
        branch: String,
        /// Cria o ramo a partir do commit atual.
        #[serde(default)]
        create: bool,
    },
    /// Traz do remoto os ramos e commits novos, sem mexer nos arquivos.
    /// Resposta: [`SyncResult`].
    #[serde(rename = "git.fetch")]
    GitFetch {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
    },
    /// Traz os commits do ramo remoto, só em avanço direto. Resposta:
    /// [`SyncResult`].
    #[serde(rename = "git.pull")]
    GitPull {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
    },
    /// Envia o ramo atual; sem ramo remoto, publica. Resposta:
    /// [`SyncResult`].
    #[serde(rename = "git.push")]
    GitPush {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
    },
    /// Grava o nome e o e-mail dos commits. Resposta: nada.
    #[serde(rename = "git.set_identity")]
    GitSetIdentity {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// O nome.
        name: String,
        /// O e-mail.
        email: String,
        /// Na configuração global do git, para todos os repositórios desta
        /// máquina; sem o campo, só neste.
        #[serde(default)]
        global: bool,
    },
    /// Dá um endereço a um remoto: cria o remoto ou troca o endereço de um
    /// que já existe. Resposta: nada.
    #[serde(rename = "git.set_remote")]
    GitSetRemote {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// O nome (`origin`).
        name: String,
        /// O endereço.
        url: String,
    },
    /// Tira um remoto. Resposta: nada.
    #[serde(rename = "git.remove_remote")]
    GitRemoveRemote {
        /// O projeto.
        #[schemars(with = "String")]
        project: Utf8PathBuf,
        /// O nome.
        name: String,
    },
    /// A conta do GitHub que o git desta máquina conhece. Resposta:
    /// [`Account`], ou `null`.
    #[serde(rename = "github.account")]
    GithubAccount,
    /// Começa o login no GitHub: o código que a pessoa digita em
    /// github.com/login/device. Resposta: [`DeviceCode`].
    #[serde(rename = "github.login_start")]
    GithubLoginStart,
    /// Pergunta se a pessoa já autorizou o login; com a autorização, guarda o
    /// token no gerenciador de credenciais do git. Resposta: [`LoginPoll`].
    #[serde(rename = "github.login_poll")]
    GithubLoginPoll {
        /// O `device_code` de `github.login_start`.
        device_code: String,
    },
    /// Sai do GitHub: tira o token do gerenciador de credenciais do git.
    /// Resposta: se havia um.
    #[serde(rename = "github.logout")]
    GithubLogout,
}

impl Request {
    /// O nome do método (`project.status`), como vai em JSON.
    pub fn method(&self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.get("method")?.as_str().map(str::to_owned))
            .unwrap_or_default()
    }

    /// O projeto a que o pedido se refere, quando há um: é por ele que um
    /// servidor enfileira os pedidos (um de cada vez por projeto).
    pub fn project(&self) -> Option<&Utf8PathBuf> {
        match self {
            Request::ProjectStatus { project }
            | Request::ProjectAdd { project, .. }
            | Request::ProjectRemove { project, .. }
            | Request::ProjectTop { project, .. }
            | Request::ProjectTestbench { project, .. }
            | Request::ProjectOrder { project, .. }
            | Request::ProjectMove { project, .. }
            | Request::ProjectConvert { project }
            | Request::ProcessorAdd { project, .. }
            | Request::ProcessorSet { project, .. }
            | Request::ProcessorRemove { project, .. }
            | Request::ProcessorOutputs { project, .. }
            | Request::FlowRun { project, .. }
            | Request::WaveSignals { project }
            | Request::WaveChoose { project, .. }
            | Request::HistoryList { project, .. }
            | Request::HistoryShow { project, .. }
            | Request::HistoryCompare { project, .. }
            | Request::HistoryPlanCleanup { project, .. }
            | Request::HistoryClean { project, .. }
            | Request::FpgaConfigGet { project }
            | Request::FpgaConfigSet { project, .. }
            | Request::FpgaConnect { project, .. }
            | Request::FpgaCheck { project }
            | Request::FpgaModules { project }
            | Request::FpgaStatus { project }
            | Request::FpgaTop { project, .. }
            | Request::VboardLayout { project, .. }
            | Request::VboardRun { project, .. }
            | Request::GitStatus { project }
            | Request::GitDiff { project, .. }
            | Request::GitReadFile { project, .. }
            | Request::GitCommit { project, .. }
            | Request::GitDiscard { project, .. }
            | Request::GitInit { project }
            | Request::GitLog { project, .. }
            | Request::GitCommitFiles { project, .. }
            | Request::GitBranches { project }
            | Request::GitSwitch { project, .. }
            | Request::GitFetch { project }
            | Request::GitPull { project }
            | Request::GitPush { project }
            | Request::GitSetIdentity { project, .. }
            | Request::GitSetRemote { project, .. }
            | Request::GitRemoveRemote { project, .. } => Some(project),
            Request::WaveOpen { project, .. } => project.as_ref(),
            _ => None,
        }
    }
}

/// O que volta de um pedido que deu certo. Serializa como o valor em si (o
/// `result` do JSON-RPC), sem etiqueta: cada método diz qual é o dele.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(untagged)]
#[non_exhaustive]
pub enum Response {
    /// `initialize`.
    Server(ServerInfo),
    /// `project.create`, `project.status`, `project.testbench`.
    Status(Box<ProjectStatus>),
    /// `project.add`.
    Added(Vec<AddedFile>),
    /// `project.remove`.
    Removed(RemoveOutcome),
    /// `project.top`.
    Top(TopOutcome),
    /// `project.order`.
    Files(Vec<ProjectFile>),
    /// `project.move`.
    Moved(MovedPath),
    /// `project.convert`.
    Converted(LayoutConversion),
    /// `project.check_name`, `processor.check_name`.
    Name(NameCheck),
    /// `processor.add`, `processor.set`.
    Processor(Box<Processor>),
    /// `processor.remove`.
    ProcessorRemoved(ProcessorRemoval),
    /// `processor.outputs`.
    Values(Vec<i64>),
    /// `processor.defaults`.
    Defaults(Box<NewProcessor>),
    /// `flow.command`.
    Text(String),
    /// `wave.layout`.
    Layout(Box<WaveLayoutInfo>),
    /// `verilog.testbench_names`.
    Flags(Vec<bool>),
    /// `fpga.top`.
    Interface(Box<ModuleInterface>),
    /// `vboard.layout`.
    VboardLayout(Box<BoardLayout>),
    /// `vboard.run`.
    Vboard(Box<BoardRun>),
    /// `flow.run`.
    Flow(Box<FlowOutcome>),
    /// `wave.open`.
    Wave(WaveOpened),
    /// `wave.signals`.
    Signals(Box<WaveSignals>),
    /// `wave.choose`.
    Choice(WaveChoiceOutcome),
    /// `wave.reset_layout`.
    Flag(bool),
    /// `history.list`.
    Reports(Vec<RunSummary>),
    /// `history.show`.
    Report(Box<ReportView>),
    /// `history.compare`.
    Comparison(Box<RunComparison>),
    /// `history.plan_cleanup`.
    Ids(Vec<String>),
    /// `history.clean`.
    Cleaned(CleanOutcome),
    /// `toolchain.info`.
    Toolchain(Box<ToolchainInfo>),
    /// `toolchain.verify`.
    Mismatches(Vec<FileMismatch>),
    /// `experimental.list`, `experimental.set`.
    Experimental(ExperimentalInfo),
    /// `fpga.boards`.
    Boards(Vec<Board>),
    /// `fpga.config`.
    FpgaConfig(Option<FpgaConfig>),
    /// `fpga.connect`.
    FpgaConnected(Box<FpgaConfig>),
    /// `fpga.check`.
    Prepared(Box<Prepared>),
    /// `fpga.modules`, `verilog.modules`.
    Modules(Vec<String>),
    /// `fpga.status`.
    Bitstream(BitstreamState),
    /// `fpga.cables`.
    Cables(Vec<String>),
    /// `shutdown`, `fpga.config_set`, `learn.reset`: nada.
    Nothing(()),
    /// `update.check`, `update.apply`.
    Update(Box<UpdateReport>),
    /// `install.add`.
    Install(Box<InstallReport>),
    /// `learn.check`.
    LearnCheck(Box<LearnCheckOutcome>),
    /// `learn.tracks`.
    Tracks(LearnTracks),
    /// `learn.status`, `learn.init`, `learn.set_current`.
    Workspace(Box<WorkspaceSnapshot>),
    /// `git.status`.
    GitStatus(Box<GitStatus>),
    /// `git.read_file`.
    FileVersion(Box<FileVersion>),
    /// `git.commit`.
    Commit(Box<CommitResult>),
    /// `git.discard`.
    Discarded(Box<DiscardResult>),
    /// `git.init`.
    GitInit(Box<InitResult>),
    /// `git.log`.
    GitLog(Box<GitLog>),
    /// `git.commit_files`.
    CommitFiles(Box<CommitFiles>),
    /// `git.branches`.
    Branches(Box<Branches>),
    /// `git.fetch`, `git.pull`, `git.push`.
    Sync(Box<SyncResult>),
    /// `github.account`.
    Account(Option<Box<Account>>),
    /// `github.login_start`.
    DeviceCode(Box<DeviceCode>),
    /// `github.login_poll`.
    LoginPoll(Box<LoginPoll>),
}

/// O que `initialize` devolve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ServerInfo {
    /// A versão da AURORA.
    pub aurora: String,
    /// A versão do protocolo ([`PROTOCOL_VERSION`]).
    pub protocol: u32,
    /// A pasta da instalação da AURORA, quando o servidor é o `aurora`
    /// instalado.
    #[schemars(with = "Option<String>")]
    pub prefix: Option<Utf8PathBuf>,
    /// Cada recurso experimental e se está ligado.
    pub experimental: BTreeMap<String, bool>,
    /// Os métodos que este servidor atende.
    pub methods: Vec<String>,
}

/// `wave.layout`: o que um visualizador próprio precisa para mostrar a
/// onda com o layout da AURORA.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct WaveLayoutInfo {
    /// O layout dos processadores SAPHO, gerado da onda; `None` sem
    /// processador.
    pub layout: Option<WaveLayout>,
    /// O layout salvo no projeto da onda (`wave/<testbench>.surf.ron`), onde
    /// o Surfer salva (Ctrl+S): o do usuário, se ele salvou depois da última
    /// geração, senão o gerado agora; `None` numa onda fora de projeto.
    pub saved: Option<SavedLayout>,
    /// Por que o layout não foi gerado (a onda abre sem ele).
    pub layout_error: Option<String>,
    /// Os comandos do Surfer de um exercício do `aurora learn` (o
    /// `.aurora-learn/wave.sucl` ao lado da onda), quando a onda é de um.
    #[schemars(with = "Option<String>")]
    pub exercise_layout: Option<Utf8PathBuf>,
}

/// `project.top`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct TopOutcome {
    /// O arquivo de topo, se escolhido.
    #[schemars(with = "Option<String>")]
    pub top_level: Option<Utf8PathBuf>,
    /// O módulo de topo.
    pub module: Option<String>,
    /// Por que não dá para saber o módulo de um arquivo de topo escolhido.
    pub module_error: Option<ErrorInfo>,
}

/// `project.check_name` e `processor.check_name`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct NameCheck {
    /// O nome serve.
    pub ok: bool,
    /// Por que não serve (`invalid_name`).
    pub error: Option<ErrorInfo>,
}

/// `processor.remove`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ProcessorRemoval {
    /// O que saiu: o nome, a pasta e os arquivos que saíram das listas.
    pub removed: RemovedProcessor,
    /// A pasta foi para a lixeira do sistema.
    pub trashed: bool,
}

/// Como um pedido terminou, do ponto de vista de quem o atende.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShutdownReason {
    /// O cliente pediu (`shutdown`) ou fechou a entrada.
    Requested,
    /// O processo do servidor recebeu um sinal.
    Signal,
    /// A instalação foi atualizada e o binário do servidor é outro: o
    /// cliente o abre de novo.
    Updated,
}

/// O schema dos pedidos, na vista de quem desserializa: um campo com padrão
/// é opcional. Em `$defs`, cada tipo que um pedido carrega.
pub fn request_schema() -> schemars::Schema {
    schemars::generate::SchemaSettings::draft2020_12()
        .for_deserialize()
        .into_generator()
        .into_root_schema_for::<Request>()
}

/// O schema das respostas e das notificações, na vista de quem serializa:
/// um campo que sai sempre é obrigatório. Em `$defs`, cada tipo de
/// resultado, mais [`Notification`], [`ErrorInfo`], [`ServerInfo`] e
/// [`ShutdownReason`].
pub fn response_schema() -> schemars::Schema {
    #[derive(JsonSchema)]
    #[allow(dead_code)]
    struct Responses {
        response: Response,
        notification: Notification,
        error: ErrorInfo,
        shutdown: ShutdownReason,
    }
    schemars::generate::SchemaSettings::draft2020_12()
        .for_serialize()
        .into_generator()
        .into_root_schema_for::<Responses>()
}

/// O schema do resultado de cada método, na vista de quem serializa: um
/// objeto com uma propriedade por método (`project.status`), na ordem de
/// [`METHODS`], que é o `result` que o servidor devolve a ele. Em `$defs`,
/// os tipos dos resultados.
pub fn result_schema() -> schemars::Schema {
    schemars::generate::SchemaSettings::draft2020_12()
        .for_serialize()
        .into_generator()
        .into_root_schema_for::<Results>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn methods_have_dotted_names_and_optional_params() {
        let request: Request =
            serde_json::from_str(r#"{"method":"project.status","params":{"project":"/p"}}"#)
                .unwrap();
        assert_eq!(request.method(), "project.status");
        assert_eq!(request.project().unwrap(), "/p");
        assert!(request.is_read_only());

        let request: Request = serde_json::from_str(r#"{"method":"toolchain.info"}"#).unwrap();
        assert_eq!(request, Request::ToolchainInfo);
        assert!(request.project().is_none());

        let request: Request = serde_json::from_str(
            r#"{"method":"flow.run","params":{"project":"/p","request":{"flow":"build"}}}"#,
        )
        .unwrap();
        assert!(matches!(
            request,
            Request::FlowRun {
                output: true,
                options: None,
                ..
            }
        ));
        assert!(!request.is_read_only());
        assert_eq!(request.method(), "flow.run");

        let request: Request = serde_json::from_str(
            r#"{"method":"processor.add","params":{"project":"/p","spec":{"name":"soma","language":"cmm"}}}"#,
        )
        .unwrap();
        if let Request::ProcessorAdd { spec, .. } = request {
            assert_eq!(spec.nubits, 23, "os padrões valem no protocolo");
        } else {
            panic!("outro pedido");
        }
        assert!(
            Request::UpdateApply {
                full: false,
                from: None,
                reopen_ide: false,
            }
            .is_installation()
        );
    }

    #[test]
    fn every_method_has_its_marks_in_the_order_of_the_list() {
        let names: Vec<&str> = METHOD_INFO.iter().map(|info| info.name).collect();
        assert_eq!(names, METHODS);
        // Um método que só vira ferramenta com --allow-update mexe na
        // instalação.
        for info in METHOD_INFO
            .iter()
            .filter(|info| info.agent == Agent::AllowUpdate)
        {
            assert!(info.installation, "{}", info.name);
        }
        let info = method_info("toolchain.info").unwrap();
        assert!(info.read_only && !info.installation);
        assert_eq!(info.agent, Agent::Tool);
        assert_eq!(method_info("project.check_name").unwrap().agent, Agent::No);
        assert!(method_info("nada").is_none());
    }

    #[test]
    fn the_method_list_matches_the_request_enum() {
        // Cada nome da lista é um método que o enum aceita (com parâmetros
        // vazios ou inválidos, o erro é de parâmetros, não de método), e
        // cada variante do schema está na lista.
        let schema = serde_json::to_value(request_schema()).unwrap();
        let mut from_schema: Vec<String> = schema["oneOf"]
            .as_array()
            .expect("oneOf")
            .iter()
            .filter_map(|v| {
                v["properties"]["method"]["const"]
                    .as_str()
                    .map(str::to_owned)
            })
            .collect();
        from_schema.sort();
        let mut listed: Vec<String> = METHODS.iter().map(|m| (*m).to_owned()).collect();
        listed.sort();
        assert_eq!(listed, from_schema);
    }

    #[test]
    fn every_method_has_a_result_in_the_schema() {
        let schema = serde_json::to_value(result_schema()).unwrap();
        let results = schema["properties"].as_object().expect("properties");
        let methods: Vec<&str> = results.keys().map(String::as_str).collect();
        assert_eq!(methods, METHODS, "um resultado por método, na ordem");
        assert_eq!(results["flow.run"]["$ref"], "#/$defs/FlowOutcome");
        assert_eq!(results["update.check"]["$ref"], "#/$defs/UpdateReport");
        assert_eq!(results["install.add"]["$ref"], "#/$defs/InstallReport");
        assert_eq!(results["learn.check"]["$ref"], "#/$defs/LearnCheckOutcome");
        assert_eq!(results["learn.tracks"]["$ref"], "#/$defs/LearnTracks");
        assert_eq!(results["learn.status"]["$ref"], "#/$defs/WorkspaceSnapshot");
        assert_eq!(results["shutdown"]["type"], "null");
        assert_eq!(results["processor.outputs"]["type"], "array");
    }

    #[test]
    fn every_result_has_a_shape_in_the_schema() {
        // Nenhum método devolve JSON solto: o resultado de cada um tem um
        // tipo no schema (um `serde_json::Value` sairia como `true`).
        let schema = serde_json::to_value(result_schema()).unwrap();
        for (method, result) in schema["properties"].as_object().expect("properties") {
            let loose = *result == serde_json::Value::Bool(true)
                || result
                    .as_object()
                    .is_some_and(|o| o.keys().all(|k| k == "description" || k == "title"));
            assert!(!loose, "{method} devolve JSON sem formato: {result}");
        }
    }

    #[test]
    fn a_result_from_another_crate_serializes_as_that_crate_does() {
        // O resultado tipado sai igual ao tipo do instalador serializado
        // direto, que é o `--json` do `aurora install`.
        let report = InstallReport {
            prefix: "/opt/aurora".into(),
            components: vec!["yanc".into(), "icarus".into()],
            added: vec!["icarus".into()],
        };
        assert_eq!(
            serde_json::to_value(Response::Install(Box::new(report.clone()))).unwrap(),
            serde_json::to_value(&report).unwrap()
        );
    }

    #[test]
    fn responses_serialize_as_the_value_itself() {
        let value = serde_json::to_value(Response::Values(vec![1, 2])).unwrap();
        assert_eq!(value, serde_json::json!([1, 2]));
        let value = serde_json::to_value(Response::Nothing(())).unwrap();
        assert!(value.is_null());
        let value = serde_json::to_value(Response::Name(NameCheck {
            ok: false,
            error: Some(ErrorInfo::new("invalid_name", "x")),
        }))
        .unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "invalid_name");
    }

    #[test]
    fn the_schemas_cover_the_request_and_the_results() {
        let request = serde_json::to_value(request_schema()).unwrap();
        let defs = request["$defs"].as_object().expect("defs");
        for name in [
            "FlowRequest",
            "NewProcessor",
            "ListPosition",
            "Cleanup",
            "WaveRequest",
        ] {
            assert!(
                defs.contains_key(name),
                "{name} falta no schema dos pedidos"
            );
        }
        // Na vista de quem desserializa, os campos com padrão são opcionais.
        let spec = &defs["NewProcessor"];
        let required: Vec<&str> = spec["required"]
            .as_array()
            .map(|r| r.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        assert_eq!(required, ["name", "language"], "{spec}");

        let response = serde_json::to_value(response_schema()).unwrap();
        let defs = response["$defs"].as_object().expect("defs");
        for name in [
            "FlowOutcome",
            "ProjectStatus",
            "ToolchainInfo",
            "Notification",
            "ErrorInfo",
            "ServerInfo",
            "WaveOpened",
        ] {
            assert!(
                defs.contains_key(name),
                "{name} falta no schema das respostas"
            );
        }
    }
}
