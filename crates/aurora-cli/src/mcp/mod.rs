//! `aurora mcp`: o orquestrador para um agente, como servidor MCP (Model
//! Context Protocol) por stdin e stdout. É parte do recurso experimental
//! `assistant` (`aurora_core::experimental`): `commands.rs` só chega aqui com
//! ele ligado.
//!
//! O MCP também é JSON-RPC 2.0 com uma mensagem por linha, então este é um
//! adaptador fino sobre o despacho do `aurora serve`
//! ([`Server::handle_with`]): uma ferramenta por método do protocolo que faz
//! sentido para um agente, com o `inputSchema` tirado do mesmo schema dos
//! pedidos (`aurora_protocol::request_schema`) e a descrição tirada
//! das docs. `flow.run` é desdobrado em `build`, `check`, `hierarchy`,
//! `sim` e `synth`, cada um com os campos do próprio fluxo.
//!
//! O resultado de uma ferramenta vai em `content` como um resumo em texto
//! ([`summary`]: passou ou falhou, onde, qual relatório) e em
//! `structuredContent` como o JSON do resultado, com a saída dos passos
//! cortada; `isError` marca a AURORA que não conseguiu rodar, com o
//! `ErrorInfo` no texto. As ferramentas que mudam a instalação
//! (`update_apply`, `install_add`) só existem com `--allow-update`.
//!
//! O projeto é o padrão do servidor (o do `-C` global, que é a pasta atual
//! por padrão) quando a chamada não traz `project`; um caminho relativo é
//! resolvido a partir da pasta atual. `project_find` lista os `.spf` de uma
//! pasta.
//!
//! As chamadas rodam numa fila de um worker, fora da thread que lê a
//! entrada: `notifications/cancelled` cancela uma chamada em andamento, e
//! `ping` responde durante uma simulação longa. Uma chamada com
//! `_meta.progressToken` recebe `notifications/progress` a cada fase e a
//! cada processador compilado.
//!
//! Versão do protocolo MCP: `2025-06-18`. Métodos: `initialize`,
//! `notifications/initialized`, `notifications/cancelled`, `ping`,
//! `tools/list`, `tools/call`. Sem recursos nem prompts. `aurora mcp setup`
//! registra o servidor num cliente ([`setup`]).

mod setup;
mod summary;

use std::collections::{BTreeSet, HashMap};
use std::io::BufRead;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use aurora_core::delivery::{Notification, trim_step_output};
use aurora_core::{CancelToken, Project};
use aurora_protocol::{Agent, method_info, request_schema};
use camino::{Utf8Path, Utf8PathBuf};
use serde_json::{Map, Value, json};

use crate::serve::{
    Emit, Server, Writer, flow_fields, method_fields, parse_request, unknown_field,
};
use crate::settings::ToolchainArgs;

pub use setup::SetupArgs;

/// A versão do MCP que este servidor fala.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// Quanto da saída de cada passo (`stdout`, `stderr`) vai em
/// `structuredContent`: o fim, onde estão o erro e o `$finish`.
const STEP_OUTPUT_BYTES: usize = 2048;

/// Quanto esperar as chamadas em andamento ao encerrar por um sinal.
const SHUTDOWN_WAIT: Duration = Duration::from_secs(3);

/// As opções de `aurora mcp`.
#[derive(clap::Args, Debug, Clone)]
pub struct McpArgs {
    #[command(subcommand)]
    pub command: Option<McpCommand>,
    /// Also expose update_apply and install_add, which change the AURORA installation
    #[arg(long)]
    pub allow_update: bool,
}

/// Os subcomandos de `aurora mcp`.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum McpCommand {
    /// Register the AURORA MCP server in a client (Claude Code, Claude Desktop) or print the entry
    Setup(SetupArgs),
}

/// Uma ferramenta: o nome para o agente e o método do protocolo por trás.
/// Se o método vira ferramenta, e se só com `--allow-update`, é a marcação
/// dele no protocolo ([`method_info`]); aqui ficam o nome e a ordem.
struct Tool {
    name: &'static str,
    method: &'static str,
    /// Um fluxo de `flow.run`, desdobrado.
    flow: Option<&'static str>,
}

impl Tool {
    /// Muda a instalação da AURORA: só aparece com `--allow-update` (a
    /// marcação `agent_update` do método).
    fn needs_allow_update(&self) -> bool {
        method_info(self.method).is_some_and(|info| info.agent == Agent::AllowUpdate)
    }
}

const fn tool(name: &'static str, method: &'static str) -> Tool {
    Tool {
        name,
        method,
        flow: None,
    }
}

const fn flow(name: &'static str, flow: &'static str) -> Tool {
    Tool {
        name,
        method: "flow.run",
        flow: Some(flow),
    }
}

