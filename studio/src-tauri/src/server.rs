//! O servidor da AURORA no Studio: um `aurora serve` filho que atende, para a
//! interface, os métodos do protocolo da AURORA (`aurora_protocol`). O
//! Studio não tem despacho próprio desses métodos: manda o pedido ao
//! servidor e devolve a resposta, como a extensão do VS Code faz. A
//! interface chama [`aurora_call`] com o método e os parâmetros do protocolo.
//!
//! O servidor abre no primeiro pedido e reabre se cair. O `aurora` é, nesta
//! ordem: o da variável `AURORA_IDE_CLI`, o da preferência `aurora_path` e
//! o da instalação do bundle. O Studio passa ao servidor o bundle que ele
//! mesmo achou e o compilador e o Quartus das preferências, para os dois
//! verem as mesmas ferramentas; mudou uma dessas coisas, o pedido seguinte
//! fecha o servidor e abre outro.
//!
//! No `initialize`, o Studio confere a versão do protocolo: um `aurora` de
//! outro protocolo é recusado com `aurora_incompatible`. Um `aurora` de outra
//! versão da AURORA, com o mesmo protocolo, só gera um aviso no log.
//!
//! No Windows o servidor roda sem janela de console e num Job Object, e
//! morre junto com o Studio. O job deixa um processo sair dele
//! (`CREATE_BREAKAWAY_FROM_JOB`): é assim que o assistente de instalação,
//! que o `update.apply` abre, sobrevive ao Studio, que fecha para ele
//! trocar os arquivos. Ao fechar, o Studio manda `shutdown`, espera até 3 s
//! e mata o que sobrou.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant};

use camino::Utf8PathBuf;
use serde_json::{Value, json};
use tauri::AppHandle;

use crate::error::{IpcError, IpcResult, codes};
use crate::settings::Settings;
use crate::state::{AppState, blocking};
use crate::toolchain;

/// A versão do protocolo da AURORA que este Studio fala
/// (`aurora_protocol::PROTOCOL_VERSION`).
const PROTOCOL_VERSION: u64 = 2;

/// O schema dos pedidos do protocolo, gerado do código da AURORA
/// (`docs/schema/protocol-request.json`): é por ele que o Studio sabe quais
/// métodos têm `project`.
const REQUEST_SCHEMA: &str = include_str!("../../../docs/schema/protocol-request.json");

/// Quanto esperar o `initialize` de um servidor que acabou de abrir.
const START_WAIT: Duration = Duration::from_secs(30);
/// Quanto esperar o servidor sair depois do `shutdown`, antes de matá-lo.
const STOP_WAIT: Duration = Duration::from_secs(3);
/// Quantas linhas do stderr do servidor guardar, para a mensagem de quando
/// ele cai.
const STDERR_TAIL: usize = 20;

/// Os métodos que não passam por [`aurora_call`]: a sessão é do Studio, e os
/// longos, com progresso, vão pelas operações (`jobs.rs`); o `update.check`
/// tem o comando dele, `aurora_update_check`.
const NOT_THROUGH_CALL: &[&str] = &[
    "initialize",
    "shutdown",
    "flow.run",
    "update.check",
    "update.apply",
    "install.add",
    "learn.check",
    "vboard.run",
];

/// O que volta de um pedido.
type Reply = Result<Value, IpcError>;

/// Quem ouve as notificações de um pedido: o método (`phase`, `events`,
/// `build`, `output_omitted`, `install`) e os `params`, sem o `id`.
pub type Listener = Box<dyn Fn(&str, Value) + Send + Sync>;

/// O servidor do Studio, aberto quando preciso.
#[derive(Default)]
pub struct AuroraServer {
    session: Mutex<Option<Arc<Session>>>,
}

