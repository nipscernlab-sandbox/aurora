//! `aurora serve`: a API do orquestrador para outro programa, por stdin e
//! stdout, em JSON-RPC 2.0 com uma mensagem por linha.
//!
//! É o transporte do contrato de `aurora_protocol` para quem não é Rust:
//! a extensão do VS Code, um agente, um script. Cada linha de entrada é um
//! pedido (`{"jsonrpc":"2.0","id":1,"method":"project.status","params":
//! {...}}`) ou a notificação `$/cancel`; cada linha de saída é uma resposta
//! (`result` ou `error`) ou uma notificação do que acontece enquanto um
//! pedido roda (`phase`, `events`, `output_omitted`, `build`, `install`,
//! `shutdown`), com o `id` do pedido em `params.id`. O stdout é só do
//! protocolo; o log vai para o stderr.
//!
//! Não há "projeto atual": cada pedido traz o caminho do projeto, e o
//! projeto e o bundle são abertos a cada pedido, como a CLI e o Studio
//! fazem. Os pedidos do mesmo projeto rodam um de cada vez, na ordem, num
//! worker daquele projeto; os que só leem (`project.status`,
//! `history.list`) rodam em paralelo, numa thread própria; os que mexem na
//! instalação (`update.*`, `install.add`) num worker só. Cancelar é mandar
//! `$/cancel` com o `id`: um pedido que ainda não começou responde
//! `-32001`; um fluxo em andamento para e devolve o resultado com `status:
//! cancelled`, como a CLI faz com o Ctrl+C.
//!
//! Quando a entrada fecha, o que já foi pedido roda até o fim e é
//! respondido, e o servidor sai depois, com a notificação `shutdown`; se o
//! cliente fechou também a saída, o que roda é cancelado. Com `shutdown`,
//! com um sinal e depois de um `update.apply` que trocou os arquivos (o
//! binário dele é outro), o servidor cancela o que está rodando, espera até
//! 3 s e sai, com a notificação `shutdown` antes.
//!
//! Um parâmetro que o método não tem é recusado (`-32602`), com o nome mais
//! parecido, em vez de ignorado; no `flow.run`, também um campo que o fluxo
//! pedido não tem. Um BOM no começo da linha é ignorado: o PowerShell o põe
//! na primeira coisa que escreve para um programa. Os métodos `fpga.*`
//! exigem o recurso experimental `fpga` ligado, como na CLI e no Studio, e
//! um projeto cujo caminho não existe responde `spf_missing`.
//!
//! `update.*` e `install.add` rodam a biblioteca do instalador em processo,
//! como a CLI e o Studio; o progresso sai como notificação `install`, com o
//! evento do instalador em `params.event`.
//!
//! Códigos JSON-RPC: `-32700` JSON inválido, `-32600` pedido malformado,
//! `-32601` método desconhecido, `-32602` parâmetros inválidos, `-32000` um
//! erro da AURORA (o `ErrorInfo` em `error.data`, com `code` e `hint`),
//! `-32001` cancelado antes de começar, `-32002` sessão não inicializada,
//! `-32003` servidor encerrando.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, Write};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender, SyncSender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use aurora_core::delivery::{Dispatcher, Limits, Notification, trim_step_output};
use aurora_core::experimental::Feature;
use aurora_core::flows::{self, FlowOptions, FlowRequest};
use aurora_core::fpga::{self, FpgaConfig};
use aurora_core::history;
use aurora_core::locate;
use aurora_core::virtual_board::{self, BoardInput, BoardLayout};
use aurora_core::{AuroraError, CancelToken, Control, ErrorInfo, NewProcessor, Project, Toolchain};
use aurora_installer::add::{self as installer_add, Payload};
use aurora_installer::install::Event as InstallEvent;
use aurora_installer::payload::Index;
use aurora_installer::release::{Bar, Remote};
use aurora_installer::update::{
    AppliedUpdate, ChildOutput, Hooks, Prepared as UpdatePrepared, Reopen, RunOptions,
    UpdateAction, UpdateReport,
};
use aurora_protocol::{
    METHODS, NameCheck, PROTOCOL_VERSION, ProcessorRemoval, Request, Response, ServerInfo,
    ShutdownReason, WaveLayoutInfo,
};
use camino::{Utf8Path, Utf8PathBuf};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::settings::ToolchainArgs;

/// Quanto esperar os pedidos em andamento depois de cancelá-los, ao
/// encerrar.
const SHUTDOWN_WAIT: Duration = Duration::from_secs(3);

/// O prazo de cada consulta de versão do `update.check`, o mesmo que o Studio
/// usava: uma rede parada não segura a resposta.
const CHECK_TIMEOUT: Duration = Duration::from_secs(90);

/// Os códigos de erro do JSON-RPC que o servidor usa.
pub(crate) mod rpc {
    pub const PARSE: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const AURORA: i64 = -32000;
    pub const CANCELLED: i64 = -32001;
    pub const NOT_INITIALIZED: i64 = -32002;
    pub const SHUTTING_DOWN: i64 = -32003;

    /// Cada código e quando sai, para o docs/PROTOCOL.md.
    #[cfg(test)]
    pub const ALL: &[(i64, &str)] = &[
        (PARSE, "a linha não é JSON"),
        (INVALID_REQUEST, "o JSON não é um pedido JSON-RPC 2.0"),
        (METHOD_NOT_FOUND, "método desconhecido"),
        (INVALID_PARAMS, "parâmetros que o método não aceita"),
        (
            AURORA,
            "a AURORA não conseguiu atender: o `ErrorInfo` vai em `error.data`",
        ),
        (CANCELLED, "cancelado com `$/cancel` antes de começar"),
        (NOT_INITIALIZED, "pedido antes do `initialize`"),
        (SHUTTING_DOWN, "pedido depois do `shutdown`"),
    ];
}

/// As opções de `aurora serve`.
#[derive(clap::Args, Debug, Clone)]
pub struct ServeArgs {
    /// Print the JSON Schema of the protocol (requests and responses) and exit
    #[arg(long)]
    pub schema: bool,
    /// In an MCP session, also expose update_apply and install_add, which change the AURORA installation
    #[arg(long)]
    pub allow_update: bool,
}

/// `aurora serve`. `cancel` é o pedido dos sinais, que aqui encerra o
/// servidor do jeito certo: cancela o que roda, espera e sai.
///
/// O primeiro pedido diz o idioma da sessão: o `initialize` do MCP traz
/// `protocolVersion`, e o do protocolo da AURORA não traz. Uma sessão MCP vai
/// para o tradutor do `aurora mcp`, com `project_dir` (o `-C` global) como o
/// projeto padrão das ferramentas; o idioma MCP é o do assistente de IA e
/// exige o recurso experimental `assistant`.
pub fn run(
    project_dir: &Utf8Path,
    toolchain: &ToolchainArgs,
    args: &ServeArgs,
    cancel: &CancelToken,
) -> anyhow::Result<bool> {
    if args.schema {
        let schema = json!({
            "request": aurora_protocol::request_schema(),
            "response": aurora_protocol::response_schema(),
        });
        println!("{}", serde_json::to_string_pretty(&schema)?);
        return Ok(true);
    }
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let first = first_line(&mut input);
    if let Some(line) = first.as_deref().filter(|line| is_mcp_initialize(line)) {
        if !aurora_core::experimental::is_enabled(Feature::Assistant) {
            refuse_mcp(line);
            return Err(AuroraError::ExperimentalDisabled {
                feature: Feature::Assistant,
            }
            .into());
        }
        return crate::mcp::serve_session(
            project_dir,
            toolchain,
            args.allow_update,
            cancel,
            first,
            input,
        );
    }
    let server = Arc::new(Server::new(toolchain.clone(), Writer::stdout()));
    let signalled = cancel.clone();
    let watcher = Arc::clone(&server);
    thread::Builder::new()
        .name("aurora-serve-signals".into())
        .spawn(move || {
            while !signalled.is_cancelled() {
                thread::sleep(Duration::from_millis(50));
            }
            watcher.finish(ShutdownReason::Signal);
            std::process::exit(0);
        })?;
    let ended = match &first {
        Some(line) => server.handle_line(line).or_else(|| server.serve(input)),
        None => None,
    };
    match ended {
        Some(reason) => server.finish(reason),
        // A entrada fechou: o que já foi pedido roda até o fim e é
        // respondido.
        None => server.finish_after_input(),
    }
    Ok(true)
}

/// A primeira linha com conteúdo da entrada; `None` se ela fecha antes.
fn first_line(input: &mut impl BufRead) -> Option<String> {
    let mut line = String::new();
    loop {
        line.clear();
        match input.read_line(&mut line) {
            Ok(0) | Err(_) => return None,
            Ok(_) if line.trim().is_empty() => {}
            Ok(_) => return Some(line.trim_end_matches(['\r', '\n']).to_owned()),
        }
    }
}

/// A linha é o `initialize` de um cliente MCP: ele traz `protocolVersion`,
/// e o do protocolo da AURORA não traz.
fn is_mcp_initialize(line: &str) -> bool {
    let line = line.strip_prefix('\u{feff}').unwrap_or(line);
    serde_json::from_str::<Value>(line)
        .is_ok_and(|v| v["method"] == "initialize" && v["params"]["protocolVersion"].is_string())
}

/// Responde ao `initialize` de um cliente MCP com o recurso `assistant`
/// desligado: o erro, com a dica, no idioma dele, para o agente mostrar.
fn refuse_mcp(line: &str) {
    let line = line.strip_prefix('\u{feff}').unwrap_or(line);
    let id = serde_json::from_str::<Value>(line)
        .ok()
        .and_then(|v| v.get("id").cloned())
        .unwrap_or(Value::Null);
    let info = crate::output::error_info(&AuroraError::ExperimentalDisabled {
        feature: Feature::Assistant,
    });
    let message = match &info.hint {
        Some(hint) => format!("{}. {hint}", info.message),
        None => info.message.clone(),
    };
    let writer = Writer::stdout();
    writer.fail(
        Some(&id),
        rpc::AURORA,
        &message,
        serde_json::to_value(&info).ok(),
    );
    writer.drain(SHUTDOWN_WAIT);
}

/// Um pedido como chega, antes de virar um [`Request`].
#[derive(Deserialize)]
struct Incoming {
    #[serde(default)]
    id: Option<Value>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    params: Option<Value>,
}

/// O que vai para a thread do stdout: uma linha, ou uma marca que ela
/// confirma quando chega nela, com tudo o que veio antes já escrito.
enum Out {
    Line(String),
    Barrier(mpsc::Sender<()>),
}

/// O escritor único do stdout: uma thread, uma linha por mensagem, `flush`
/// a cada uma. Quem manda só espera a fila, nunca o cliente.
#[derive(Clone)]
pub(crate) struct Writer {
    tx: SyncSender<Out>,
    /// Sem notificações: o `aurora mcp` usa o despacho do servidor, mas o
    /// stdout dele é do MCP, que não entende as notificações da AURORA.
    quiet: bool,
    /// O cliente fechou a saída: não há mais a quem responder.
    closed: Arc<AtomicBool>,
}

impl Writer {
    /// Um escritor cujas linhas vão para um canal: os testes leem dali.
    #[cfg(test)]
    pub(crate) fn channel() -> (Self, mpsc::Receiver<String>) {
        let (tx, rx) = mpsc::sync_channel::<Out>(1024);
        let (lines, received) = mpsc::sync_channel::<String>(1024);
        thread::spawn(move || {
            for out in rx {
                match out {
                    Out::Line(line) => {
                        if lines.send(line).is_err() {
                            break;
                        }
                    }
                    Out::Barrier(ack) => {
                        let _ = ack.send(());
                    }
                }
            }
        });
        let writer = Writer {
            tx,
            quiet: false,
            closed: Arc::default(),
        };
        (writer, received)
    }