/// As ferramentas do protocolo, na ordem em que o agente as vê.
const TOOLS: &[Tool] = &[
    tool("project_create", "project.create"),
    tool("project_status", "project.status"),
    tool("project_add", "project.add"),
    tool("project_remove", "project.remove"),
    tool("project_top", "project.top"),
    tool("project_testbench", "project.testbench"),
    tool("project_order", "project.order"),
    tool("project_move", "project.move"),
    tool("project_convert", "project.convert"),
    tool("processor_add", "processor.add"),
    tool("processor_set", "processor.set"),
    tool("processor_remove", "processor.remove"),
    tool("processor_outputs", "processor.outputs"),
    flow("build", "build"),
    flow("check", "check"),
    flow("hierarchy", "hierarchy"),
    flow("sim", "simulate"),
    flow("synth", "synthesize"),
    tool("wave_signals", "wave.signals"),
    tool("wave_choose", "wave.choose"),
    tool("history_list", "history.list"),
    tool("history_show", "history.show"),
    tool("history_compare", "history.compare"),
    tool("toolchain_info", "toolchain.info"),
    tool("toolchain_verify", "toolchain.verify"),
    tool("experimental_list", "experimental.list"),
    tool("fpga_boards", "fpga.boards"),
    tool("fpga_config", "fpga.config"),
    tool("fpga_config_set", "fpga.config_set"),
    tool("fpga_connect", "fpga.connect"),
    tool("fpga_check", "fpga.check"),
    tool("fpga_modules", "fpga.modules"),
    tool("fpga_status", "fpga.status"),
    tool("learn_check", "learn.check"),
    tool("git_status", "git.status"),
    tool("git_diff", "git.diff"),
    tool("git_log", "git.log"),
    tool("git_commit_files", "git.commit_files"),
    tool("git_branches", "git.branches"),
    tool("git_commit", "git.commit"),
    tool("update_check", "update.check"),
    tool("update_apply", "update.apply"),
    tool("install_add", "install.add"),
];

/// A ferramenta local que procura projetos.
const PROJECT_FIND: &str = "project_find";
/// Até onde `project_find` desce por padrão.
const FIND_DEPTH: u64 = 3;
/// As pastas que `project_find` não entra.
const FIND_SKIP: &[&str] = &[".aurora8", ".git", "node_modules", "target", ".claude"];

/// `aurora mcp`. `project_dir` é o `-C` global (a pasta atual por padrão): o
/// projeto padrão das ferramentas é o dele, se há um. `cancel` é o pedido
/// dos sinais, que cancela as chamadas em andamento e encerra.
pub fn run(
    project_dir: &Utf8Path,
    toolchain: &ToolchainArgs,
    args: &McpArgs,
    cancel: &CancelToken,
) -> anyhow::Result<bool> {
    if let Some(McpCommand::Setup(setup)) = &args.command {
        setup::run(toolchain, setup)?;
        return Ok(true);
    }
    let stdin = std::io::stdin();
    serve_session(
        project_dir,
        toolchain,
        args.allow_update,
        cancel,
        None,
        stdin.lock(),
    )
}

/// Uma sessão MCP: a do `aurora mcp`, e a que o `aurora serve` abre quando o
/// primeiro pedido é um `initialize` do MCP (`first`, já lido da entrada).
/// Quem chama já conferiu o recurso experimental `assistant`.
pub(crate) fn serve_session(
    project_dir: &Utf8Path,
    toolchain: &ToolchainArgs,
    allow_update: bool,
    cancel: &CancelToken,
    first: Option<String>,
    input: impl BufRead,
) -> anyhow::Result<bool> {
    let default_project = match Project::discover(project_dir) {
        Ok(project) => Some(project.spf_path().to_owned()),
        Err(error) => {
            tracing::info!("No default project: {error}");
            None
        }
    };
    let mcp = Arc::new(Mcp::new(
        Server::new(toolchain.clone(), Writer::silent()),
        Writer::stdout(),
        allow_update,
        default_project,
    ));
    // Um sinal cancela o que roda, espera e sai.
    let signalled = cancel.clone();
    let watcher = Arc::clone(&mcp);
    std::thread::Builder::new()
        .name("aurora-mcp-signals".into())
        .spawn(move || {
            while !signalled.is_cancelled() {
                std::thread::sleep(Duration::from_millis(50));
            }
            watcher.finish();
            std::process::exit(0);
        })?;
    if let Some(line) = &first {
        mcp.handle_line(line);
    }
    for line in input.lines() {
        let Ok(line) = line else {
            break;
        };
        mcp.handle_line(&line);
    }
    mcp.finish();
    Ok(true)
}

type Job = Box<dyn FnOnce() + Send>;

/// O servidor MCP: o despacho da AURORA mais o schema dos pedidos, de onde
/// saem as ferramentas.
pub(crate) struct Mcp {
    server: Server,
    out: Writer,
    allow_update: bool,
    default_project: Option<Utf8PathBuf>,
    schema: Value,
    /// As ferramentas cujo `inputSchema` tem `project`.
    project_tools: BTreeSet<&'static str>,
    /// As chamadas em andamento ou na fila, pelo `id` do pedido.
    pending: Mutex<HashMap<String, CancelToken>>,
    /// A fila do worker que roda as chamadas, uma de cada vez.
    queue: Mutex<Option<Sender<Job>>>,
}