impl AuroraServer {
    /// Um pedido ao servidor, que abre se preciso. Bloqueia até a resposta:
    /// chame numa thread de trabalho.
    pub fn call(&self, settings: &Settings, method: &str, params: Value) -> IpcResult<Value> {
        self.session(settings)?
            .request(method, params, None, |_| {}, None)
    }

    /// Um pedido com notificações (os fluxos, a instalação): `listener`
    /// recebe cada uma, e `started` o `id` do pedido, para um
    /// cancelamento ([`AuroraServer::cancel`]).
    pub fn call_with(
        &self,
        settings: &Settings,
        method: &str,
        params: Value,
        listener: Listener,
        started: impl FnOnce(u64),
    ) -> IpcResult<Value> {
        self.session(settings)?
            .request(method, params, Some(listener), started, None)
    }

    /// Pede o cancelamento do pedido `id` (`$/cancel`).
    pub fn cancel(&self, id: u64) {
        let session = self.session.lock().expect("server lock").clone();
        if let Some(session) = session.filter(|s| s.alive()) {
            session.notify("$/cancel", json!({ "id": id }));
        }
    }

    /// O que o servidor disse no `initialize` (`ServerInfo`): a versão do
    /// AURORA, do protocolo, os métodos.
    pub fn info(&self, settings: &Settings) -> IpcResult<Value> {
        let session = self.session(settings)?;
        let info = session.info.lock().expect("info lock").clone();
        Ok(info)
    }

    /// Fecha o servidor, se estiver aberto.
    pub fn stop(&self) {
        let session = self.session.lock().expect("server lock").take();
        if let Some(session) = session {
            session.stop();
        }
    }

    /// O servidor certo para estas preferências: o aberto, se ainda vivo e
    /// com o mesmo `aurora`, bundle, compilador, Quartus e trilhas; senão um
    /// novo.
    fn session(&self, settings: &Settings) -> IpcResult<Arc<Session>> {
        let key = StartKey::of(settings)?;
        let mut guard = self.session.lock().expect("server lock");
        if let Some(session) = guard.as_ref() {
            if session.alive() && session.key == key {
                return Ok(Arc::clone(session));
            }
            if let Some(old) = guard.take() {
                tracing::info!("Restarting the aurora server");
                old.stop();
            }
        }
        let session = Session::start(key)?;
        *guard = Some(Arc::clone(&session));
        Ok(session)
    }
}

/// Com que o servidor abriu: mudou algo daqui, é outro servidor.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StartKey {
    aurora: Utf8PathBuf,
    toolchain: Option<Utf8PathBuf>,
    compiler: Option<String>,
    quartus: Option<String>,
    /// As trilhas do `aurora learn` de outra pasta (preferência `learn_dir`),
    /// que o servidor recebe em `AURORA_LEARN_DIR`.
    learn_dir: Option<String>,
}

impl StartKey {
    fn of(settings: &Settings) -> IpcResult<StartKey> {
        Ok(StartKey {
            aurora: toolchain::aurora_executable(settings)?,
            toolchain: toolchain::bundle_dir(settings),
            compiler: nonempty(&settings.compiler_dir),
            quartus: nonempty(&settings.quartus_dir),
            learn_dir: nonempty(&settings.learn_dir),
        })
    }
}

fn nonempty(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .map(str::to_owned)
}

/// O `CREATE_NO_WINDOW` do Windows para o servidor: sem ele, o `aurora`, um
/// programa de console, abriria uma janela de console própria quando o
/// Studio não tem console. As ferramentas que o servidor roda dividem esse
/// console escondido. Fora do Windows não faz nada.
fn hide_console(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // `CREATE_NO_WINDOW`, de `WinBase.h`.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = command;
}

/// O Job Object do Windows que leva o servidor, e o que ele roda, junto
/// quando o Studio fecha ou morre (`KILL_ON_JOB_CLOSE`). Um processo preso
/// pode iniciar outro fora dele (`CREATE_BREAKAWAY_FROM_JOB`): o assistente
/// de instalação. Fora do Windows, ou se o sistema recusar o job, não faz
/// nada, e o `stop` mata o servidor.
struct ProcessJob {
    #[cfg(windows)]
    job: Option<win32job::Job>,
}