    /// Um escritor que não escreve nada: para quem só usa [`Server::handle`].
    pub(crate) fn silent() -> Self {
        let (tx, _rx) = mpsc::sync_channel::<Out>(1);
        Writer {
            tx,
            quiet: true,
            closed: Arc::default(),
        }
    }

    pub(crate) fn stdout() -> Self {
        let (tx, rx) = mpsc::sync_channel::<Out>(1024);
        let closed = Arc::new(AtomicBool::new(false));
        let gone = Arc::clone(&closed);
        thread::Builder::new()
            .name("aurora-serve-stdout".into())
            .spawn(move || {
                let stdout = std::io::stdout();
                for out in rx {
                    match out {
                        Out::Line(line) => {
                            let mut handle = stdout.lock();
                            if handle.write_all(line.as_bytes()).is_err()
                                || handle.write_all(b"\n").is_err()
                                || handle.flush().is_err()
                            {
                                // O cliente fechou a saída: não há mais a
                                // quem falar.
                                gone.store(true, Ordering::SeqCst);
                                break;
                            }
                        }
                        Out::Barrier(ack) => {
                            let _ = ack.send(());
                        }
                    }
                }
            })
            .expect("spawn the stdout thread");
        Writer {
            tx,
            quiet: false,
            closed,
        }
    }

    pub(crate) fn send(&self, value: &Value) {
        if let Ok(line) = serde_json::to_string(value) {
            let _ = self.tx.send(Out::Line(line));
        }
    }

    /// Espera a thread do stdout escrever o que já está na fila, até
    /// `wait`. Sem isso, um processo que sai logo depois da última resposta
    /// pode sair antes de ela chegar ao cliente.
    pub(crate) fn drain(&self, wait: Duration) {
        let (ack, done) = mpsc::channel();
        if self.tx.send(Out::Barrier(ack)).is_ok() {
            let _ = done.recv_timeout(wait);
        }
    }

    /// O cliente fechou a saída.
    pub(crate) fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    fn respond(&self, id: &Value, result: Value) {
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "result": result }));
    }

    fn fail(&self, id: Option<&Value>, code: i64, message: &str, data: Option<Value>) {
        let mut error = json!({ "code": code, "message": message });
        if let Some(data) = data {
            error["data"] = data;
        }
        let id = id.cloned().unwrap_or(Value::Null);
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "error": error }));
    }

    fn notify(&self, method: &str, params: Value) {
        if self.quiet {
            return;
        }
        self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    /// Uma notificação do core sobre o pedido `id`: o `type` vira o método,
    /// e o resto vai em `params` com o `id`.
    fn notify_for(&self, id: &Value, notification: &Notification) {
        let Ok(Value::Object(mut fields)) = serde_json::to_value(notification) else {
            return;
        };
        let method = fields
            .remove("type")
            .and_then(|t| t.as_str().map(str::to_owned))
            .unwrap_or_else(|| "notification".to_owned());
        fields.insert("id".to_owned(), id.clone());
        self.notify(&method, Value::Object(fields));
    }
}

/// Um pedido aceito e ainda não respondido.
struct Pending {
    cancel: CancelToken,
    /// Já começou a rodar: cancelar é pelo token; antes disso, o worker o
    /// pula e responde `-32001`.
    started: Arc<AtomicBool>,
    /// É um `update.check`, que só consulta.
    check_only: bool,
}

type Job = Box<dyn FnOnce() + Send>;

/// Por que um pedido não deu certo: um erro do core (que ganha a dica da
/// CLI ao sair) ou um já em forma de [`ErrorInfo`].
pub(crate) enum Failure {
    Aurora(Box<AuroraError>),
    Info(Box<ErrorInfo>),
}

impl From<AuroraError> for Failure {
    fn from(error: AuroraError) -> Self {
        Failure::Aurora(Box::new(error))
    }
}

impl From<ErrorInfo> for Failure {
    fn from(info: ErrorInfo) -> Self {
        Failure::Info(Box::new(info))
    }
}

impl From<aurora_git::Error> for Failure {
    fn from(error: aurora_git::Error) -> Self {
        let mut info = ErrorInfo::new(error.code(), error.to_string());
        info.data = error.data();
        if matches!(error, aurora_git::Error::GitMissing) {
            info = info.with_hint("Install Git from https://git-scm.com/downloads");
        }
        Failure::Info(Box::new(info))
    }
}

impl From<aurora_installer::Error> for Failure {
    fn from(error: aurora_installer::Error) -> Self {
        Failure::Info(Box::new(ErrorInfo::new(error.code(), error.to_string())))
    }
}

impl Failure {
    pub(crate) fn into_info(self) -> ErrorInfo {
        match self {
            Failure::Aurora(error) => crate::output::error_info(&error),
            Failure::Info(info) => *info,
        }
    }
}

pub(crate) type Handled = Result<Response, Failure>;

/// Para onde vão as notificações de um pedido enquanto ele roda: no `serve`,
/// o stdout com o `id` do pedido; no `aurora mcp`, o progresso do MCP.
pub(crate) type Emit = Arc<dyn Fn(Notification) + Send + Sync>;

/// O servidor: o estado da sessão e as filas.
pub(crate) struct Server {
    toolchain: ToolchainArgs,
    writer: Writer,
    initialized: AtomicBool,
    /// Pedidos novos são recusados (`-32003`).
    shutting_down: AtomicBool,
    /// O encerramento já foi feito (a notificação, o cancelamento, a
    /// espera): só uma vez.
    finished: AtomicBool,
    /// Sair quando o pedido que está rodando terminar: o `update.apply`
    /// trocou o binário deste servidor.
    exit_when_done: AtomicBool,
    pending: Mutex<HashMap<String, Pending>>,
    workers: Mutex<HashMap<String, Sender<Job>>>,
    /// As placas virtuais rodando (`vboard.run`), pelo id do pedido: por onde
    /// vão as entradas de `vboard.input` e a placa, para conferi-las.
    boards: Mutex<HashMap<String, (Sender<BoardInput>, BoardLayout)>>,
    /// Se um recurso experimental está ligado: o `experimental.json` do
    /// usuário, ou o que o teste disser.
    features: fn(Feature) -> bool,
}

impl Server {
    pub(crate) fn new(toolchain: ToolchainArgs, writer: Writer) -> Self {
        Server {
            toolchain,
            writer,
            initialized: AtomicBool::new(false),
            shutting_down: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            exit_when_done: AtomicBool::new(false),
            pending: Mutex::new(HashMap::new()),
            workers: Mutex::new(HashMap::new()),
            boards: Mutex::new(HashMap::new()),
            features: aurora_core::experimental::is_enabled,
        }
    }

    /// O mesmo servidor, com outra regra para os recursos experimentais.
    #[cfg(test)]
    pub(crate) fn with_features(mut self, features: fn(Feature) -> bool) -> Self {
        self.features = features;
        self
    }

    /// Confere um recurso experimental antes de atender um método dele,
    /// como a CLI e o Studio fazem.
    fn require(&self, feature: Feature) -> Result<(), Failure> {
        if (self.features)(feature) {
            Ok(())
        } else {
            Err(AuroraError::ExperimentalDisabled { feature }.into())
        }
    }

    /// Lê a entrada até ela fechar ou até um pedido encerrar a sessão.
    /// `Some` com o motivo, no segundo caso; `None` quando a entrada fechou.
    fn serve(self: &Arc<Self>, input: impl BufRead) -> Option<ShutdownReason> {
        for line in input.lines() {
            let Ok(line) = line else {
                break;
            };
            if line.trim().is_empty() {
                continue;
            }
            if let Some(reason) = self.handle_line(&line) {
                return Some(reason);
            }
        }
        None
    }