impl Mcp {
    pub(crate) fn new(
        server: Server,
        out: Writer,
        allow_update: bool,
        default_project: Option<Utf8PathBuf>,
    ) -> Self {
        let schema = serde_json::to_value(request_schema()).unwrap_or(Value::Null);
        let mut mcp = Mcp {
            server,
            out,
            allow_update,
            default_project,
            schema,
            project_tools: BTreeSet::new(),
            pending: Mutex::new(HashMap::new()),
            queue: Mutex::new(None),
        };
        mcp.project_tools = TOOLS
            .iter()
            .filter(|t| {
                mcp.tool_schema(t)
                    .is_some_and(|(_, schema)| schema["properties"].get("project").is_some())
            })
            .map(|t| t.name)
            .collect();
        mcp
    }

    /// Uma linha da entrada: a resposta sai pelo escritor (as chamadas de
    /// ferramenta, mais tarde, pelo worker); uma notificação não responde.
    pub(crate) fn handle_line(self: &Arc<Self>, line: &str) {
        // O PowerShell põe um BOM na primeira coisa que escreve para um
        // programa, e o JSON não tem BOM.
        let line = line.strip_prefix('\u{feff}').unwrap_or(line);
        if line.trim().is_empty() {
            return;
        }
        let message: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                self.error(Value::Null, -32700, &format!("Invalid JSON: {error}"));
                return;
            }
        };
        let id = message.get("id").cloned();
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            self.error(
                id.unwrap_or(Value::Null),
                -32600,
                "The request has no method",
            );
            return;
        };
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        let Some(id) = id else {
            if method == "notifications/cancelled"
                && let Some(request) = params.get("requestId")
            {
                self.cancel(request);
            }
            return;
        };
        match method {
            "initialize" => self.respond(id, self.initialize(&params)),
            "ping" => self.respond(id, json!({})),
            "tools/list" => self.respond(id, json!({ "tools": self.tools() })),
            "tools/call" => self.start_call(id, params),
            other => self.error(id, -32601, &format!("Method not found: {other}")),
        }
    }

    fn respond(&self, id: Value, result: Value) {
        self.out
            .send(&json!({ "jsonrpc": "2.0", "id": id, "result": result }));
    }

    fn error(&self, id: Value, code: i64, message: &str) {
        self.out.send(&json!({
            "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message }
        }));
    }

    /// Põe uma chamada de ferramenta na fila do worker.
    fn start_call(self: &Arc<Self>, id: Value, params: Value) {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let token = params
            .pointer("/_meta/progressToken")
            .filter(|t| t.is_string() || t.is_number())
            .cloned();
        let cancel = CancelToken::new();
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(id.to_string(), cancel.clone());
        let mcp = Arc::clone(self);
        let job: Job = Box::new(move || {
            if cancel.is_cancelled() {
                mcp.error(id.clone(), -32800, "Cancelled before it started");
            } else {
                let progress = token.map(|token| {
                    Arc::new(Progress {
                        out: mcp.out.clone(),
                        token,
                        count: AtomicU64::new(0),
                    })
                });
                match mcp.call(&name, arguments, &cancel, progress) {
                    Ok(result) => mcp.respond(id.clone(), result),
                    Err(message) => mcp.error(id.clone(), -32602, &message),
                }
            }
            mcp.pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&id.to_string());
        });
        self.enqueue(job);
    }

    fn enqueue(&self, job: Job) {
        let mut queue = self
            .queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let sender = queue.get_or_insert_with(|| {
            let (tx, rx) = mpsc::channel::<Job>();
            std::thread::Builder::new()
                .name("aurora-mcp-worker".into())
                .spawn(move || {
                    for job in rx {
                        job();
                    }
                })
                .expect("spawn the worker thread");
            tx
        });
        if sender.send(job).is_err() {
            tracing::error!("The MCP worker is gone");
        }
    }

    /// `notifications/cancelled`: cancela a chamada com esse `requestId`,
    /// rodando ou na fila.
    fn cancel(&self, id: &Value) {
        let pending = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(token) = pending.get(&id.to_string()) {
            token.cancel();
            tracing::info!(id = %id, "MCP call cancelled");
        }
    }

    /// Cancela tudo que está em andamento e espera até [`SHUTDOWN_WAIT`].
    pub(crate) fn finish(&self) {
        for token in self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
        {
            token.cancel();
        }
        let until = Instant::now() + SHUTDOWN_WAIT;
        while Instant::now() < until {
            if self
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_empty()
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    fn initialize(&self, params: &Value) -> Value {
        let client = params
            .get("clientInfo")
            .and_then(|c| c.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("?");
        tracing::info!(client, "MCP session started");
        let project = match &self.default_project {
            Some(spf) => format!(
                "The default project is {spf}: tools use it when `project` is omitted."
            ),
            None => "There is no default project: pass `project` (the .spf file or the project \
                     folder, absolute or relative to the current folder) to every project tool, \
                     or find one with project_find (or start the server with aurora -C <project> mcp)."
                .to_owned(),
        };
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "aurora", "version": env!("CARGO_PKG_VERSION") },
            "instructions": format!(
                "AURORA orchestrates Verilog, VHDL and SAPHO processor projects. {project} Use build, \
                 check, sim and synth for the flows (each returns a text summary and the full \
                 result in structuredContent), project_status to read the project, history_show \
                 to read a past report. Long calls report progress when the request carries a \
                 progressToken, and stop on notifications/cancelled."
            ),
        })
    }

    /// Cada ferramenta com o `inputSchema` do método dela.
    pub(crate) fn tools(&self) -> Vec<Value> {
        let mut tools: Vec<Value> = TOOLS
            .iter()
            .filter(|t| self.allow_update || !t.needs_allow_update())
            .filter_map(|t| {
                let (description, schema) = self.tool_schema(t)?;
                Some(json!({
                    "name": t.name,
                    "description": description,
                    "inputSchema": schema,
                }))
            })
            .collect();
        tools.insert(
            1,
            json!({
                "name": PROJECT_FIND,
                "description": "Lists the AURORA projects (.spf files) under a folder, to find the \
                    project path the other tools take.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "dir": { "type": "string", "description": "The folder to search (default: the current folder, or the default project's parent)." },
                        "depth": { "type": "integer", "minimum": 0, "default": FIND_DEPTH, "description": "How many folder levels to descend." }
                    }
                }
            }),
        );
        tools
    }

    /// A descrição e o `inputSchema` de uma ferramenta, do schema dos
    /// pedidos. O `project` fica opcional: o padrão do servidor cobre.
    fn tool_schema(&self, tool: &Tool) -> Option<(String, Value)> {
        let variants = self.schema.get("oneOf")?.as_array()?;
        let variant = variants.iter().find(|v| {
            v.pointer("/properties/method/const")
                .and_then(Value::as_str)
                .is_some_and(|m| m == tool.method)
        })?;
        let mut description = clean(variant.get("description").and_then(Value::as_str)?);
        let mut params = variant
            .pointer("/properties/params")
            .cloned()
            .unwrap_or_else(|| json!({ "type": "object", "properties": {} }));
        if let Some(flow) = tool.flow {
            // Os campos do fluxo no lugar de `request`, mais o projeto.
            let flows = self
                .schema
                .pointer("/$defs/FlowRequest/oneOf")?
                .as_array()?;
            let variant = flows.iter().find(|v| {
                v.pointer("/properties/flow/const")
                    .and_then(Value::as_str)
                    .is_some_and(|f| f == flow)
            })?;
            description = clean(variant.get("description").and_then(Value::as_str)?);
            let mut properties = Map::new();
            properties.insert("project".to_owned(), json!({ "type": "string" }));
            if let Some(Value::Object(fields)) = variant.get("properties") {
                for (name, field) in fields {
                    if name != "flow" {
                        properties.insert(name.clone(), field.clone());
                    }
                }
            }
            let required: Vec<Value> = variant
                .get("required")
                .and_then(Value::as_array)
                .map(|fields| {
                    fields
                        .iter()
                        .filter(|f| f.as_str() != Some("flow"))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            params = json!({ "type": "object", "properties": properties, "required": required });
        }
        if params.get("type").is_none() {
            params["type"] = json!("object");
        }
        if params.get("properties").is_none() {
            params["properties"] = json!({});
        }
        if let Some(project) = params["properties"].get_mut("project") {
            project["description"] = json!(
                "The project: its .spf file or its folder, absolute or relative to the current \
                 folder. Omit it to use the server's default project."
            );
            if let Some(required) = params.get_mut("required").and_then(Value::as_array_mut) {
                required.retain(|r| r.as_str() != Some("project"));
            }
        }
        let defs = self.referenced_defs(&params);
        if !defs.is_empty() {
            params["$defs"] = Value::Object(defs);
        }
        Some((description, params))
    }

    /// As definições de `$defs` que `schema` referencia, e as que elas
    /// referenciam.
    fn referenced_defs(&self, schema: &Value) -> Map<String, Value> {
        let Some(all) = self.schema.get("$defs").and_then(Value::as_object) else {
            return Map::new();
        };
        let mut wanted = BTreeSet::new();
        collect_refs(schema, &mut wanted);
        let mut done = BTreeSet::new();
        let mut defs = Map::new();
        while let Some(name) = wanted.iter().next().cloned() {
            wanted.remove(&name);
            if !done.insert(name.clone()) {
                continue;
            }
            if let Some(def) = all.get(&name) {
                collect_refs(def, &mut wanted);
                defs.insert(name, def.clone());
            }
        }
        defs
    }

    /// O projeto de uma chamada: o do campo (absoluto ou relativo à pasta
    /// atual), senão o padrão do servidor.
    fn project_of(&self, arguments: &mut Map<String, Value>) -> Result<(), String> {
        let given = arguments
            .get("project")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(Utf8PathBuf::from);
        let project = match given {
            Some(path) => crate::settings::absolute(&path).map_err(|e| e.to_string())?,
            None => match &self.default_project {
                Some(spf) => spf.clone(),
                None => {
                    return Err(
                        "No project: pass `project` (the .spf file or the project folder) \
                                or start the server with `aurora -C <project> mcp`. project_find \
                                lists the projects under a folder."
                            .to_owned(),
                    );
                }
            },
        };
        arguments.insert("project".to_owned(), json!(project));
        Ok(())
    }

    /// Roda uma ferramenta. `Err` é um pedido que o MCP recusa (ferramenta
    /// desconhecida); um erro da AURORA volta como resultado com `isError`.
    pub(crate) fn call(
        &self,
        name: &str,
        arguments: Value,
        cancel: &CancelToken,
        progress: Option<Arc<Progress>>,
    ) -> Result<Value, String> {
        let Value::Object(mut arguments) = arguments else {
            return Ok(tool_error(
                "invalid_argument",
                "The arguments must be an object",
            ));
        };
        if name == PROJECT_FIND {
            return Ok(self.project_find(&arguments));
        }
        let tool = TOOLS
            .iter()
            .filter(|t| self.allow_update || !t.needs_allow_update())
            .find(|t| t.name == name)
            .ok_or_else(|| format!("Unknown tool: {name}"))?;
        if self.project_tools.contains(tool.name)
            && let Err(message) = self.project_of(&mut arguments)
        {
            return Ok(tool_error("no_project", &message));
        }
        // Um argumento escrito errado seria ignorado em silêncio, e a
        // ferramenta rodaria com o padrão no lugar do que o agente quis.
        let known: Option<Vec<String>> = match tool.flow {
            Some(flow) => flow_fields(flow).map(|fields| {
                let mut fields = fields.to_vec();
                fields.retain(|f| f != "flow");
                fields.push("project".to_owned());
                fields
            }),
            None => method_fields(tool.method).map(<[String]>::to_vec),
        };
        if let Some(known) = known
            && let Some(problem) = unknown_field(&arguments, &known)
        {
            return Ok(tool_error(
                "invalid_argument",
                &format!("Invalid arguments for {name}: {problem}"),
            ));
        }
        let params = match tool.flow {
            Some(flow) => {
                let project = arguments.remove("project").unwrap_or(Value::Null);
                arguments.insert("flow".to_owned(), json!(flow));
                // Sem a saída ao vivo: o agente lê o resultado, e o `vvp`
                // roda com buffer, que é mais rápido.
                json!({ "project": project, "request": Value::Object(arguments), "output": false })
            }
            None => Value::Object(arguments),
        };
        let request = match parse_request(tool.method, Some(params)) {
            Ok(request) => request,
            Err(error) => {
                return Ok(tool_error(
                    "invalid_argument",
                    &format!("Invalid arguments for {name}: {error}"),
                ));
            }
        };
        let sink: Option<Emit> = progress.map(|progress| {
            Arc::new(move |notification: Notification| progress.report(&notification)) as Emit
        });
        match self
            .server
            .handle_with(&json!(name), request, cancel.clone(), sink)
        {
            Ok(response) => {
                let mut value = serde_json::to_value(&response).unwrap_or(Value::Null);
                let text = summary::summarize(tool.method, &value);
                trim_step_output(&mut value, STEP_OUTPUT_BYTES);
                Ok(json!({
                    "content": [{ "type": "text", "text": text }],
                    "structuredContent": value,
                }))
            }
            Err(failure) => {
                let info = failure.into_info();
                let mut text = format!("{} ({})", info.message, info.code);
                if let Some(hint) = &info.hint {
                    text.push_str("\nHint: ");
                    text.push_str(hint);
                }
                Ok(json!({
                    "content": [{ "type": "text", "text": text }],
                    "structuredContent": { "error": info },
                    "isError": true,
                }))
            }
        }
    }

    /// `project_find`: os `.spf` abaixo de uma pasta.
    fn project_find(&self, arguments: &Map<String, Value>) -> Value {
        let depth = arguments
            .get("depth")
            .and_then(Value::as_u64)
            .unwrap_or(FIND_DEPTH);
        let dir = match arguments.get("dir").and_then(Value::as_str) {
            Some(dir) if !dir.trim().is_empty() => {
                match crate::settings::absolute(Utf8Path::new(dir)) {
                    Ok(dir) => dir,
                    Err(error) => return tool_error("invalid_argument", &error.to_string()),
                }
            }
            _ => match std::env::current_dir()
                .ok()
                .and_then(|d| Utf8PathBuf::from_path_buf(d).ok())
            {
                Some(dir) => dir,
                None => return tool_error("io", "The current folder is not usable"),
            },
        };
        if !dir.is_dir() {
            return tool_error("invalid_argument", &format!("{dir} is not a folder"));
        }
        let mut found = Vec::new();
        find_spf(&dir, depth, &mut found);
        found.sort();
        let text = if found.is_empty() {
            format!("Na AURORA project (.spf) under {dir} within {depth} level(s).\n")
        } else {
            let mut text = format!("{} project(s) under {dir}:\n", found.len());
            for spf in &found {
                text.push_str("  ");
                text.push_str(spf.as_str());
                text.push('\n');
            }
            text
        };
        json!({
            "content": [{ "type": "text", "text": text }],
            "structuredContent": { "projects": found },
        })
    }
}

/// O progresso de uma chamada, para `notifications/progress`.
pub(crate) struct Progress {
    out: Writer,
    token: Value,
    count: AtomicU64,
}

impl Progress {
    fn report(&self, notification: &Notification) {
        let message = match progress_message(notification) {
            Some(message) => message,
            None => return,
        };
        let progress = self.count.fetch_add(1, Ordering::SeqCst) + 1;
        self.out.send(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": { "progressToken": self.token, "progress": progress, "message": message },
        }));
    }
}