impl ProcessJob {
    fn new() -> ProcessJob {
        #[cfg(windows)]
        {
            let mut limits = win32job::ExtendedLimitInfo::new();
            limits.limit_kill_on_job_close();
            limits.limit_breakaway_ok();
            let job = win32job::Job::create_with_limit_info(&limits)
                .inspect_err(|error| tracing::warn!(%error, "Creating the job object"))
                .ok();
            ProcessJob { job }
        }
        #[cfg(not(windows))]
        ProcessJob {}
    }

    /// Prende `child` ao job; o que ele iniciar daqui em diante nasce preso
    /// também.
    fn adopt(&mut self, child: &Child) {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            if let Some(job) = &self.job
                && let Err(error) = job.assign_process(child.as_raw_handle() as isize)
            {
                tracing::warn!(%error, "Assigning the aurora server to the job object");
                self.job = None;
            }
        }
        #[cfg(not(windows))]
        let _ = child;
    }
}

/// Um processo `aurora serve` vivo.
struct Session {
    key: StartKey,
    child: Mutex<Child>,
    stdin: Mutex<Option<ChildStdin>>,
    pending: Mutex<HashMap<u64, mpsc::Sender<Reply>>>,
    listeners: Mutex<HashMap<u64, Listener>>,
    next_id: AtomicU64,
    alive: AtomicBool,
    /// As últimas linhas do stderr.
    stderr: Mutex<Vec<String>>,
    /// O que o servidor disse no `initialize`.
    info: Mutex<Value>,
    /// O Job Object que leva o servidor junto quando o Studio morre.
    _job: Mutex<ProcessJob>,
}