    /// Uma linha da entrada. `Some` quando a sessão termina por causa dela.
    fn handle_line(self: &Arc<Self>, line: &str) -> Option<ShutdownReason> {
        // O PowerShell põe um BOM na primeira coisa que escreve para um
        // programa, e o JSON não tem BOM.
        let line = line.strip_prefix('\u{feff}').unwrap_or(line);
        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                self.writer
                    .fail(None, rpc::PARSE, &format!("Invalid JSON: {error}"), None);
                return None;
            }
        };
        if !value.is_object() {
            self.writer.fail(
                None,
                rpc::INVALID_REQUEST,
                "A request is a JSON object (batches are not supported)",
                None,
            );
            return None;
        }
        let incoming: Incoming = match serde_json::from_value(value) {
            Ok(incoming) => incoming,
            Err(error) => {
                self.writer
                    .fail(None, rpc::INVALID_REQUEST, &error.to_string(), None);
                return None;
            }
        };
        let Some(method) = incoming.method else {
            self.writer.fail(
                incoming.id.as_ref(),
                rpc::INVALID_REQUEST,
                "The request has no method",
                None,
            );
            return None;
        };
        if method == "$/cancel" {
            if let Some(id) = incoming.params.as_ref().and_then(|p| p.get("id")) {
                self.cancel(id);
            }
            return None;
        }
        let Some(id) = incoming.id else {
            // Uma notificação que o servidor não conhece: ignorada, como o
            // JSON-RPC manda.
            tracing::debug!("Ignoring the notification {method}");
            return None;
        };
        if self.shutting_down.load(Ordering::SeqCst) {
            self.writer.fail(
                Some(&id),
                rpc::SHUTTING_DOWN,
                "The server is shutting down",
                None,
            );
            return None;
        }
        if method != "initialize" && !self.initialized.load(Ordering::SeqCst) {
            self.writer.fail(
                Some(&id),
                rpc::NOT_INITIALIZED,
                "Send initialize first",
                None,
            );
            return None;
        }
        if let Some(message) = unknown_params(&method, incoming.params.as_ref()) {
            self.writer
                .fail(Some(&id), rpc::INVALID_PARAMS, &message, None);
            return None;
        }
        let request: Request = match parse_request(&method, incoming.params) {
            Ok(request) => request,
            Err(error) => {
                let (code, message) = if METHODS.contains(&method.as_str()) {
                    (
                        rpc::INVALID_PARAMS,
                        format!("Invalid params for {method}: {error}"),
                    )
                } else {
                    (rpc::METHOD_NOT_FOUND, format!("Unknown method {method}"))
                };
                self.writer.fail(Some(&id), code, &message, None);
                return None;
            }
        };
        match request {
            Request::Initialize { client } => {
                self.initialized.store(true, Ordering::SeqCst);
                tracing::info!(client = client.as_deref().unwrap_or("?"), "Session started");
                let info = serde_json::to_value(self.info()).unwrap_or(Value::Null);
                self.writer.respond(&id, info);
                None
            }
            Request::Shutdown => {
                self.shutting_down.store(true, Ordering::SeqCst);
                self.writer.respond(&id, Value::Null);
                Some(ShutdownReason::Requested)
            }
            request => {
                self.dispatch(id, request);
                None
            }
        }
    }

    /// Põe o pedido na fila certa.
    fn dispatch(self: &Arc<Self>, id: Value, request: Request) {
        let cancel = CancelToken::new();
        let started = Arc::new(AtomicBool::new(false));
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                id.to_string(),
                Pending {
                    cancel: cancel.clone(),
                    started: Arc::clone(&started),
                    check_only: matches!(request, Request::UpdateCheck),
                },
            );
        let lane = lane_of(&request);
        // O corte da saída dos passos é do transporte, como no `aurora mcp`: o
        // resultado continua o `FlowOutcome`, e só o JSON sai mais curto.
        let step_output_limit = match &request {
            Request::FlowRun {
                step_output_limit, ..
            } => *step_output_limit,
            _ => None,
        };
        let server = Arc::clone(self);
        let job: Job = Box::new(move || {
            if cancel.is_cancelled() {
                server.writer.fail(
                    Some(&id),
                    rpc::CANCELLED,
                    "Cancelled before it started",
                    None,
                );
            } else {
                started.store(true, Ordering::SeqCst);
                match server.handle(&id, request, cancel) {
                    Ok(response) => {
                        let mut value = serde_json::to_value(&response).unwrap_or(Value::Null);
                        if let Some(limit) = step_output_limit {
                            trim_step_output(&mut value, limit);
                        }
                        server.writer.respond(&id, value);
                    }
                    Err(failure) => {
                        let info = failure.into_info();
                        let data = serde_json::to_value(&info).ok();
                        server
                            .writer
                            .fail(Some(&id), rpc::AURORA, &info.message, data);
                    }
                }
            }
            server
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&id.to_string());
            if server.exit_when_done.load(Ordering::SeqCst) {
                server.finish(ShutdownReason::Updated);
                std::process::exit(0);
            }
        });
        match lane {
            Lane::Thread => {
                if let Err(error) = thread::Builder::new()
                    .name("aurora-serve-read".into())
                    .spawn(job)
                {
                    tracing::error!("Could not start a thread: {error}");
                }
            }
            Lane::Worker(key) => self.enqueue(&key, job),
        }
    }

    /// Entrega um trabalho ao worker `key`, criando-o se preciso.
    fn enqueue(&self, key: &str, job: Job) {
        let mut workers = self
            .workers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let sender = workers.entry(key.to_owned()).or_insert_with(|| {
            let (tx, rx) = mpsc::channel::<Job>();
            let tail: Vec<char> = key.chars().rev().take(20).collect();
            let short: String = tail.into_iter().rev().collect();
            thread::Builder::new()
                .name(format!("aurora-serve-{short}"))
                .spawn(move || {
                    for job in rx {
                        job();
                    }
                })
                .expect("spawn a worker thread");
            tx
        });
        if sender.send(job).is_err() {
            tracing::error!("The worker {key} is gone");
        }
    }

    /// `$/cancel`: marca o token do pedido, rodando ou na fila.
    fn cancel(&self, id: &Value) {
        let pending = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(entry) = pending.get(&id.to_string()) {
            entry.cancel.cancel();
            tracing::info!(id = %id, started = entry.started.load(Ordering::SeqCst), "Cancelled");
        }
    }

    /// Encerra por um pedido (`shutdown`), por um sinal ou depois de uma
    /// atualização: avisa, cancela o que roda, espera até [`SHUTDOWN_WAIT`]
    /// e espera a thread do stdout escrever o que falta. Só uma vez.
    fn finish(&self, reason: ShutdownReason) {
        self.shutting_down.store(true, Ordering::SeqCst);
        if self.finished.swap(true, Ordering::SeqCst) {
            return;
        }
        let reason = serde_json::to_value(reason).unwrap_or(Value::Null);
        self.writer.notify("shutdown", json!({ "reason": reason }));
        self.cancel_all();
        self.wait_pending(SHUTDOWN_WAIT);
        self.writer.drain(SHUTDOWN_WAIT);
    }

    /// Encerra porque a entrada fechou: o que já foi pedido roda até o fim
    /// e é respondido, como num `printf ... | aurora serve`. Se o cliente
    /// fechou também a saída, não há a quem responder, e o que roda é
    /// cancelado.
    fn finish_after_input(&self) {
        self.shutting_down.store(true, Ordering::SeqCst);
        while !self.pending_is_empty() {
            if self.writer.is_closed() {
                self.cancel_all();
                self.wait_pending(SHUTDOWN_WAIT);
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        self.finish(ShutdownReason::Requested);
    }

    fn cancel_all(&self) {
        for entry in self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
        {
            entry.cancel.cancel();
        }
    }

    fn pending_is_empty(&self) -> bool {
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty()
    }

    fn wait_pending(&self, wait: Duration) {
        let until = Instant::now() + wait;
        while Instant::now() < until && !self.pending_is_empty() {
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn info(&self) -> ServerInfo {
        let experimental: BTreeMap<String, bool> = aurora_core::experimental::states()
            .into_iter()
            .map(|s| (s.feature.id().to_owned(), s.enabled))
            .collect();
        ServerInfo {
            aurora: env!("CARGO_PKG_VERSION").to_owned(),
            protocol: PROTOCOL_VERSION,
            prefix: crate::installation::prefix().ok(),
            experimental,
            methods: METHODS.iter().map(|m| (*m).to_owned()).collect(),
        }
    }

    /// O bundle, para as operações. Um compilador ou um Quartus declarado
    /// (`--compiler`, `--quartus`) que não serve não derruba o bundle, como
    /// no Studio: a sessão segue sem ele, a operação que o exige falha com o
    /// erro dela (`quartus_missing`), e o `toolchain.info` mostra o motivo.
    /// A CLI, que roda um comando só, recusa logo
    /// ([`ToolchainArgs::resolve`]).
    fn toolchain(&self) -> Result<Toolchain, Failure> {
        let lookup = self.toolchain.lookup().map_err(|e| to_info(&e))?;
        Ok(locate::resolve(&lookup)?.toolchain)
    }

    /// Atende um pedido. `Err` é a AURORA não conseguir rodar.
    pub(crate) fn handle(&self, id: &Value, request: Request, cancel: CancelToken) -> Handled {
        self.handle_with(id, request, cancel, None)
    }

    /// [`Server::handle`] com as notificações do pedido indo para `sink` em
    /// vez do stdout (o `aurora mcp`).
    pub(crate) fn handle_with(
        &self,
        id: &Value,
        request: Request,
        cancel: CancelToken,
        sink: Option<Emit>,
    ) -> Handled {
        let emit: Emit = match sink {
            Some(sink) => sink,
            None => {
                let writer = self.writer.clone();
                let id = id.clone();
                Arc::new(move |notification| writer.notify_for(&id, &notification))
            }
        };
        Ok(match request {
            Request::Initialize { .. } | Request::Shutdown => Response::Nothing(()),
            Request::ProjectCreate { parent, name } => {
                let project = Project::create(&parent, &name)?;
                Response::Status(Box::new(project.status()))
            }
            Request::ProjectStatus { project } => {
                Response::Status(Box::new(open_project(&project)?.status()))
            }
            Request::ProjectAdd {
                project,
                files,
                testbench,
            } => {
                let mut project = open_project(&project)?;
                // Sem bundle o arquivo é criado do mesmo jeito, com o leitor
                // de portas embutido.
                let toolchain = self.toolchain().ok();
                Response::Added(project.add_hdl_files(toolchain.as_ref(), &files, testbench)?)
            }
            Request::ProjectRemove { project, files } => {
                Response::Removed(open_project(&project)?.remove_hdl_files(&files)?)
            }
            Request::ProjectTop { project, target } => {
                let mut project = open_project(&project)?;
                if let Some(target) = target {
                    project.set_top(&target)?;
                }
                Response::Top(crate::output::top_outcome(&project))
            }
            Request::ProjectTestbench { project, path } => {
                let mut project = open_project(&project)?;
                project.set_testbench(&path)?;
                Response::Status(Box::new(project.status()))
            }
            Request::ProjectOrder {
                project,
                file,
                position,
            } => {
                let mut project = open_project(&project)?;
                Response::Files(project.reorder_file(&file, &position)?)
            }
            Request::ProjectMove {
                project,
                from,
                to,
                after,
                overwrite,
            } => {
                let mut project = open_project(&project)?;
                let moved = if after {
                    if overwrite {
                        return Err(ErrorInfo::new(
                            "invalid_argument",
                            "after and overwrite do not go together: the move already happened",
                        )
                        .into());
                    }
                    project.record_move(&from, &to)?
                } else {
                    move_replacing(&mut project, &from, &to, overwrite)?
                };
                Response::Moved(moved)
            }
            Request::ProjectConvert { project } => {
                let mut project = open_project(&project)?;
                Response::Converted(project.convert_layout()?)
            }
            Request::ProjectCheckName { name } => {
                Response::Name(name_check(aurora_core::validate_project_name(&name)))
            }
            Request::ProcessorCheckName { name } => {
                // A mesma regra do processor.add: a do YANC e, fora dela,
                // os nomes das pastas da AURORA.
                Response::Name(name_check(aurora_core::validate_new_processor_name(&name)))
            }
            Request::ProcessorAdd { project, spec } => {
                let mut project = open_project(&project)?;
                let processor = project.add_processor(&spec)?.clone();
                Response::Processor(Box::new(processor))
            }
            Request::ProcessorSet {
                project,
                name,
                config,
            } => {
                let mut project = open_project(&project)?;
                project.configure_processor(&name, &config)?;
                let processor = project.require_processor(&name)?.clone();
                Response::Processor(Box::new(processor))
            }
            Request::ProcessorRemove {
                project,
                name,
                to_trash,
            } => {
                let mut project = open_project(&project)?;
                let removed = project.remove_processor(&name)?;
                let trashed = to_trash && removed.dir.exists();
                if trashed {
                    trash::delete(&removed.dir).map_err(|error| {
                        ErrorInfo::new(
                            "io",
                            format!(
                                "{name} is out of the project, but its folder did not go to the trash ({error}): {}",
                                removed.dir
                            ),
                        )
                    })?;
                }
                Response::ProcessorRemoved(ProcessorRemoval { removed, trashed })
            }
            Request::ProcessorOutputs {
                project,
                processor,
                port,
            } => {
                let project = open_project(&project)?;
                let processor = project.require_processor(&processor)?;
                Response::Values(processor.read_output_values(port)?)
            }
            Request::ProcessorDefaults => {
                Response::Defaults(Box::new(NewProcessor::new("", aurora_core::Language::Cmm)))
            }
            Request::FlowRun {
                project,
                request,
                options,
                output,
                // Cortado ao escrever a resposta (`dispatch`).
                step_output_limit: _,
            } => {
                let options =
                    options.unwrap_or_else(|| FlowOptions::new(request.command_line("aurora")));
                // A correção de um exercício é do aurora-learn, que o core não
                // conhece: roda aqui, com o resultado de um fluxo.
                if let FlowRequest::Learn { root, exercise } = &request {
                    emit(Notification::Phase {
                        phase: aurora_core::flows::Phase::Learn,
                    });
                    let outcome = self.learn_flow(root, exercise, &options.command, cancel)?;
                    return Ok(Response::Flow(Box::new(outcome)));
                }
                let mut project = open_project(&project)?;
                let deliver = Arc::clone(&emit);
                let dispatcher =
                    Dispatcher::start(Limits::default(), move |notification| deliver(notification));
                let control = if output {
                    dispatcher.control(cancel)
                } else {
                    Control::new().with_cancel(cancel)
                };
                let progress = dispatcher.progress();
                // O processador que não existe e o projeto sem nada a
                // compilar vêm antes do bundle (flows::run_with).
                let outcome = flows::run_with(
                    &mut project,
                    &request,
                    &options,
                    || self.toolchain(),
                    &progress,
                    &control,
                );
                // Tudo que o fluxo disse sai antes do resultado.
                dispatcher.finish();
                Response::Flow(Box::new(outcome?))
            }
            Request::FlowCommand { request, program } => {
                Response::Text(request.command_line(program.as_deref().unwrap_or("aurora")))
            }
            Request::WaveOpen { project, request } => {
                let project = project.map(|p| open_project(&p)).transpose()?;
                // A onda de um exercício do aurora learn abre com o layout da
                // correção, como no `aurora wave`; a onda que ainda não existe é
                // erro antes do bundle (flows::open_wave_with).
                let request = aurora_learn::layout::with_exercise_layout(request);
                Response::Wave(flows::open_wave_with(project.as_ref(), &request, || {
                    self.toolchain()
                })?)
            }
            Request::WaveSignals { project } => {
                let project = open_project(&project)?;
                let toolchain = self.toolchain()?;
                let control = Control::new().with_cancel(cancel);
                Response::Signals(Box::new(aurora_core::wave_signals(
                    &toolchain, &project, &control,
                )?))
            }
            Request::WaveChoose { project, choice } => {
                let project = open_project(&project)?;
                let toolchain = self.toolchain()?;
                let control = Control::new().with_cancel(cancel);
                Response::Choice(flows::choose_signals(
                    &toolchain, &project, &choice, &control,
                )?)
            }
            Request::WaveLayoutGet { waveform } => {
                let (layout, layout_error) = match aurora_core::wave_layout(&waveform) {
                    Ok(layout) => (layout, None),
                    Err(error) => (None, Some(error.to_string())),
                };
                // O layout salvo é um extra: se não der para ler ou gravar,
                // a onda abre com o gerado, como no `wave.open`.
                let saved = layout.as_ref().and_then(|layout| {
                    aurora_core::saved_layout(&waveform, layout)
                        .inspect_err(|e| tracing::warn!("No saved layout for {waveform}: {e}"))
                        .ok()
                        .flatten()
                });
                Response::Layout(Box::new(WaveLayoutInfo {
                    exercise_layout: aurora_learn::layout::layout_of(&waveform),
                    layout,
                    saved,
                    layout_error,
                }))
            }
            Request::VerilogModules { text } => {
                Response::Modules(aurora_core::verilog::modules_in(&text))
            }
            Request::VerilogTestbenchNames { names } => Response::Flags(
                names
                    .iter()
                    .map(|name| aurora_core::verilog::is_testbench_name(name))
                    .collect(),
            ),
            Request::WaveResetLayout { waveform } => {
                let Ok(project) = Project::discover(&waveform) else {
                    return Ok(Response::Flag(false));
                };
                let Some(testbench) = aurora_core::wave_testbench_of(&waveform)? else {
                    return Ok(Response::Flag(false));
                };
                Response::Flag(aurora_core::reset_saved_layout(&project, &testbench)?)
            }
            Request::HistoryList { project, limit } => {
                let project = open_project(&project)?;
                let mut reports = history::list(&project)?;
                if let Some(limit) = limit {
                    reports.truncate(limit);
                }
                Response::Reports(reports)
            }
            Request::HistoryShow { project, id } => {
                let project = open_project(&project)?;
                Response::Report(Box::new(history::view(&project, id.as_deref())?))
            }
            Request::HistoryCompare {
                project,
                id,
                against,
            } => {
                let project = open_project(&project)?;
                Response::Comparison(Box::new(history::compare_reports(
                    &project,
                    id.as_deref(),
                    against.as_deref(),
                )?))
            }
            Request::HistoryPlanCleanup { project, cleanup } => {
                let project = open_project(&project)?;
                Response::Ids(history::plan_cleanup(&project, &cleanup)?)
            }
            Request::HistoryClean { project, ids } => {
                Response::Cleaned(history::clean(&open_project(&project)?, &ids)?)
            }
            Request::ToolchainInfo => {
                // Um bundle que não abre vem em `found: false`, e um
                // compilador ou Quartus declarado que não serve, nos campos de
                // erro.
                let lookup = self.toolchain.lookup().map_err(|e| to_info(&e))?;
                let info = match locate::resolve(&lookup) {
                    Ok(found) => locate::describe(&found),
                    Err(error) => locate::describe_error(&error),
                };
                Response::Toolchain(Box::new(info))
            }
            Request::ToolchainVerify => Response::Mismatches(self.toolchain()?.verify()?),
            Request::ExperimentalList => Response::Experimental(aurora_core::experimental::info()),
            Request::ExperimentalSet { feature, enabled } => {
                let feature =
                    aurora_core::experimental::Feature::from_id(&feature).ok_or_else(|| {
                        ErrorInfo::new(
                            "invalid_argument",
                            format!("Unknown experimental component: {feature}"),
                        )
                    })?;
                aurora_core::experimental::set_enabled(feature, enabled)?;
                Response::Experimental(aurora_core::experimental::info())
            }
            Request::FpgaBoards { board } => {
                self.require(Feature::Fpga)?;
                Response::Boards(match board {
                    Some(id) => vec![fpga::board(&id)?],
                    None => fpga::boards()?,
                })
            }
            Request::FpgaConfigGet { project } => {
                self.require(Feature::Fpga)?;
                let project = open_project(&project)?;
                let path = FpgaConfig::path(project.root());
                Response::FpgaConfig(if path.is_file() {
                    Some(FpgaConfig::read(project.root())?)
                } else {
                    None
                })
            }
            Request::FpgaConfigSet { project, config } => {
                self.require(Feature::Fpga)?;
                let project = open_project(&project)?;
                config.write(project.root())?;
                Response::Nothing(())
            }
            Request::FpgaConnect {
                project,
                board,
                change,
            } => {
                self.require(Feature::Fpga)?;
                let project = open_project(&project)?;
                let toolchain = self.toolchain().ok();
                Response::FpgaConnected(Box::new(fpga::connect(
                    toolchain.as_ref(),
                    &project,
                    board.as_deref(),
                    &change,
                )?))
            }
            Request::FpgaCheck { project } => {
                self.require(Feature::Fpga)?;
                let project = open_project(&project)?;
                let toolchain = self.toolchain().ok();
                Response::Prepared(Box::new(fpga::prepare(toolchain.as_ref(), &project)?))
            }
            Request::FpgaModules { project } => {
                self.require(Feature::Fpga)?;
                let project = open_project(&project)?;
                let toolchain = self.toolchain().ok();
                Response::Modules(fpga::modules(toolchain.as_ref(), &project)?)
            }
            Request::FpgaStatus { project } => {
                self.require(Feature::Fpga)?;
                let toolchain = self.toolchain()?;
                let project = open_project(&project)?;
                Response::Bitstream(fpga::bitstream_state(&toolchain, &project)?)
            }
            Request::FpgaCables => {
                self.require(Feature::Fpga)?;
                let toolchain = self.toolchain()?;
                Response::Cables(fpga::cables(&toolchain)?)
            }
            Request::FpgaTop { project, top } => {
                self.require(Feature::Fpga)?;
                let project = open_project(&project)?;
                // Sem bundle, as portas vêm do leitor embutido.
                let toolchain = self.toolchain().ok();
                Response::Interface(Box::new(fpga::top_interface(
                    toolchain.as_ref(),
                    &project,
                    top.as_deref(),
                )?))
            }
            Request::VboardLayout { project, board } => {
                self.require(Feature::Fpga)?;
                let project = open_project(&project)?;
                let toolchain = self.toolchain().ok();
                let mut options = virtual_board::BuildOptions::default();
                options.board = board;
                Response::VboardLayout(Box::new(virtual_board::layout(
                    toolchain.as_ref(),
                    &project,
                    &options,
                )?))
            }
            Request::VboardRun {
                project,
                board,
                limit,
                fps,
            } => {
                self.require(Feature::Fpga)?;
                let project = open_project(&project)?;
                let toolchain = self.toolchain()?;
                let deliver = Arc::clone(&emit);
                let dispatcher =
                    Dispatcher::start(Limits::default(), move |notification| deliver(notification));
                let mut options = virtual_board::BuildOptions::default();
                options.board = board;
                let built = virtual_board::build(
                    &toolchain,
                    &project,
                    &options,
                    &dispatcher.control(cancel.clone()),
                )?;
                let Some(layout) = built.layout.clone().filter(|_| built.succeeded()) else {
                    dispatcher.finish();
                    return Ok(Response::Vboard(Box::new(virtual_board::BoardRun::new(
                        built, None,
                    ))));
                };
                let (inputs, received) = std::sync::mpsc::channel();
                let key = id.to_string();
                self.boards
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(key.clone(), (inputs, layout.clone()));
                let mut on_event = |event: virtual_board::SessionEvent| {
                    let notification = match event {
                        virtual_board::SessionEvent::Ready => Notification::VboardReady {
                            layout: layout.clone(),
                        },
                        virtual_board::SessionEvent::State(state) => {
                            Notification::VboardState { state }
                        }
                        virtual_board::SessionEvent::Frame(frame) => {
                            Notification::VboardFrame { frame }
                        }
                        virtual_board::SessionEvent::Output { lines, omitted } => {
                            Notification::VboardOutput { lines, omitted }
                        }
                        _ => return,
                    };
                    dispatcher.notify(notification);
                };
                let options =
                    virtual_board::RunOptions::new(limit.unwrap_or(1.0), fps.unwrap_or(30.0));
                let outcome = virtual_board::run(
                    &toolchain,
                    &project,
                    &built,
                    &options,
                    &received,
                    &mut on_event,
                    &Control::new().with_cancel(cancel),
                );
                self.boards
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .remove(&key);
                dispatcher.finish();
                Response::Vboard(Box::new(virtual_board::BoardRun::new(
                    built,
                    Some(outcome?),
                )))
            }
            Request::VboardInput { run, input } => {
                self.require(Feature::Fpga)?;
                let boards = self
                    .boards
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let Some((inputs, layout)) = boards.get(&run.to_string()) else {
                    return Err(ErrorInfo::new(
                        "vboard_not_running",
                        format!("No virtual board is running with request id {run}"),
                    )
                    .into());
                };
                layout.check(&input)?;
                let _ = inputs.send(input);
                Response::Nothing(())
            }
            Request::UpdateCheck => Response::Update(Box::new(
                self.update(id, &emit, false, None, None, &cancel)?,
            )),
            Request::UpdateApply {
                full,
                from,
                reopen_ide,
            } => {
                let reopen = reopen_ide.then_some(Reopen::Studio);
                let report = self.update(id, &emit, full, from.as_deref(), reopen, &cancel)?;
                Response::Update(Box::new(report))
            }
            Request::InstallAdd { components } => {
                // Um nome errado aparece antes de qualquer download, como no
                // `aurora install`.
                if components.is_empty() {
                    return Err(ErrorInfo::new("invalid_argument", "No app to install").into());
                }
                if let Some(message) = aurora_core::component::unknown_message(&components) {
                    return Err(ErrorInfo::new("invalid_argument", message).into());
                }
                let network = crate::update::network().map_err(cli_error)?;
                let remote = std::sync::Arc::new(Remote(network.clone()));
                let prefix = crate::installation::prefix_result()?;
                aurora_installer::update::recover_interrupted(&prefix)?;
                aurora_installer::update::clean_leftovers(prefix.as_std_path());
                let prepared = installer_add::prepare(
                    remote,
                    &network,
                    &prefix,
                    Payload::Release {
                        version: env!("CARGO_PKG_VERSION"),
                    },
                    &cancel,
                )?;
                let report = installer_add::add_components(
                    &prepared,
                    &prefix,
                    &components,
                    &mut |event| emit(install_notification(&event, Some(&prepared.index))),
                    &cancel,
                )?;
                Response::Install(Box::new(report))
            }
            Request::LearnCheck {
                root,
                exercise,
                all,
            } => {
                let toolchain = self.toolchain()?;
                let mut workspace = aurora_learn::Workspace::find(&root, Some(&toolchain), None)
                    .map_err(|e| learn_info(&e))?;
                let exercises: Vec<aurora_learn::Exercise> = if all {
                    workspace.track().exercises().cloned().collect()
                } else {
                    vec![
                        workspace
                            .exercise(exercise.as_deref())
                            .map_err(|e| learn_info(&e))?,
                    ]
                };
                let control = Control::new().with_cancel(cancel);
                let outcome = aurora_learn::flows::check(
                    &toolchain,
                    &mut workspace,
                    &exercises,
                    &control,
                    &mut |_| {},
                )
                .map_err(|e| learn_info(&e))?;
                Response::LearnCheck(Box::new(outcome))
            }
            Request::LearnTracks => {
                let toolchain = self.toolchain().ok();
                Response::Tracks(
                    aurora_learn::tracks(toolchain.as_ref()).map_err(|e| learn_info(&e))?,
                )
            }
            Request::LearnStatus { root, lang } => {
                let workspace = self.workspace(&root, lang.as_deref())?;
                Response::Workspace(Box::new(workspace.snapshot()))
            }
            Request::LearnInit { dir, track, lang } => {
                let toolchain = self.toolchain().ok();
                let tracks =
                    aurora_learn::tracks_dir(toolchain.as_ref()).map_err(|e| learn_info(&e))?;
                let id = track.unwrap_or_else(|| aurora_learn::DEFAULT_TRACK.to_owned());
                let track = aurora_learn::load_track(&tracks, &id, lang.as_deref())
                    .map_err(|e| learn_info(&e))?;
                let workspace =
                    aurora_learn::Workspace::init(&dir, track).map_err(|e| learn_info(&e))?;
                Response::Workspace(Box::new(workspace.snapshot()))
            }
            Request::LearnSetCurrent { root, name, lang } => {
                let mut workspace = self.workspace(&root, lang.as_deref())?;
                workspace.set_current(&name).map_err(|e| learn_info(&e))?;
                Response::Workspace(Box::new(workspace.snapshot()))
            }
            Request::LearnReset { root, name } => {
                let workspace = self.workspace(&root, None)?;
                let exercise = workspace
                    .track()
                    .require(&name)
                    .map_err(|e| learn_info(&e))?
                    .clone();
                workspace.reset(&exercise).map_err(|e| learn_info(&e))?;
                Response::Nothing(())
            }
            Request::GitStatus { project } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                Response::GitStatus(Box::new(aurora_git::status(&git_tool()?, project.root())?))
            }
            Request::GitDiff { project, path } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                Response::Text(aurora_git::diff(
                    &git_tool()?,
                    project.root(),
                    path.as_deref(),
                )?)
            }
            Request::GitReadFile {
                project,
                revision,
                path,
            } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                Response::FileVersion(Box::new(aurora_git::read_file(
                    &git_tool()?,
                    project.root(),
                    &revision,
                    &path,
                )?))
            }
            Request::GitCommit {
                project,
                summary,
                description,
                paths,
                amend,
            } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                let mut options = aurora_git::CommitOptions::new(summary);
                options.description = description;
                options.paths = paths;
                options.amend = amend;
                Response::Commit(Box::new(aurora_git::commit(
                    &git_tool()?,
                    project.root(),
                    &options,
                )?))
            }
            Request::GitDiscard { project, paths } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                Response::Discarded(Box::new(aurora_git::discard(
                    &git_tool()?,
                    project.root(),
                    &paths,
                )?))
            }
            Request::GitInit { project } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                Response::GitInit(Box::new(aurora_git::init(&git_tool()?, &project)?))
            }
            Request::GitLog {
                project,
                skip,
                limit,
            } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                Response::GitLog(Box::new(aurora_git::log(
                    &git_tool()?,
                    project.root(),
                    skip.unwrap_or(0),
                    limit.unwrap_or(50),
                )?))
            }
            Request::GitCommitFiles { project, oid } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                Response::CommitFiles(Box::new(aurora_git::commit_files(
                    &git_tool()?,
                    project.root(),
                    &oid,
                )?))
            }
            Request::GitBranches { project } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                Response::Branches(Box::new(aurora_git::branches(
                    &git_tool()?,
                    project.root(),
                )?))
            }
            Request::GitSwitch {
                project,
                branch,
                create,
            } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                let options = if create {
                    aurora_git::SwitchOptions::create(branch)
                } else {
                    aurora_git::SwitchOptions::to(branch)
                };
                Response::Text(aurora_git::switch(&git_tool()?, project.root(), &options)?)
            }
            Request::GitFetch { project } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                Response::Sync(Box::new(aurora_git::fetch(
                    &git_tool()?,
                    project.root(),
                    Some(&cancel),
                )?))
            }
            Request::GitPull { project } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                Response::Sync(Box::new(aurora_git::pull(
                    &git_tool()?,
                    project.root(),
                    Some(&cancel),
                )?))
            }
            Request::GitPush { project } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                Response::Sync(Box::new(aurora_git::push(
                    &git_tool()?,
                    project.root(),
                    Some(&cancel),
                )?))
            }
            Request::GitSetIdentity {
                project,
                name,
                email,
                global,
            } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                let identity = aurora_git::Identity::new(name, email);
                aurora_git::set_identity(&git_tool()?, project.root(), &identity, global)?;
                Response::Nothing(())
            }
            Request::GitSetRemote { project, name, url } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                aurora_git::set_remote(&git_tool()?, project.root(), &name, &url)?;
                Response::Nothing(())
            }
            Request::GitRemoveRemote { project, name } => {
                self.require(Feature::Git)?;
                let project = open_project(&project)?;
                aurora_git::remove_remote(&git_tool()?, project.root(), &name)?;
                Response::Nothing(())
            }
            Request::GithubAccount => {
                self.require(Feature::Git)?;
                let account = aurora_git::github::account(&git_tool()?, &curl_tool()?)?;
                Response::Account(account.map(Box::new))
            }
            Request::GithubLoginStart => {
                self.require(Feature::Git)?;
                Response::DeviceCode(Box::new(aurora_git::github::login_start(
                    &curl_tool()?,
                    &github_client_id(),
                )?))
            }
            Request::GithubLoginPoll { device_code } => {
                self.require(Feature::Git)?;
                Response::LoginPoll(Box::new(aurora_git::github::login_poll(
                    &git_tool()?,
                    &curl_tool()?,
                    &github_client_id(),
                    &device_code,
                )?))
            }
            Request::GithubLogout => {
                self.require(Feature::Git)?;
                Response::Flag(aurora_git::github::logout(&git_tool()?)?)
            }
        })
    }

    /// `update.check` e `update.apply` pela biblioteca do instalador, com o
    /// progresso como notificação `install`. Depois de uma atualização que
    /// trocou os arquivos, o servidor encerra assim que responder: o binário
    /// dele é outro.
    fn update(
        &self,
        id: &Value,
        emit: &Emit,
        full: bool,
        from: Option<&Utf8Path>,
        reopen: Option<Reopen>,
        cancel: &CancelToken,
    ) -> Result<UpdateReport, Failure> {
        let check = from.is_none() && !full && self.check_only(id);
        let mut network = crate::update::network().map_err(cli_error)?;
        if check {
            // Só consulta versões: cada transferência tem prazo, para uma
            // rede parada não segurar a resposta.
            network.timeout = Some(CHECK_TIMEOUT);
        }
        let remote = std::sync::Arc::new(Remote(network));
        let toolchain = self.toolchain().ok();
        let reg = crate::update::reg_exe();
        let options = RunOptions {
            check,
            full,
            from,
            reopen,
            bar: Bar::Quiet,
            child_output: ChildOutput::Capture,
            reg: reg.as_deref(),
        };
        let index: Rc<RefCell<Option<Index>>> = Rc::default();
        let mut labels = Labels {
            index: Rc::clone(&index),
        };
        let report = aurora_installer::update::run(
            &remote,
            env!("CARGO_PKG_VERSION"),
            toolchain.as_ref().map(Toolchain::manifest),
            crate::installation::prefix_result(),
            &options,
            &mut labels,
            &mut |event| emit(install_notification(&event, index.borrow().as_ref())),
            cancel,
        )?;
        if matches!(
            report.action,
            UpdateAction::Updated | UpdateAction::WizardOpened
        ) {
            // O binário deste servidor foi (ou vai ser) trocado: a sessão
            // termina depois desta resposta.
            self.shutting_down.store(true, Ordering::SeqCst);
            self.exit_when_done.store(true, Ordering::SeqCst);
        }
        Ok(report)
    }

    /// A pasta de exercícios que contém `root`, aberta e posta em dia com a
    /// trilha, com as trilhas de onde o servidor as acha.
    fn workspace(
        &self,
        root: &Utf8Path,
        lang: Option<&str>,
    ) -> Result<aurora_learn::Workspace, Failure> {
        let toolchain = self.toolchain().ok();
        aurora_learn::Workspace::find(root, toolchain.as_ref(), lang)
            .map_err(|e| learn_info(&e).into())
    }

    /// A correção de um exercício como fluxo (`flow.run` com `learn`): o
    /// resultado de um fluxo, com a correção em `learn`, cancelado ou falho
    /// pelo veredito. A saída das ferramentas não vai ao vivo: o resumo do
    /// testbench é para a correção ler.
    fn learn_flow(
        &self,
        root: &Utf8Path,
        exercise: &str,
        command: &str,
        cancel: CancelToken,
    ) -> Result<aurora_core::flows::FlowOutcome, Failure> {
        let toolchain = self.toolchain()?;
        let mut workspace = aurora_learn::Workspace::find(root, Some(&toolchain), None)
            .map_err(|e| learn_info(&e))?;
        let exercise = workspace
            .exercise(Some(exercise))
            .map_err(|e| learn_info(&e))?;
        let control = Control::new().with_cancel(cancel);
        let checked = aurora_learn::flows::check(
            &toolchain,
            &mut workspace,
            std::slice::from_ref(&exercise),
            &control,
            &mut |_| {},
        )
        .map_err(|e| learn_info(&e))?;
        let grade = checked.results.into_iter().next();
        let mut outcome = aurora_core::flows::FlowOutcome::new("learn", command);
        match &grade {
            Some(grade) if grade.verdict == aurora_learn::Verdict::Cancelled => {
                outcome.worsen(aurora_core::Status::Cancelled);
            }
            Some(grade) if !grade.solved() => outcome.worsen(aurora_core::Status::Failed),
            _ => {}
        }
        outcome.learn = grade.map(|g| json!(g));
        Ok(outcome)
    }

    /// O pedido em andamento com `id` é um `update.check` (e não um
    /// `update.apply`): a distinção vem do método, guardado ao despachar.
    fn check_only(&self, id: &Value) -> bool {
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&id.to_string())
            .is_some_and(|p| p.check_only)
    }
}