/// A linha de progresso de uma notificação do core; `None` para o que não
/// interessa ao agente (os eventos linha a linha).
fn progress_message(notification: &Notification) -> Option<String> {
    let value = serde_json::to_value(notification).ok()?;
    match value.get("type").and_then(Value::as_str)? {
        "phase" => Some(format!(
            "phase: {}",
            value.get("phase").and_then(Value::as_str).unwrap_or("?")
        )),
        "build" => {
            let result = value.get("result")?;
            Some(format!(
                "build {}: {}",
                result
                    .get("processor")
                    .and_then(Value::as_str)
                    .unwrap_or("?"),
                result.get("status").and_then(Value::as_str).unwrap_or("?")
            ))
        }
        "install" => {
            // A linha do console, quando o servidor a mandou.
            if let Some(line) = value.get("line").and_then(Value::as_str) {
                return Some(line.trim().to_owned());
            }
            let event = value.get("event")?;
            match event.get("event").and_then(Value::as_str)? {
                "chunk" => Some(format!(
                    "chunk {}/{}: {}",
                    event.get("index").and_then(Value::as_u64).unwrap_or(0),
                    event.get("count").and_then(Value::as_u64).unwrap_or(0),
                    event
                        .get("components")
                        .and_then(Value::as_array)
                        .map(|c| c
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", "))
                        .filter(|c| !c.is_empty())
                        .unwrap_or_else(|| "AURORA".to_owned())
                )),
                "replacing" => Some("replacing files".to_owned()),
                "verifying" => Some("verifying the executables".to_owned()),
                "fallback" | "warning" => event
                    .get("reason")
                    .or_else(|| event.get("message"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                "fallback_download" => event
                    .get("url")
                    .and_then(Value::as_str)
                    .map(|url| format!("downloading {url}")),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Os `.spf` abaixo de `dir`, até `depth` níveis, fora das pastas de
/// [`FIND_SKIP`].
fn find_spf(dir: &Utf8Path, depth: u64, into: &mut Vec<Utf8PathBuf>) {
    let Ok(entries) = dir.read_dir_utf8() else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if depth > 0 && !FIND_SKIP.contains(&entry.file_name()) {
                find_spf(path, depth - 1, into);
            }
        } else if path.extension() == Some("spf") {
            into.push(path.to_owned());
        }
    }
}

/// Um resultado de ferramenta que é erro da AURORA (ou do adaptador).
fn tool_error(code: &str, message: &str) -> Value {
    json!({
        "content": [{ "type": "text", "text": format!("{message} ({code})") }],
        "structuredContent": { "error": { "code": code, "message": message } },
        "isError": true,
    })
}

/// Os nomes em `"$ref": "#/$defs/<nome>"` dentro de `value`.
fn collect_refs(value: &Value, into: &mut BTreeSet<String>) {
    match value {
        Value::Object(fields) => {
            for (key, field) in fields {
                if key == "$ref"
                    && let Some(name) = field.as_str().and_then(|r| r.strip_prefix("#/$defs/"))
                {
                    into.insert(name.to_owned());
                } else {
                    collect_refs(field, into);
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|i| collect_refs(i, into)),
        _ => {}
    }
}

/// Uma doc do Rust como descrição: sem os links (`` [`X`] `` vira `X`) e numa
/// linha só.
fn clean(doc: &str) -> String {
    let mut text = String::with_capacity(doc.len());
    let mut rest = doc;
    while let Some(start) = rest.find("[`") {
        text.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find("`]") {
            Some(end) => {
                let name = &after[..end];
                text.push_str(name.rsplit("::").next().unwrap_or(name));
                rest = &after[end + 2..];
            }
            None => {
                text.push_str("[`");
                rest = after;
            }
        }
    }
    text.push_str(rest);
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::Receiver;

    use super::*;

    fn server(
        allow_update: bool,
        default_project: Option<Utf8PathBuf>,
    ) -> (Arc<Mcp>, Receiver<String>) {
        let args = ToolchainArgs {
            toolchain: None,
            compiler: None,
            quartus: None,
        };
        let (out, rx) = Writer::channel();
        (
            Arc::new(Mcp::new(
                Server::new(args, Writer::silent()),
                out,
                allow_update,
                default_project,
            )),
            rx,
        )
    }

    fn next(rx: &Receiver<String>) -> Value {
        let line = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("uma mensagem");
        serde_json::from_str(&line).expect("JSON")
    }

    fn line(mcp: &Arc<Mcp>, rx: &Receiver<String>, text: &str) -> Value {
        mcp.handle_line(text);
        next(rx)
    }

    fn call(mcp: &Arc<Mcp>, rx: &Receiver<String>, id: u32, name: &str, arguments: Value) -> Value {
        line(
            mcp,
            rx,
            &json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string(),
        )
    }

    fn text_of(reply: &Value) -> &str {
        reply["result"]["content"][0]["text"].as_str().unwrap_or("")
    }

    #[test]
    fn initialize_ping_and_unknown_methods() {
        let (mcp, rx) = server(false, None);
        let reply = line(
            &mcp,
            &rx,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"t"}}}"#,
        );
        assert_eq!(reply["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(reply["result"]["serverInfo"]["name"], "aurora");
        assert!(reply["result"]["capabilities"]["tools"].is_object());
        assert!(
            reply["result"]["instructions"]
                .as_str()
                .unwrap()
                .contains("no default project")
        );
        mcp.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
        mcp.handle_line(
            r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":99}}"#,
        );
        assert!(rx.recv_timeout(Duration::from_millis(100)).is_err());
        assert_eq!(
            line(&mcp, &rx, r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#)["result"],
            json!({})
        );
        assert_eq!(
            line(
                &mcp,
                &rx,
                r#"{"jsonrpc":"2.0","id":3,"method":"resources/list"}"#
            )["error"]["code"],
            -32601
        );
        assert_eq!(line(&mcp, &rx, "nope")["error"]["code"], -32700);
    }

    #[test]
    fn the_tools_are_the_methods_marked_for_agents() {
        // Cada método com a marcação `agent` ou `agent_update` no protocolo
        // vira ferramenta (o flow.run, uma por fluxo), e nenhuma ferramenta
        // usa um método sem ela.
        let marked: BTreeSet<&str> = aurora_protocol::METHOD_INFO
            .iter()
            .filter(|info| info.agent != Agent::No)
            .map(|info| info.name)
            .collect();
        let used: BTreeSet<&str> = TOOLS.iter().map(|t| t.method).collect();
        assert_eq!(marked, used);
    }

    #[test]
    fn every_tool_has_a_schema_from_the_protocol() {
        let (mcp, _rx) = server(false, None);
        let tools = mcp.tools();
        assert_eq!(
            tools.len(),
            TOOLS.iter().filter(|t| !t.needs_allow_update()).count() + 1,
            "toda ferramenta acha o método dela no schema, mais project_find"
        );
        for tool in &tools {
            let name = tool["name"].as_str().unwrap();
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
                "{name}"
            );
            assert!(!tool["description"].as_str().unwrap().is_empty(), "{name}");
            assert!(
                !tool["description"].as_str().unwrap().contains("[`"),
                "{name}: links na descrição"
            );
            let schema = &tool["inputSchema"];
            assert_eq!(schema["type"], "object", "{name}");
            assert!(schema["properties"].is_object(), "{name}");
            // O projeto nunca é obrigatório: o padrão do servidor cobre.
            if let Some(required) = schema["required"].as_array() {
                assert!(!required.iter().any(|r| r == "project"), "{name}");
            }
        }
        assert_eq!(tools[1]["name"], PROJECT_FIND);
        let build = tools.iter().find(|t| t["name"] == "build").unwrap();
        let props = build["inputSchema"]["properties"].as_object().unwrap();
        assert!(props.contains_key("project") && props.contains_key("processors"));
        assert!(!props.contains_key("flow"));
        assert!(
            props["project"]["description"]
                .as_str()
                .unwrap()
                .contains("default project")
        );
        let sim = tools.iter().find(|t| t["name"] == "sim").unwrap();
        assert_eq!(sim["inputSchema"]["required"], json!(["simulator"]));
        assert!(sim["inputSchema"]["$defs"]["Simulator"].is_object());
        let add = tools.iter().find(|t| t["name"] == "processor_add").unwrap();
        assert!(add["inputSchema"]["$defs"]["NewProcessor"].is_object());
        assert!(add["inputSchema"]["$defs"]["Language"].is_object());
        assert!(tools.iter().all(|t| t["name"] != "update_apply"));
        assert!(mcp.project_tools.contains("project_status"));
        assert!(mcp.project_tools.contains("sim"));
        assert!(!mcp.project_tools.contains("toolchain_info"));
        assert!(!mcp.project_tools.contains("project_create"));
        let (with_update, _rx) = server(true, None);
        assert!(
            with_update
                .tools()
                .iter()
                .any(|t| t["name"] == "update_apply")
        );
    }

    #[test]
    fn tool_calls_return_a_summary_or_the_aurora_error() {
        let (mcp, rx) = server(false, None);
        let reply = call(&mcp, &rx, 1, "toolchain_info", json!({}));
        let result = &reply["result"];
        assert!(result.get("isError").is_none(), "{reply}");
        assert!(result["structuredContent"]["found"].is_boolean(), "{reply}");
        assert_eq!(result["content"][0]["type"], "text");
        assert!(
            text_of(&reply).starts_with("No bundle found") || text_of(&reply).starts_with("Bundle"),
            "{reply}"
        );

        let missing = std::env::temp_dir().join("aurora-mcp-nao-existe");
        let missing = missing.to_str().unwrap().replace('\\', "/");
        let reply = call(
            &mcp,
            &rx,
            2,
            "project_status",
            json!({ "project": missing }),
        );
        assert_eq!(reply["result"]["isError"], true, "{reply}");
        assert!(
            reply["result"]["structuredContent"]["error"]["code"].is_string(),
            "{reply}"
        );
        assert!(text_of(&reply).contains('('), "{reply}");

        let reply = call(
            &mcp,
            &rx,
            3,
            "build",
            json!({ "processors": "x", "project": missing }),
        );
        assert_eq!(reply["result"]["isError"], true, "{reply}");
        assert!(text_of(&reply).contains("Invalid arguments"), "{reply}");

        let reply = call(&mcp, &rx, 4, "update_apply", json!({}));
        assert_eq!(reply["error"]["code"], -32602, "{reply}");

        // Sem projeto padrão nem campo: um erro que diz o que fazer.
        let reply = call(&mcp, &rx, 5, "project_status", json!({}));
        assert_eq!(reply["result"]["isError"], true, "{reply}");
        assert!(text_of(&reply).contains("project_find"), "{reply}");
    }

    #[test]
    fn the_default_project_and_relative_paths_fill_in_project() {
        let dir = tempfile::tempdir().unwrap();
        let parent = Utf8PathBuf::from_path_buf(dunce::canonicalize(dir.path()).unwrap()).unwrap();
        let (mcp, rx) = server(false, None);
        let reply = call(
            &mcp,
            &rx,
            1,
            "project_create",
            json!({ "parent": parent, "name": "meu" }),
        );
        assert!(reply["result"].get("isError").is_none(), "{reply}");
        assert!(text_of(&reply).starts_with("Project meu at"), "{reply}");
        let spf = parent.join("meu").join("meu.spf");
        assert!(spf.is_file());

        // Com o padrão, project_status sem argumentos lê esse projeto.
        let (with_default, rx2) = server(false, Some(spf.clone()));
        let reply = call(&with_default, &rx2, 2, "project_status", json!({}));
        assert!(reply["result"].get("isError").is_none(), "{reply}");
        assert_eq!(reply["result"]["structuredContent"]["name"], "meu");
        assert!(text_of(&reply).contains("processors: none"), "{reply}");
        let init = line(
            &with_default,
            &rx2,
            r#"{"jsonrpc":"2.0","id":3,"method":"initialize"}"#,
        );
        assert!(
            init["result"]["instructions"]
                .as_str()
                .unwrap()
                .contains("meu.spf")
        );

        // project_find acha o .spf abaixo da pasta.
        let reply = call(&mcp, &rx, 4, PROJECT_FIND, json!({ "dir": parent }));
        let found = &reply["result"]["structuredContent"]["projects"];
        assert_eq!(found.as_array().unwrap().len(), 1, "{reply}");
        assert!(text_of(&reply).contains("meu.spf"));
        let reply = call(
            &mcp,
            &rx,
            5,
            PROJECT_FIND,
            json!({ "dir": parent, "depth": 0 }),
        );
        assert!(text_of(&reply).starts_with("Na AURORA project"), "{reply}");

        // history_list no projeto novo: vazio, com o resumo.
        let reply = call(&mcp, &rx, 6, "history_list", json!({ "project": spf }));
        assert!(text_of(&reply).starts_with("No reports"), "{reply}");
    }

    #[test]
    fn progress_messages_come_from_phases_and_builds() {
        use aurora_core::flows::Phase;
        assert_eq!(
            progress_message(&Notification::Phase {
                phase: Phase::Build
            })
            .as_deref(),
            Some("phase: build")
        );
        assert_eq!(
            progress_message(&Notification::Install {
                event: json!({ "event": "chunk", "index": 1, "count": 3, "components": ["yanc"] }),
                line: None,
            })
            .as_deref(),
            Some("chunk 1/3: yanc")
        );
        assert_eq!(
            progress_message(&Notification::Install {
                event: json!({ "event": "verifying" }),
                line: Some("  Verifying the executables".into()),
            })
            .as_deref(),
            Some("Verifying the executables")
        );
        assert_eq!(
            progress_message(&Notification::Events { events: Vec::new() }),
            None
        );
        let (out, rx) = Writer::channel();
        let progress = Progress {
            out,
            token: json!("t1"),
            count: AtomicU64::new(0),
        };
        progress.report(&Notification::Phase {
            phase: Phase::Simulate,
        });
        let line = next(&rx);
        assert_eq!(line["method"], "notifications/progress");
        assert_eq!(line["params"]["progressToken"], "t1");
        assert_eq!(line["params"]["progress"], 1);
        assert_eq!(line["params"]["message"], "phase: simulate");
    }

    #[test]
    fn a_queued_call_can_be_cancelled_before_it_starts() {
        let (mcp, rx) = server(false, None);
        // Ocupa o worker com um trabalho que espera o portão.
        let gate = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let g = Arc::clone(&gate);
        mcp.enqueue(Box::new(move || {
            while !g.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(5));
            }
        }));
        mcp.handle_line(
            r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"toolchain_info","arguments":{}}}"#,
        );
        assert!(rx.recv_timeout(Duration::from_millis(100)).is_err());
        // O ping responde enquanto a fila está parada.
        assert_eq!(
            line(&mcp, &rx, r#"{"jsonrpc":"2.0","id":8,"method":"ping"}"#)["result"],
            json!({})
        );
        mcp.handle_line(
            r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":7}}"#,
        );
        gate.store(true, Ordering::SeqCst);
        let reply = next(&rx);
        assert_eq!(reply["id"], 7);
        assert_eq!(reply["error"]["code"], -32800, "{reply}");
        mcp.finish();
        assert!(mcp.pending.lock().unwrap().is_empty());
    }

    #[test]
    fn docs_lose_their_links() {
        assert_eq!(
            clean("O retrato do projeto. Resposta: [`ProjectStatus`].\nMais."),
            "O retrato do projeto. Resposta: ProjectStatus. Mais."
        );
        assert_eq!(clean("[`crate::a::B`] e [`C"), "B e [`C");
    }
}