impl Session {
    fn start(key: StartKey) -> IpcResult<Arc<Session>> {
        let mut command = Command::new(key.aurora.as_std_path());
        if let Some(dir) = &key.toolchain {
            command.arg("--toolchain").arg(dir.as_std_path());
        }
        if let Some(dir) = &key.compiler {
            command.arg("--compiler").arg(dir);
        }
        if let Some(dir) = &key.quartus {
            command.arg("--quartus").arg(dir);
        }
        if let Some(dir) = &key.learn_dir {
            command.env("AURORA_LEARN_DIR", dir);
        }
        command
            .arg("serve")
            .env("NO_COLOR", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_console(&mut command);
        let mut job = ProcessJob::new();
        let mut child = command.spawn().map_err(|error| {
            IpcError::new(
                codes::AURORA_MISSING,
                format!("Could not start {} serve: {error}", key.aurora),
            )
        })?;
        job.adopt(&child);
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        tracing::info!(aurora = %key.aurora, "aurora server started");
        let session = Arc::new(Session {
            key,
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            pending: Mutex::new(HashMap::new()),
            listeners: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            alive: AtomicBool::new(true),
            stderr: Mutex::new(Vec::new()),
            info: Mutex::new(Value::Null),
            _job: Mutex::new(job),
        });
        if let Some(stdout) = stdout {
            let reader = Arc::clone(&session);
            std::thread::Builder::new()
                .name("aurora-server-stdout".into())
                .spawn(move || {
                    for line in BufReader::new(stdout).lines() {
                        let Ok(line) = line else {
                            break;
                        };
                        reader.on_line(&line);
                    }
                    reader.died();
                })
                .map_err(|e| IpcError::new(codes::INTERNAL, e.to_string()))?;
        }
        if let Some(stderr) = stderr {
            let reader = Arc::clone(&session);
            std::thread::Builder::new()
                .name("aurora-server-stderr".into())
                .spawn(move || {
                    for line in BufReader::new(stderr).lines() {
                        let Ok(line) = line else {
                            break;
                        };
                        tracing::info!(target: "aurora_serve", "{line}");
                        let mut tail = reader.stderr.lock().expect("stderr lock");
                        if tail.len() == STDERR_TAIL {
                            tail.remove(0);
                        }
                        tail.push(line);
                    }
                })
                .map_err(|e| IpcError::new(codes::INTERNAL, e.to_string()))?;
        }
        let client = format!("aurora-ide {}", env!("CARGO_PKG_VERSION"));
        let info = match session.request(
            "initialize",
            json!({ "client": client }),
            None,
            |_| {},
            Some(START_WAIT),
        ) {
            Ok(info) => info,
            Err(error) => {
                session.stop();
                return Err(error);
            }
        };
        let protocol = info["protocol"].as_u64();
        if protocol != Some(PROTOCOL_VERSION) {
            session.stop();
            return Err(IpcError::new(
                codes::AURORA_INCOMPATIBLE,
                format!(
                    "{} speaks protocol {}, and this AURORA IDE speaks {PROTOCOL_VERSION}: use an aurora of the same version as the IDE (Settings > AURORA bundle)",
                    session.key.aurora,
                    protocol.map_or_else(|| "?".to_owned(), |p| p.to_string()),
                ),
            ));
        }
        let version = info["aurora"].as_str().unwrap_or("?");
        if version != env!("CARGO_PKG_VERSION") {
            tracing::warn!(
                "The aurora server is version {version} and the Studio {}: same protocol, so they work together",
                env!("CARGO_PKG_VERSION")
            );
        }
        *session.info.lock().expect("info lock") = info;
        Ok(session)
    }

    fn alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    /// Manda um pedido e espera a resposta (até `timeout`, se houver).
    fn request(
        &self,
        method: &str,
        params: Value,
        listener: Option<Listener>,
        started: impl FnOnce(u64),
        timeout: Option<Duration>,
    ) -> Reply {
        if !self.alive() {
            return Err(self.gone());
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        self.pending.lock().expect("pending lock").insert(id, tx);
        if let Some(listener) = listener {
            self.listeners
                .lock()
                .expect("listeners lock")
                .insert(id, listener);
        }
        let mut message = json!({ "jsonrpc": "2.0", "id": id, "method": method });
        if !params.is_null() {
            message["params"] = params;
        }
        if let Err(error) = self.write(&message) {
            self.forget(id);
            tracing::warn!("Could not write to the aurora server: {error}");
            return Err(self.gone());
        }
        started(id);
        let reply = match timeout {
            Some(wait) => rx.recv_timeout(wait).map_err(|_| {
                IpcError::new(
                    codes::SERVER_GONE,
                    format!("The aurora server did not answer {method} in {wait:?}"),
                )
            }),
            None => rx.recv().map_err(|_| self.gone()),
        };
        self.forget(id);
        reply?
    }

    fn forget(&self, id: u64) {
        self.pending.lock().expect("pending lock").remove(&id);
        self.listeners.lock().expect("listeners lock").remove(&id);
    }

    fn notify(&self, method: &str, params: Value) {
        let message = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        if let Err(error) = self.write(&message) {
            tracing::warn!("Could not write to the aurora server: {error}");
        }
    }

    fn write(&self, message: &Value) -> std::io::Result<()> {
        let mut stdin = self.stdin.lock().expect("stdin lock");
        let Some(stdin) = stdin.as_mut() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "the server input is closed",
            ));
        };
        let mut line = message.to_string();
        line.push('\n');
        stdin.write_all(line.as_bytes())?;
        stdin.flush()
    }

    /// Uma linha do stdout: uma resposta vai para quem a espera; uma
    /// notificação, para quem ouve o pedido dela.
    fn on_line(&self, line: &str) {
        let Ok(mut value) = serde_json::from_str::<Value>(line) else {
            tracing::warn!("The aurora server wrote a line that is not JSON: {line}");
            return;
        };
        let answered = value.get("result").is_some() || value.get("error").is_some();
        if answered {
            let Some(id) = value.get("id").and_then(Value::as_u64) else {
                tracing::warn!("The aurora server answered without an id: {line}");
                return;
            };
            let sender = self.pending.lock().expect("pending lock").remove(&id);
            if let Some(sender) = sender {
                let reply = match value.get("error") {
                    Some(error) => Err(ipc_error(error)),
                    None => Ok(value["result"].take()),
                };
                let _ = sender.send(reply);
            }
            return;
        }
        let Some(method) = value
            .get("method")
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            return;
        };
        let mut params = value
            .get_mut("params")
            .map(Value::take)
            .unwrap_or(Value::Null);
        if method == "shutdown" {
            tracing::info!("The aurora server is shutting down: {params}");
            return;
        }
        let id = params.get("id").and_then(Value::as_u64);
        if let Some(fields) = params.as_object_mut() {
            fields.remove("id");
        }
        if let Some(id) = id
            && let Some(listener) = self.listeners.lock().expect("listeners lock").get(&id)
        {
            listener(&method, params);
        }
    }

    /// O servidor saiu: quem esperava resposta recebe o erro.
    fn died(&self) {
        self.alive.store(false, Ordering::SeqCst);
        let error = self.gone();
        let waiting: Vec<_> = self
            .pending
            .lock()
            .expect("pending lock")
            .drain()
            .map(|(_, sender)| sender)
            .collect();
        for sender in waiting {
            let _ = sender.send(Err(error.clone()));
        }
        self.listeners.lock().expect("listeners lock").clear();
        tracing::info!("aurora server ended");
    }

    fn gone(&self) -> IpcError {
        let tail = self.stderr.lock().expect("stderr lock").join("\n");
        let message = if tail.is_empty() {
            "The aurora server stopped; the next request starts it again".to_owned()
        } else {
            format!(
                "The aurora server stopped; the next request starts it again. Its last messages:\n{tail}"
            )
        };
        IpcError::new(codes::SERVER_GONE, message)
    }

    /// `shutdown`, a entrada fechada, até [`STOP_WAIT`]; depois, o que
    /// sobrou morre.
    fn stop(&self) {
        if self.alive() {
            let _ = self.request("shutdown", Value::Null, None, |_| {}, Some(STOP_WAIT));
        }
        drop(self.stdin.lock().expect("stdin lock").take());
        let mut child = self.child.lock().expect("child lock");
        let until = Instant::now() + STOP_WAIT;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < until => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
            }
        }
        self.alive.store(false, Ordering::SeqCst);
    }
}