/// A notificação `install` com o evento do instalador e a linha do console,
/// com os rótulos de `index` quando já se sabe a versão.
fn install_notification(event: &InstallEvent, index: Option<&Index>) -> Notification {
    Notification::Install {
        event: serde_json::to_value(event).unwrap_or(Value::Null),
        line: event.console_line(&|name| aurora_installer::install::label(index, name)),
    }
}

/// Guarda o índice da versão nova, para os rótulos das linhas do console.
struct Labels {
    index: Rc<RefCell<Option<Index>>>,
}

impl Hooks for Labels {
    fn plan(&mut self, prepared: &UpdatePrepared) {
        *self.index.borrow_mut() = Some(prepared.index.clone());
    }

    fn applied(&mut self, _prepared: &UpdatePrepared, applied: &AppliedUpdate, _prefix: &Utf8Path) {
        if let Some(warning) = &applied.registry_warning {
            tracing::info!("{warning}");
        }
    }
}

/// Um erro da CLI (`anyhow`) como [`Failure`], com o código da biblioteca
/// do instalador quando é dela.
fn cli_error(error: anyhow::Error) -> Failure {
    Failure::Info(Box::new(to_info(&error)))
}

/// O [`Request`] de um método e seus parâmetros. Um método sem parâmetros
/// aceita `params` ausente ou `{}` (um cliente MCP manda `{}`), e um cujos
/// parâmetros têm todos padrão (`initialize`) aceita `params` ausente.
pub(crate) fn parse_request(
    method: &str,
    params: Option<Value>,
) -> Result<Request, serde_json::Error> {
    let envelope = |params: Option<Value>| {
        let mut envelope = json!({ "method": method });
        if let Some(params) = params {
            envelope["params"] = params;
        }
        envelope
    };
    let empty = params
        .as_ref()
        .is_none_or(|p| p.as_object().is_some_and(|o| o.is_empty()));
    match serde_json::from_value(envelope(params.clone())) {
        Ok(request) => Ok(request),
        Err(error) if empty => {
            // A outra forma de "sem parâmetros".
            let other = if params.is_none() {
                Some(json!({}))
            } else {
                None
            };
            serde_json::from_value(envelope(other)).map_err(|_| error)
        }
        Err(error) => Err(error),
    }
}

/// Onde um pedido roda.
enum Lane {
    /// Uma thread só para ele: só lê.
    Thread,
    /// O worker com esta chave, um pedido de cada vez.
    Worker(String),
}

fn lane_of(request: &Request) -> Lane {
    if request.is_read_only() {
        return Lane::Thread;
    }
    // A placa virtual roda até ser cancelada, e as entradas dela chegam
    // enquanto isso: nenhuma das duas espera na fila do projeto, nem a
    // segura.
    if matches!(
        request,
        Request::VboardRun { .. } | Request::VboardInput { .. }
    ) {
        return Lane::Thread;
    }
    if request.is_installation() {
        return Lane::Worker("install".to_owned());
    }
    match request.project() {
        Some(project) => Lane::Worker(project_key(project)),
        None => Lane::Worker("general".to_owned()),
    }
}

/// A chave do worker de um projeto: a raiz canônica dele.
fn project_key(project: &Utf8Path) -> String {
    let dir = if project.extension() == Some("spf") {
        project.parent().unwrap_or(project)
    } else {
        project
    };
    dunce::canonicalize(dir)
        .ok()
        .and_then(|p| Utf8PathBuf::from_path_buf(p).ok())
        .unwrap_or_else(|| dir.to_owned())
        .to_string()
}

fn name_check(result: Result<(), AuroraError>) -> NameCheck {
    match result {
        Ok(()) => NameCheck {
            ok: true,
            error: None,
        },
        Err(error) => NameCheck {
            ok: false,
            error: Some(crate::output::error_info(&error)),
        },
    }
}