/// O erro de uma resposta do servidor como erro do Studio: o código da AURORA
/// (`error.data.code`) quando há, senão um pelo código do JSON-RPC.
fn ipc_error(error: &Value) -> IpcError {
    let data = error.get("data");
    if let Some(code) = data.and_then(|d| d.get("code")).and_then(Value::as_str) {
        let message = data
            .and_then(|d| d.get("message"))
            .and_then(Value::as_str)
            .or_else(|| error["message"].as_str())
            .unwrap_or_default();
        return IpcError::new(code, message).with_data(data.and_then(|d| d.get("data")).cloned());
    }
    let code = match error["code"].as_i64() {
        Some(-32602 | -32600) => codes::INVALID_ARGUMENT,
        Some(-32601) => "unsupported",
        Some(-32001) => codes::CANCELLED,
        Some(-32003) => codes::SERVER_GONE,
        _ => codes::INTERNAL,
    };
    IpcError::new(
        code,
        error["message"]
            .as_str()
            .unwrap_or("The aurora server could not answer"),
    )
}

/// Se um método do protocolo tem o campo `project`, e se o exige.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectField {
    /// Não tem.
    None,
    /// Tem, opcional (`wave.open`, que também abre um arquivo).
    Optional,
    /// Exige.
    Required,
}