/// Move `from` para `to` no projeto. Com `overwrite`, o que está no destino
/// vai para a lixeira do sistema e o movimento é tentado de novo: o core
/// nunca sobrescreve. Menos o que o core mantém no lugar, a pasta do projeto
/// e uma pasta que contém a origem ([`Project::replace_refusal`]): aí o erro é
/// `cannot_replace`, e nada vai para a lixeira.
fn move_replacing(
    project: &mut Project,
    from: &Utf8Path,
    to: &Utf8Path,
    overwrite: bool,
) -> Result<aurora_core::MovedPath, Failure> {
    match project.move_path(from, to) {
        Err(AuroraError::PathExists(target)) if overwrite => {
            if let Some(reason) = project.replace_refusal(from, &target) {
                return Err(ErrorInfo::new(
                    "cannot_replace",
                    format!("{target} is not replaced: {reason}"),
                )
                .into());
            }
            trash::delete(&target)
                .map_err(|error| ErrorInfo::new("io", format!("{target}: {error}")))?;
            Ok(project.move_path(from, to)?)
        }
        result => Ok(result?),
    }
}

/// As pastas onde o git e o `curl` da máquina são procurados: as do `PATH`
/// e, no Windows, depois delas, as do instalador do Git for Windows, para
/// quem o instalou sem pô-lo no `PATH`. A AURORA não traz git no bundle.
fn system_dirs() -> Vec<Utf8PathBuf> {
    let mut dirs: Vec<Utf8PathBuf> = std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path)
                .filter_map(|d| Utf8PathBuf::from_path_buf(d).ok())
                .collect()
        })
        .unwrap_or_default();
    if cfg!(windows) {
        let base = |var: &str| {
            std::env::var_os(var).and_then(|v| Utf8PathBuf::from_path_buf(v.into()).ok())
        };
        if let Some(programs) = base("ProgramFiles") {
            dirs.push(programs.join("Git").join("cmd"));
        }
        if let Some(local) = base("LOCALAPPDATA") {
            dirs.push(local.join("Programs").join("Git").join("cmd"));
        }
    }
    dirs
}

/// O git da máquina, achado uma vez por servidor (o `git --version` custa
/// um processo). Sem git, procura de novo no próximo pedido: a pessoa pode
/// instalá-lo com a AURORA aberta.
fn git_tool() -> Result<aurora_git::Git, Failure> {
    static FOUND: OnceLock<aurora_git::Git> = OnceLock::new();
    if let Some(git) = FOUND.get() {
        return Ok(git.clone());
    }
    let git = aurora_git::Git::locate(&system_dirs())?;
    Ok(FOUND.get_or_init(|| git).clone())
}

fn curl_tool() -> Result<aurora_git::github::Curl, Failure> {
    Ok(aurora_git::github::Curl::locate(&system_dirs())?)
}

/// O aplicativo OAuth do login no GitHub: o da AURORA, ou o de
/// `AURORA_GITHUB_CLIENT_ID`, para testar com outro.
fn github_client_id() -> String {
    std::env::var("AURORA_GITHUB_CLIENT_ID")
        .ok()
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| aurora_git::github::DEFAULT_CLIENT_ID.to_owned())
}

/// Abre o projeto de um pedido. Um caminho que não existe é
/// `spf_missing`, o código que o Studio já usava para um `.spf` que sumiu
/// depois de aberto (apagado, movido por fora).
fn open_project(path: &Utf8Path) -> Result<Project, Failure> {
    Project::open(path).map_err(|error| {
        if path.exists() {
            Failure::from(error)
        } else {
            ErrorInfo::new(
                "spf_missing",
                format!(
                    "The project file {path} does not exist anymore: without it AURORA cannot build, simulate or change the project"
                ),
            )
            .into()
        }
    })
}

/// Os campos de `params` de cada método e os de cada fluxo do `flow.run`,
/// tirados do schema dos pedidos.
struct Fields {
    methods: HashMap<String, Vec<String>>,
    flows: HashMap<String, Vec<String>>,
}

fn fields() -> &'static Fields {
    static FIELDS: OnceLock<Fields> = OnceLock::new();
    FIELDS.get_or_init(|| {
        let schema = serde_json::to_value(aurora_protocol::request_schema()).unwrap_or(Value::Null);
        let names = |node: &Value| -> Vec<String> {
            node.as_object()
                .map(|p| p.keys().cloned().collect())
                .unwrap_or_default()
        };
        let mut methods = HashMap::new();
        for entry in schema["oneOf"].as_array().into_iter().flatten() {
            if let Some(method) = entry["properties"]["method"]["const"].as_str() {
                methods.insert(
                    method.to_owned(),
                    names(&entry["properties"]["params"]["properties"]),
                );
            }
        }
        let mut flows = HashMap::new();
        for entry in schema["$defs"]["FlowRequest"]["oneOf"]
            .as_array()
            .into_iter()
            .flatten()
        {
            if let Some(flow) = entry["properties"]["flow"]["const"].as_str() {
                flows.insert(flow.to_owned(), names(&entry["properties"]));
            }
        }
        Fields { methods, flows }
    })
}

/// Os campos de `params` de um método; `None` se o método não existe.
pub(crate) fn method_fields(method: &str) -> Option<&'static [String]> {
    fields().methods.get(method).map(Vec::as_slice)
}

/// Os campos de um fluxo do `flow.run`, com o `flow`; `None` se o fluxo
/// não existe.
pub(crate) fn flow_fields(flow: &str) -> Option<&'static [String]> {
    fields().flows.get(flow).map(Vec::as_slice)
}

/// A recusa de um campo que não está em `known`, com o mais parecido. Um
/// campo escrito errado seria ignorado em silêncio, e o pedido rodaria com
/// o padrão no lugar do que o cliente quis.
pub(crate) fn unknown_field(given: &Map<String, Value>, known: &[String]) -> Option<String> {
    let unknown = given.keys().find(|key| !known.contains(key))?;
    let mut message = format!("unknown field `{unknown}`");
    if let Some(near) = closest(unknown, known) {
        message.push_str(&format!(" (did you mean `{near}`?)"));
    }
    if known.is_empty() {
        message.push_str("; this method takes no params");
    } else {
        message.push_str(&format!("; the fields are: {}", known.join(", ")));
    }
    Some(message)
}

/// A recusa dos parâmetros de um pedido com um campo que o método não
/// tem; no `flow.run`, também um campo que o fluxo pedido não tem.
pub(crate) fn unknown_params(method: &str, params: Option<&Value>) -> Option<String> {
    let given = params?.as_object()?;
    let known = method_fields(method)?;
    if let Some(problem) = unknown_field(given, known) {
        return Some(format!("Invalid params for {method}: {problem}"));
    }
    let request = given.get("request")?.as_object()?;
    let flow = request.get("flow")?.as_str()?;
    let known = flow_fields(flow)?;
    unknown_field(request, known)
        .map(|problem| format!("Invalid params for {method}, flow {flow}: {problem}"))
}

/// O nome de `candidates` mais parecido com `word`, se for parecido o
/// bastante para ser um erro de digitação.
fn closest<'a>(word: &str, candidates: &'a [String]) -> Option<&'a str> {
    candidates
        .iter()
        .map(|candidate| (distance(word, candidate), candidate))
        .filter(|(d, _)| *d <= 3 && *d < word.chars().count())
        .min_by_key(|(d, _)| *d)
        .map(|(_, candidate)| candidate.as_str())
}

/// A distância de edição entre `a` e `b`.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            let value = (row[j + 1] + 1).min(row[j] + 1).min(diagonal + cost);
            diagonal = row[j + 1];
            row[j + 1] = value;
        }
    }
    row[b.len()]
}

/// Um erro da CLI (`anyhow`) como [`ErrorInfo`]: o do core que houver na
/// cadeia, com a dica, senão `cli` com o texto inteiro.
fn to_info(error: &anyhow::Error) -> ErrorInfo {
    if let Some(aurora) = error.chain().find_map(|e| e.downcast_ref::<AuroraError>()) {
        return crate::output::error_info(aurora);
    }
    if let Some(installer) = error
        .chain()
        .find_map(|e| e.downcast_ref::<aurora_installer::Error>())
    {
        return ErrorInfo::new(installer.code(), installer.to_string());
    }
    if error.chain().any(|e| e.is::<crate::cancel::Cancelled>()) {
        return ErrorInfo::new("cancelled", error.to_string());
    }
    if let Some(learn) = error
        .chain()
        .find_map(|e| e.downcast_ref::<aurora_learn::LearnError>())
    {
        return learn_info(learn);
    }
    ErrorInfo::new("cli", format!("{error:#}"))
}