/// O campo `project` de `method`, tirado do schema dos pedidos.
fn project_field(method: &str) -> ProjectField {
    static FIELDS: OnceLock<HashMap<String, ProjectField>> = OnceLock::new();
    FIELDS
        .get_or_init(|| {
            let schema: Value = serde_json::from_str(REQUEST_SCHEMA).unwrap_or(Value::Null);
            schema["oneOf"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|entry| {
                    let method = entry["properties"]["method"]["const"].as_str()?;
                    let params = &entry["properties"]["params"];
                    let field = if params["required"]
                        .as_array()
                        .is_some_and(|r| r.iter().any(|f| f == "project"))
                    {
                        ProjectField::Required
                    } else if params["properties"].get("project").is_some() {
                        ProjectField::Optional
                    } else {
                        return None;
                    };
                    Some((method.to_owned(), field))
                })
                .collect()
        })
        .get(method)
        .copied()
        .unwrap_or(ProjectField::None)
}

/// Os parâmetros de `method` com o projeto aberto no `project`, quando o
/// método tem o campo e o pedido não o traz: a interface não lida com
/// caminhos de `.spf`, como antes. Sem projeto aberto, um método que o exige
/// é `no_project`; um que o tem opcional vai sem ele. Quem não quer o
/// projeto num campo opcional manda `project: null`.
pub(crate) fn with_project(
    state: &AppState,
    method: &str,
    params: Option<Value>,
) -> IpcResult<Value> {
    let mut params = params.unwrap_or(Value::Null);
    let field = project_field(method);
    if field == ProjectField::None {
        return Ok(params);
    }
    if params.is_null() {
        params = json!({});
    }
    if let Some(fields) = params.as_object_mut()
        && !fields.contains_key("project")
    {
        match (state.spf(), field) {
            (Ok(spf), _) => {
                fields.insert("project".to_owned(), json!(spf));
            }
            (Err(error), ProjectField::Required) => return Err(error),
            (Err(_), _) => {}
        }
    }
    Ok(params)
}

/// Um método do protocolo da AURORA, pelo servidor. Sem `project` nos
/// parâmetros, vale o projeto aberto. Os fluxos, a atualização e a
/// instalação vão pelas operações, que mostram o progresso.
#[tauri::command]
pub async fn aurora_call(
    app: AppHandle,
    method: String,
    params: Option<Value>,
) -> IpcResult<Value> {
    blocking(app, move |_, state| {
        if NOT_THROUGH_CALL.contains(&method.as_str()) {
            return Err(IpcError::new(
                codes::INVALID_ARGUMENT,
                format!("{method} does not go through aurora_call"),
            ));
        }
        let params = with_project(state, &method, params)?;
        state.server.call(&state.settings.get(), &method, params)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_errors_keep_the_aurora_code() {
        let error = ipc_error(&json!({
            "code": -32000,
            "message": "No project",
            "data": { "code": "spf_missing", "message": "The project file x does not exist anymore" }
        }));
        assert_eq!(error.code, "spf_missing");
        assert!(error.message.contains("does not exist anymore"));
        assert_eq!(
            ipc_error(&json!({ "code": -32602, "message": "Invalid params" })).code,
            codes::INVALID_ARGUMENT
        );
        assert_eq!(
            ipc_error(&json!({ "code": -32001, "message": "Cancelled" })).code,
            "cancelled"
        );
    }

    #[test]
    fn the_protocol_and_the_schema_are_the_aurora_ones() {
        assert_eq!(
            PROTOCOL_VERSION,
            u64::from(aurora_protocol::PROTOCOL_VERSION)
        );
        let embedded: Value = serde_json::from_str(REQUEST_SCHEMA).unwrap();
        let generated = serde_json::to_value(aurora_protocol::request_schema()).unwrap();
        assert_eq!(
            embedded, generated,
            "docs/schema/protocol-request.json está velho: AURORA_UPDATE_SCHEMA=1 cargo test -p aurora-cli"
        );
    }

    #[test]
    fn the_project_goes_where_the_method_has_it() {
        assert_eq!(project_field("project.status"), ProjectField::Required);
        assert_eq!(project_field("history.compare"), ProjectField::Required);
        assert_eq!(project_field("toolchain.info"), ProjectField::None);
        assert_eq!(project_field("project.check_name"), ProjectField::None);
        // A onda pode ser de um arquivo, sem projeto.
        assert_eq!(project_field("wave.open"), ProjectField::Optional);
    }

    /// O servidor de verdade, com o `aurora` de `AURORA_IDE_CLI`: cria um
    /// projeto, lê, erra e fecha.
    #[test]
    fn a_real_server_answers_and_stops() {
        if std::env::var_os("AURORA_IDE_CLI").is_none() {
            eprintln!("PULADO: defina AURORA_IDE_CLI com o aurora a testar");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_owned()).unwrap();
        let settings = Settings::default();
        let server = AuroraServer::default();
        let status = server
            .call(
                &settings,
                "project.create",
                json!({ "parent": root, "name": "demo" }),
            )
            .unwrap();
        assert_eq!(status["name"], "demo");
        let spf = status["spf"].as_str().unwrap().to_owned();
        let status = server
            .call(&settings, "project.status", json!({ "project": spf }))
            .unwrap();
        assert_eq!(status["name"], "demo");
        let error = server
            .call(&settings, "project.status", json!({ "projetc": spf }))
            .unwrap_err();
        assert_eq!(error.code, codes::INVALID_ARGUMENT, "{error}");
        assert!(server.info(&settings).unwrap()["methods"].is_array());
        server.stop();
        assert!(server.session.lock().unwrap().is_none());
    }

    /// Os pedidos com a forma que o `api.ts` manda, no servidor de verdade:
    /// um nome de campo errado ali seria recusado aqui.
    #[test]
    fn the_interface_requests_reach_the_server() {
        if std::env::var_os("AURORA_IDE_CLI").is_none() {
            eprintln!("PULADO: defina AURORA_IDE_CLI com o aurora a testar");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_owned()).unwrap();
        let settings = Settings::default();
        let server = AuroraServer::default();
        let call = |method: &str, mut params: Value| {
            params["project"] = json!(root.join("demo/demo.spf"));
            server.call(&settings, method, params)
        };
        server
            .call(
                &settings,
                "project.create",
                json!({ "parent": root, "name": "demo" }),
            )
            .unwrap();
        let (top, tb) = (root.join("demo/top.v"), root.join("demo/top_tb.v"));
        let added = call("project.add", json!({ "files": [top], "testbench": false })).unwrap();
        assert_eq!(added.as_array().map(Vec::len), Some(1), "{added}");
        call("project.add", json!({ "files": [tb], "testbench": true })).unwrap();
        let outcome = call("project.top", json!({ "target": top })).unwrap();
        assert!(outcome["top_level"].as_str().unwrap().ends_with("top.v"));
        call("project.testbench", json!({ "path": tb })).unwrap();
        let moved = root.join("demo/rtl/top.v");
        let result = call(
            "project.move",
            json!({ "from": top, "to": moved, "overwrite": false }),
        )
        .unwrap();
        assert!(
            result["to"].as_str().unwrap().ends_with("top.v"),
            "{result}"
        );
        let removed = call("project.remove", json!({ "files": [moved] })).unwrap();
        assert_eq!(removed["removed"].as_array().map(Vec::len), Some(1));
        let spec = json!({ "name": "soma", "language": "cmm", "nbmant": 20, "nbexpo": 7 });
        let processor = call("processor.add", json!({ "spec": spec })).unwrap();
        assert_eq!(processor["name"], "soma", "{processor}");
        server.stop();
    }
}