/// Um erro do aurora-learn como [`ErrorInfo`], com o código e a dica da CLI.
fn learn_info(error: &aurora_learn::LearnError) -> ErrorInfo {
    let (code, hint) = crate::learn::error_info(error);
    let mut info = ErrorInfo::new(code, error.to_string());
    info.hint = hint;
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server() -> (Arc<Server>, mpsc::Receiver<String>) {
        // Os recursos experimentais ligados: os testes não dependem do
        // `experimental.json` de quem os roda.
        server_with(|_| true)
    }

    fn server_with(features: fn(Feature) -> bool) -> (Arc<Server>, mpsc::Receiver<String>) {
        let (writer, rx) = Writer::channel();
        let args = ToolchainArgs {
            toolchain: None,
            compiler: None,
            quartus: None,
        };
        (
            Arc::new(Server::new(args, writer).with_features(features)),
            rx,
        )
    }

    fn next(rx: &mpsc::Receiver<String>) -> Value {
        let line = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("uma mensagem");
        serde_json::from_str(&line).expect("JSON")
    }

    #[test]
    fn rejects_bad_json_malformed_requests_and_unknown_methods() {
        let (server, rx) = server();
        assert!(server.handle_line("{nope").is_none());
        assert_eq!(next(&rx)["error"]["code"], rpc::PARSE);
        assert!(server.handle_line("[1,2]").is_none());
        assert_eq!(next(&rx)["error"]["code"], rpc::INVALID_REQUEST);
        assert!(server.handle_line(r#"{"jsonrpc":"2.0","id":1}"#).is_none());
        let reply = next(&rx);
        assert_eq!(reply["error"]["code"], rpc::INVALID_REQUEST);
        assert_eq!(reply["id"], 1);
        // Antes do initialize, nada além dele.
        server.handle_line(r#"{"jsonrpc":"2.0","id":2,"method":"toolchain.info"}"#);
        assert_eq!(next(&rx)["error"]["code"], rpc::NOT_INITIALIZED);
        server.handle_line(
            r#"{"jsonrpc":"2.0","id":3,"method":"initialize","params":{"client":"test"}}"#,
        );
        let reply = next(&rx);
        assert_eq!(reply["id"], 3);
        assert_eq!(reply["result"]["protocol"], PROTOCOL_VERSION);
        assert!(
            reply["result"]["methods"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m == "flow.run")
        );
        server.handle_line(r#"{"jsonrpc":"2.0","id":4,"method":"nothing.here"}"#);
        assert_eq!(next(&rx)["error"]["code"], rpc::METHOD_NOT_FOUND);
        server.handle_line(r#"{"jsonrpc":"2.0","id":5,"method":"project.status","params":{}}"#);
        let reply = next(&rx);
        assert_eq!(reply["error"]["code"], rpc::INVALID_PARAMS);
        assert!(
            reply["error"]["message"]
                .as_str()
                .unwrap()
                .contains("project.status")
        );
        // Uma notificação desconhecida é ignorada em silêncio.
        server.handle_line(r#"{"jsonrpc":"2.0","method":"whatever"}"#);
        assert!(rx.recv_timeout(Duration::from_millis(100)).is_err());
        // shutdown encerra a sessão e responde null.
        assert_eq!(
            server.handle_line(r#"{"jsonrpc":"2.0","id":6,"method":"shutdown"}"#),
            Some(ShutdownReason::Requested)
        );
        assert!(next(&rx)["result"].is_null());
        server.handle_line(r#"{"jsonrpc":"2.0","id":7,"method":"toolchain.info"}"#);
        assert_eq!(next(&rx)["error"]["code"], rpc::SHUTTING_DOWN);
    }

    #[test]
    fn an_aurora_error_carries_the_error_info_in_data() {
        let (server, rx) = server();
        server.handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#);
        next(&rx);
        let missing = std::env::temp_dir().join("aurora-serve-nao-existe");
        let missing = missing.to_str().unwrap().replace('\\', "/");
        server.handle_line(&format!(
            r#"{{"jsonrpc":"2.0","id":"a","method":"project.status","params":{{"project":"{missing}"}}}}"#
        ));
        let reply = next(&rx);
        assert_eq!(reply["id"], "a");
        assert_eq!(reply["error"]["code"], rpc::AURORA);
        assert!(reply["error"]["data"]["code"].is_string(), "{reply}");
        assert_eq!(reply["error"]["message"], reply["error"]["data"]["message"]);
        // O pedido sai da lista dos pendentes depois de respondido.
        let until = Instant::now() + Duration::from_secs(2);
        while Instant::now() < until && !server.pending.lock().unwrap().is_empty() {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(server.pending.lock().unwrap().is_empty());
    }

    /// Manda o pedido, espera a resposta e confere o `result` contra o tipo
    /// que `protocol::result_schema` declara para o método.
    fn call_conforming(
        server: &Arc<Server>,
        rx: &mpsc::Receiver<String>,
        schema: &Value,
        method: &str,
        params: Value,
    ) -> Value {
        let id = format!("{method} {params}");
        server.handle_line(
            &json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string(),
        );
        let reply = loop {
            let line = next(rx);
            if line["id"] == id.as_str() {
                break line;
            }
        };
        let result = reply
            .get("result")
            .unwrap_or_else(|| panic!("{method}: {reply}"))
            .clone();
        let validator = jsonschema::validator_for(&json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$defs": schema["$defs"],
            "allOf": [schema["properties"][method]],
        }))
        .unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&result)
            .map(|e| format!("{}: {e}", e.instance_path()))
            .collect();
        assert!(
            errors.is_empty(),
            "{method} não devolve o tipo declarado no aurora-protocol:\n{}\n\n{result:#}",
            errors.join("\n")
        );
        result
    }

    /// O resultado de cada método é o tipo que `protocol::result_schema`
    /// declara, que é o que o docs/PROTOCOL.md mostra. Roda, num projeto
    /// temporário, os métodos que não precisam de simulação, de síntese, de
    /// relatório guardado, do Quartus nem da rede.
    #[test]
    fn the_virtual_board_draws_the_board_and_refuses_input_without_a_run() {
        let schema = serde_json::to_value(aurora_protocol::result_schema()).unwrap();
        let (server, rx) = server();
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dunce::canonicalize(dir.path()).unwrap()).unwrap();
        let mut project = Project::create(&root, "placa").unwrap();
        let top = project.root().join("placa.v");
        std::fs::write(
            &top,
            "module placa (input CLOCK_50, input [3:0] SW, output [3:0] LEDR);\n\
             assign LEDR = SW;\nendmodule\n",
        )
        .unwrap();
        // O primeiro módulo do projeto vira o topo.
        project.add_verilog(None, &top, false).unwrap();
        let spf = project.spf_path().to_owned();
        call_conforming(
            &server,
            &rx,
            &schema,
            "initialize",
            json!({"client": "test"}),
        );

        // Sem fpga.json: a placa pedida, com as portas ligadas pelo nome.
        let layout = call_conforming(
            &server,
            &rx,
            &schema,
            "vboard.layout",
            json!({"project": spf, "board": "de2-115"}),
        );
        let connected: Vec<&str> = layout["peripherals"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["connected"] == true)
            .map(|p| p["id"].as_str().unwrap())
            .collect();
        assert_eq!(connected, ["SW", "LEDR"], "{layout}");
        assert_eq!(layout["clocks"][0]["signal"], "CLOCK_50");

        // Uma entrada sem placa rodando com esse id.
        server.handle_line(
            &json!({"jsonrpc": "2.0", "id": 7, "method": "vboard.input",
                "params": {"run": 99, "input": {"kind": "set", "device": "SW", "value": 1}}})
            .to_string(),
        );
        let reply = loop {
            let line = next(&rx);
            if line["id"] == 7 {
                break line;
            }
        };
        assert_eq!(
            reply["error"]["data"]["code"], "vboard_not_running",
            "{reply}"
        );
    }

    #[test]
    fn results_conform_to_the_declared_schema() {
        let schema = serde_json::to_value(aurora_protocol::result_schema()).unwrap();
        let (server, rx) = server();
        let call =
            |method: &str, params: Value| call_conforming(&server, &rx, &schema, method, params);
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dunce::canonicalize(dir.path()).unwrap()).unwrap();
        let project = root.join("demo");
        let (top, tb) = (project.join("top.v"), project.join("top_tb.v"));

        call("initialize", json!({"client": "test"}));
        call("project.check_name", json!({"name": "demo"}));
        call("processor.check_name", json!({"name": "x-y"}));
        call("project.create", json!({"parent": root, "name": "demo"}));
        call("project.status", json!({"project": project}));
        call(
            "project.add",
            json!({"project": project, "files": [top, tb]}),
        );
        call("project.top", json!({"project": project}));
        call("project.testbench", json!({"project": project, "path": tb}));
        call(
            "project.order",
            json!({"project": project, "file": top, "position": {"kind": "first"}}),
        );
        let moved = project.join("rtl/top.v");
        call(
            "project.move",
            json!({"project": project, "from": top, "to": moved}),
        );
        call(
            "project.remove",
            json!({"project": project, "files": [moved]}),
        );
        call("project.convert", json!({"project": project}));
        call(
            "processor.add",
            json!({"project": project, "spec": {"name": "soma", "language": "cmm"}}),
        );
        call(
            "processor.set",
            json!({"project": project, "name": "soma", "config": {"frequency_mhz": 50}}),
        );
        std::fs::create_dir_all(project.join("soma/soma")).unwrap();
        std::fs::write(project.join("soma/soma/output_0.txt"), "1\n-2\n").unwrap();
        let values = call(
            "processor.outputs",
            json!({"project": project, "processor": "soma", "port": 0}),
        );
        assert_eq!(values, json!([1, -2]));
        call("history.list", json!({"project": project}));
        call("experimental.list", json!({}));
        call("fpga.boards", json!({}));
        assert!(call("fpga.config", json!({"project": project})).is_null());
        call(
            "fpga.config_set",
            json!({"project": project, "config": {"board": "de10-nano"}}),
        );
        call("fpga.config", json!({"project": project}));
        call(
            "wave.reset_layout",
            json!({"waveform": project.join("top_tb.fst")}),
        );
        call(
            "processor.remove",
            json!({"project": project, "name": "soma"}),
        );
        // Os métodos que tiraram o core do Studio.
        let defaults = call("processor.defaults", json!({}));
        assert_eq!(
            (defaults["name"].clone(), defaults["nubits"].clone()),
            (json!(""), json!(23))
        );
        let command = call(
            "flow.command",
            json!({
                "request": { "flow": "simulate", "processor": "soma", "simulator": "icarus", "open_wave": true },
                "program": "aurora-ide",
            }),
        );
        assert_eq!(command, "aurora-ide sim -p soma --open");
        let learn = call(
            "flow.command",
            json!({ "request": { "flow": "learn", "root": root, "exercise": "mux 2" } }),
        );
        assert_eq!(learn, "aurora learn check \"mux 2\"");
        let modules = call(
            "verilog.modules",
            json!({ "text": "module a; endmodule\n// module b;\nmodule c(); endmodule\n" }),
        );
        assert_eq!(modules, json!(["a", "c"]));
        let names = call(
            "verilog.testbench_names",
            json!({ "names": ["top_tb.v", "top.v", "tb_x.sv"] }),
        );
        assert_eq!(names, json!([true, false, true]));
        let layout = call(
            "wave.layout",
            json!({ "waveform": project.join("nao_existe.vcd") }),
        );
        assert!(
            layout["layout"].is_null() && layout["layout_error"].is_string(),
            "{layout}"
        );
        let tracks = call("learn.tracks", json!({}));
        assert!(
            tracks["tracks"]
                .as_array()
                .is_some_and(|t| t.contains(&json!("verilog"))),
            "{tracks}"
        );
        let exercises = root.join("exercicios");
        let status = call("learn.init", json!({ "dir": exercises }));
        let first = status["current"].as_str().unwrap().to_owned();
        let second = status["chapters"][0]["exercises"][1]["name"]
            .as_str()
            .unwrap()
            .to_owned();
        call("learn.status", json!({ "root": exercises }));
        let status = call(
            "learn.set_current",
            json!({ "root": exercises, "name": second }),
        );
        assert_eq!(status["current"], second.as_str());
        assert_ne!(first, second);
        call("learn.reset", json!({ "root": exercises, "name": second }));
        call("shutdown", json!({}));
    }

    #[test]
    fn cancelling_a_queued_request_answers_before_it_starts() {
        let (server, rx) = server();
        server.handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#);
        next(&rx);
        // Ocupa o worker "general" com um trabalho que espera o portão: a
        // fila é por chave, então o pedido seguinte espera atrás dele.
        let gate = Arc::new(AtomicBool::new(false));
        let g = Arc::clone(&gate);
        server.enqueue(
            "general",
            Box::new(move || {
                while !g.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(5));
                }
            }),
        );
        server.handle_line(
            r#"{"jsonrpc":"2.0","id":2,"method":"project.check_name","params":{"name":"x"}}"#,
        );
        // check_name é só leitura: roda já, fora da fila.
        let reply = next(&rx);
        assert_eq!(reply["id"], 2);
        assert_eq!(reply["result"]["ok"], true);
        // experimental.set vai para o worker "general", que está ocupado.
        server.handle_line(
            r#"{"jsonrpc":"2.0","id":3,"method":"experimental.set","params":{"feature":"nope","enabled":true}}"#,
        );
        assert!(rx.recv_timeout(Duration::from_millis(100)).is_err());
        server.handle_line(r#"{"jsonrpc":"2.0","method":"$/cancel","params":{"id":3}}"#);
        gate.store(true, Ordering::SeqCst);
        let reply = next(&rx);
        assert_eq!(reply["id"], 3);
        assert_eq!(reply["error"]["code"], rpc::CANCELLED);
    }

    #[test]
    fn notifications_carry_the_request_id_and_the_type_as_method() {
        let (writer, rx) = Writer::channel();
        writer.notify_for(
            &json!(7),
            &Notification::Phase {
                phase: flows::Phase::Build,
            },
        );
        let line = next(&rx);
        assert_eq!(line["method"], "phase");
        assert_eq!(line["params"]["id"], 7);
        assert_eq!(line["params"]["phase"], "build");
        assert!(line.get("id").is_none());
    }

    #[test]
    fn methods_without_params_parse_with_and_without_defaults() {
        assert!(matches!(
            parse_request("initialize", None),
            Ok(Request::Initialize { client: None })
        ));
        assert!(matches!(
            parse_request("toolchain.info", None),
            Ok(Request::ToolchainInfo)
        ));
        assert!(matches!(
            parse_request("fpga.boards", None),
            Ok(Request::FpgaBoards { board: None })
        ));
        // Um cliente MCP manda `{}` para uma ferramenta sem parâmetros.
        assert!(matches!(
            parse_request("toolchain.info", Some(json!({}))),
            Ok(Request::ToolchainInfo)
        ));
        assert!(matches!(
            parse_request("initialize", Some(json!({}))),
            Ok(Request::Initialize { client: None })
        ));
        assert!(parse_request("project.status", None).is_err());
        assert!(parse_request("nothing.here", None).is_err());
    }

    #[test]
    fn project_keys_ignore_the_spf_file_name() {
        let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
        let spf = dir.join("x.spf");
        assert_eq!(project_key(&spf), project_key(&dir));
    }

    fn initialized() -> (Arc<Server>, mpsc::Receiver<String>) {
        let (server, rx) = server();
        server.handle_line(r#"{"jsonrpc":"2.0","id":0,"method":"initialize"}"#);
        next(&rx);
        (server, rx)
    }

    #[test]
    fn an_unknown_param_is_refused_with_the_closest_name() {
        let (server, rx) = initialized();
        server.handle_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"project.move","params":{"project":"/p","form":"a","to":"b"}}"#,
        );
        let reply = next(&rx);
        assert_eq!(reply["error"]["code"], rpc::INVALID_PARAMS, "{reply}");
        let message = reply["error"]["message"].as_str().unwrap();
        assert!(message.contains("`form`"), "{message}");
        assert!(message.contains("did you mean `from`"), "{message}");
        // Um campo de um fluxo, dentro do pedido do flow.run.
        server.handle_line(
            r#"{"jsonrpc":"2.0","id":2,"method":"flow.run","params":{"project":"/p","request":{"flow":"simulate","simulator":"icarus","proccessor":"x"}}}"#,
        );
        let reply = next(&rx);
        assert_eq!(reply["error"]["code"], rpc::INVALID_PARAMS, "{reply}");
        let message = reply["error"]["message"].as_str().unwrap();
        assert!(message.contains("did you mean `processor`"), "{message}");
        // Um método sem parâmetros aceita `{}`, que um cliente MCP manda.
        assert!(unknown_params("toolchain.info", Some(&json!({}))).is_none());
        assert!(unknown_params("toolchain.info", Some(&json!({ "x": 1 }))).is_some());
        // O que o cliente manda certo passa.
        assert!(
            unknown_params(
                "project.move",
                Some(&json!({ "project": "/p", "from": "a", "to": "b", "after": true }))
            )
            .is_none()
        );
    }

    #[test]
    fn a_byte_order_mark_before_a_request_is_ignored() {
        let (server, rx) = server();
        server.handle_line("\u{feff}{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}");
        let reply = next(&rx);
        assert_eq!(reply["result"]["protocol"], PROTOCOL_VERSION, "{reply}");
    }

    #[test]
    fn closing_the_input_answers_what_was_already_asked() {
        let (server, rx) = server();
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dunce::canonicalize(dir.path()).unwrap()).unwrap();
        let spf = Project::create(&root, "p").unwrap().spf_path().to_owned();
        let input = format!(
            "{}\n{}\n",
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
            json!({ "jsonrpc": "2.0", "id": 2, "method": "processor.add",
                    "params": { "project": spf, "spec": { "name": "soma", "language": "cmm" } } })
        );
        assert!(server.serve(std::io::Cursor::new(input)).is_none());
        server.finish_after_input();
        let lines: Vec<Value> = rx
            .try_iter()
            .map(|l| serde_json::from_str(&l).unwrap())
            .collect();
        assert!(
            lines
                .iter()
                .any(|l| l["id"] == 2 && l["result"]["name"] == "soma"),
            "{lines:?}"
        );
        assert_eq!(lines.last().unwrap()["method"], "shutdown", "{lines:?}");
    }

    #[test]
    fn a_processor_name_is_checked_like_processor_add() {
        let (server, rx) = initialized();
        // `wave` é a pasta das escolhas de sinais: o processor.add recusa.
        server.handle_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"processor.check_name","params":{"name":"wave"}}"#,
        );
        let reply = next(&rx);
        assert_eq!(reply["result"]["ok"], false, "{reply}");
        assert_eq!(reply["result"]["error"]["code"], "invalid_name", "{reply}");
    }

    #[test]
    fn fpga_methods_need_the_experimental_component() {
        let (server, rx) = server_with(|_| false);
        server.handle_line(r#"{"jsonrpc":"2.0","id":0,"method":"initialize"}"#);
        next(&rx);
        server.handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"fpga.boards"}"#);
        let reply = next(&rx);
        assert_eq!(reply["error"]["code"], rpc::AURORA, "{reply}");
        assert_eq!(
            reply["error"]["data"]["code"], "experimental_disabled",
            "{reply}"
        );
    }

    #[test]
    fn an_app_that_is_not_in_the_bundle_is_refused_before_any_download() {
        let (server, rx) = initialized();
        server.handle_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"install.add","params":{"components":[]}}"#,
        );
        let reply = next(&rx);
        assert_eq!(
            reply["error"]["data"]["code"], "invalid_argument",
            "{reply}"
        );
        server.handle_line(
            r#"{"jsonrpc":"2.0","id":2,"method":"install.add","params":{"components":["nope"]}}"#,
        );
        let reply = next(&rx);
        assert_eq!(
            reply["error"]["data"]["code"], "invalid_argument",
            "{reply}"
        );
        let message = reply["error"]["data"]["message"].as_str().unwrap();
        assert!(message.contains("Unknown app: nope"), "{message}");
    }

    #[test]
    fn what_is_wrong_in_the_request_comes_before_the_bundle() {
        // Um bundle que não existe: o erro do pedido aparece antes dele.
        let (writer, rx) = Writer::channel();
        let args = ToolchainArgs {
            toolchain: Some(Utf8PathBuf::from("/aurora/nao/existe")),
            compiler: None,
            quartus: None,
        };
        let server = Arc::new(Server::new(args, writer).with_features(|_| true));
        server.handle_line(r#"{"jsonrpc":"2.0","id":0,"method":"initialize"}"#);
        next(&rx);
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dunce::canonicalize(dir.path()).unwrap()).unwrap();
        let mut project = Project::create(&root, "demo").unwrap();
        project
            .add_verilog(None, root.join("demo").join("demo_tb.v"), true)
            .unwrap();
        let spf = project.spf_path().to_owned();
        let ask = |id: u32, method: &str, params: Value| {
            server.handle_line(
                &json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
                    .to_string(),
            );
            next(&rx)
        };
        // Um processador que não existe, em todo fluxo que o nomeia.
        for (id, request) in [
            (1, json!({ "flow": "build", "processors": ["nada"] })),
            (2, json!({ "flow": "check", "processor": "nada" })),
        ] {
            let reply = ask(
                id,
                "flow.run",
                json!({ "project": spf, "request": request }),
            );
            assert_eq!(
                reply["error"]["data"]["code"], "processor_not_found",
                "{reply}"
            );
        }
        // Um build sem processadores não tem o que compilar, nem bundle.
        let reply = ask(
            3,
            "flow.run",
            json!({ "project": spf, "request": { "flow": "build" } }),
        );
        assert_eq!(reply["result"]["status"], "succeeded", "{reply}");
        // A onda que a simulação ainda não gravou.
        let reply = ask(
            4,
            "wave.open",
            json!({ "project": spf, "request": { "source": { "kind": "project" } } }),
        );
        assert_eq!(reply["error"]["data"]["code"], "no_waveform", "{reply}");
        // Com o pedido certo, o que falta é o bundle.
        let reply = ask(
            5,
            "flow.run",
            json!({ "project": spf, "request": { "flow": "check" } }),
        );
        assert_eq!(reply["error"]["data"]["code"], "invalid_bundle", "{reply}");
    }

    #[test]
    fn a_project_file_that_does_not_exist_is_spf_missing() {
        let (server, rx) = initialized();
        let dir = tempfile::tempdir().unwrap();
        let gone = Utf8PathBuf::from_path_buf(dir.path().join("x.spf")).unwrap();
        server.handle_line(
            &json!({ "jsonrpc": "2.0", "id": 1, "method": "project.status",
                     "params": { "project": gone } })
            .to_string(),
        );
        let reply = next(&rx);
        assert_eq!(reply["error"]["data"]["code"], "spf_missing", "{reply}");
    }

    #[test]
    fn replacing_on_a_move_keeps_what_the_aurora_keeps_in_place() {
        let (server, rx) = initialized();
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dunce::canonicalize(dir.path()).unwrap()).unwrap();
        let project = Project::create(&root, "demo").unwrap();
        let spf = project.spf_path().to_owned();
        let (a, b) = (root.join("demo/a.v"), root.join("demo/b.v"));
        std::fs::write(&a, "").unwrap();
        std::fs::write(&b, "").unwrap();
        let ask = |id: u32, params: Value| {
            let mut params = params;
            params["project"] = json!(spf);
            server.handle_line(
                &json!({ "jsonrpc": "2.0", "id": id, "method": "project.move", "params": params })
                    .to_string(),
            );
            next(&rx)
        };
        // Sem overwrite, o destino que existe é recusado.
        let reply = ask(1, json!({ "from": a, "to": b }));
        assert_eq!(reply["error"]["data"]["code"], "path_exists", "{reply}");
        // O .spf é do que a AURORA mantém no lugar: nada vai para a lixeira.
        let reply = ask(2, json!({ "from": a, "to": spf, "overwrite": true }));
        assert_eq!(reply["error"]["data"]["code"], "cannot_replace", "{reply}");
        assert!(spf.is_file() && a.is_file());
        // Um movimento já feito não substitui nada.
        let reply = ask(
            3,
            json!({ "from": a, "to": b, "after": true, "overwrite": true }),
        );
        assert_eq!(
            reply["error"]["data"]["code"], "invalid_argument",
            "{reply}"
        );
        assert!(a.is_file() && b.is_file());
    }

    #[test]
    fn the_first_request_tells_the_language() {
        assert!(is_mcp_initialize(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"c"}}}"#
        ));
        assert!(is_mcp_initialize(
            "\u{feff}{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-06-18\"}}"
        ));
        assert!(!is_mcp_initialize(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"client":"aurora-vscode"}}"#
        ));
        assert!(!is_mcp_initialize(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#
        ));
        assert!(!is_mcp_initialize("not json"));
        let mut input = std::io::Cursor::new("\n  \r\n{\"a\":1}\r\nrest\n");
        assert_eq!(first_line(&mut input).as_deref(), Some("{\"a\":1}"));
        assert_eq!(first_line(&mut std::io::Cursor::new("")), None);
    }

    #[test]
    fn edit_distance() {
        assert_eq!(distance("form", "from"), 2);
        assert_eq!(distance("proccessor", "processor"), 1);
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(distance("same", "same"), 0);
    }
}
