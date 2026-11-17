//! Uma sessão do Claude Code em modo `--print`, com entrada e saída em JSON
//! por linha (`--input-format stream-json --output-format stream-json`): a
//! pessoa manda mensagens, o Claude Code roda o modelo com as ferramentas
//! do `aurora mcp` e escreve o que acontece, e esta sessão traduz cada linha
//! em um [`AgentEvent`].
//!
//! O que o Claude Code escreve (uma mensagem JSON por linha):
//!
//! - `system`/`init`: a sessão começou, com `session_id`, `model`, as
//!   ferramentas e os servidores MCP. Só sai depois da primeira mensagem
//!   da pessoa: o Claude Code não inicia nada antes de ter o que fazer, então
//!   quem usa a sessão manda a primeira mensagem sem esperar por ele;
//! - `stream_event`: um pedaço da resposta enquanto ela chega (com
//!   `--include-partial-messages`): o texto vai saindo em `text_delta`;
//! - `assistant`: uma mensagem inteira do modelo, com blocos `text` e
//!   `tool_use`;
//! - `user`: o resultado de uma ferramenta (`tool_result`), que o próprio
//!   Claude Code produziu;
//! - `result`: o turno terminou, com o custo, a duração e o `session_id`,
//!   que serve para retomar a conversa (`--resume`).
//!
//! Interromper um turno é mandar `control_request` com `interrupt`, o
//! mesmo que o Claude Agent SDK faz; o Claude Code responde
//! `control_response` e o `result` do turno interrompido.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use aurora_core::ProcessJob;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::locate::Claude;
use crate::{AgentError, Result};

/// O que o assistente pode fazer além das ferramentas da AURORA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// As ferramentas do `aurora mcp` e ler arquivos (`Read`, `Grep`,
    /// `Glob`). Nada é escrito fora do que a AURORA faz.
    #[default]
    AuroraOnly,
    /// Também editar e criar arquivos na pasta do projeto (`Edit`, `Write`,
    /// `MultiEdit`): corrigir o C± ou o Verilog que o usuário pediu.
    Edits,
}

/// Como abrir a sessão.
#[derive(Debug, Clone)]
pub struct SessionOptions {
    /// O Claude Code.
    pub claude: Claude,
    /// A pasta em que o Claude Code roda: a raiz do projeto, para os
    /// caminhos relativos e as edições.
    pub cwd: Utf8PathBuf,
    /// O `aurora` que serve as ferramentas (`aurora mcp`).
    pub aurora: Utf8PathBuf,
    /// O bundle para o `aurora mcp` (`AURORA_TOOLCHAIN`), quando não é o da
    /// instalação dele.
    pub toolchain: Option<Utf8PathBuf>,
    /// O projeto padrão das ferramentas (`aurora mcp --project`).
    pub project: Option<Utf8PathBuf>,
    /// O que o assistente pode fazer.
    pub permission: Permission,
    /// Expor `update_apply` e `install_add` (`aurora mcp --allow-update`).
    pub allow_update: bool,
    /// O modelo (`--model`); `None` é o padrão do Claude Code.
    pub model: Option<String>,
    /// Retomar uma conversa pelo `session_id` de um [`AgentEvent::Done`].
    pub resume: Option<String>,
    /// Instruções a mais, depois das da AURORA ([`SYSTEM_PROMPT`]).
    pub system_prompt: Option<String>,
}

/// As instruções que toda sessão leva (`--append-system-prompt`), com
/// `{project}` trocado pelo projeto ou por "no project is open".
pub const SYSTEM_PROMPT: &str = "You are the AURORA assistant, inside the AURORA tools for Verilog, VHDL \
and SAPHO (C±) processor projects. Current project: {project}. Use the aurora tools \
(mcp__aurora__*) for everything about the project: project_status to see it, build to compile \
the processors with the YANC, check for the Verilog and the VHDL, sim to simulate, synth to synthesize, \
history_show to read a past report, processor_outputs for the values a processor wrote. The \
tools take the project path; it is already set as the default, so you may omit it. Paths in \
tool results are absolute. Answer in the language the user writes in, be concise, and when a \
tool reports errors, cite them as file:line: message.";

/// O que acontece na sessão, na ordem. Em JSON, etiquetado por `type`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum AgentEvent {
    /// A sessão começou.
    Ready {
        /// O identificador da conversa, para retomar depois.
        session_id: String,
        /// O modelo em uso.
        model: Option<String>,
        /// As ferramentas disponíveis (`mcp__aurora__build`, `Read`, ...).
        tools: Vec<String>,
        /// O servidor `aurora` conectou (`None`: o Claude Code não disse).
        aurora_connected: Option<bool>,
    },
    /// Um pedaço do texto da resposta, na ordem.
    Text {
        /// O texto.
        text: String,
    },
    /// O modelo chamou uma ferramenta.
    ToolUse {
        /// O identificador da chamada.
        id: String,
        /// A ferramenta (`mcp__aurora__sim`, `Read`).
        name: String,
        /// Os argumentos.
        input: Value,
    },
    /// Uma ferramenta respondeu.
    ToolResult {
        /// A chamada ([`AgentEvent::ToolUse::id`]).
        id: String,
        /// O conteúdo, como texto.
        content: String,
        /// A ferramenta falhou.
        is_error: bool,
    },
    /// O turno terminou: a resposta está completa (ou foi interrompida).
    Done {
        /// Terminou em erro (`result` com `is_error`).
        is_error: bool,
        /// O texto final, para quem não recebeu os pedaços.
        text: Option<String>,
        /// O custo do turno, em dólares.
        cost_usd: Option<f64>,
        /// A duração, em milissegundos.
        duration_ms: Option<u64>,
        /// Quantas idas ao modelo.
        turns: Option<u64>,
        /// O identificador da conversa.
        session_id: Option<String>,
        /// Tokens de entrada.
        input_tokens: Option<u64>,
        /// Tokens de saída.
        output_tokens: Option<u64>,
    },
    /// Uma linha do stderr do Claude Code (o log dele).
    Log {
        /// A linha.
        line: String,
    },
    /// O processo saiu; nada mais vem.
    Exited {
        /// O código de saída, se o sistema informou.
        code: Option<i32>,
    },
}

/// A sessão aberta.
pub struct Session {
    child: Arc<Mutex<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
    job: Mutex<ProcessJob>,
    /// A pasta com o `mcp-config.json`, viva enquanto a sessão.
    _config: tempfile::TempDir,
    requests: Mutex<u64>,
}

impl Session {
    /// Abre o Claude Code com as opções e devolve a sessão e o canal de
    /// eventos. Nada chega antes da primeira [`Session::send`]: o
    /// [`AgentEvent::Ready`] vem em resposta a ela (ou um
    /// [`AgentEvent::Exited`], se o Claude Code não abriu).
    pub fn start(options: &SessionOptions) -> Result<(Session, Receiver<AgentEvent>)> {
        let config = tempfile::Builder::new()
            .prefix("aurora-assistant.")
            .tempdir()
            .map_err(|e| AgentError::Other(anyhow::Error::new(e).context("Creating a folder")))?;
        let config_path = config.path().join("mcp-config.json");
        std::fs::write(&config_path, mcp_config(options).to_string())?;

        let mut command = Command::new(options.claude.program.as_std_path());
        command.args(&options.claude.prefix);
        command.args(arguments(options));
        command.arg("--mcp-config").arg(&config_path);
        command
            .current_dir(options.cwd.as_std_path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Um Claude Code aberto por outro Claude Code recusa rodar; a sessão
        // do assistente é dele mesma.
        command.env_remove("CLAUDECODE");
        command.env("CLAUDE_CODE_ENTRYPOINT", "aurora");
        aurora_core::hide_console(&mut command);
        let mut job = ProcessJob::new(false);
        let mut child = command.spawn().map_err(|error| AgentError::Spawn {
            program: options.claude.display(),
            reason: error.to_string(),
        })?;
        job.adopt(&child);
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let (tx, rx) = mpsc::channel();
        let child = Arc::new(Mutex::new(child));

        let errors = tx.clone();
        std::thread::Builder::new()
            .name("aurora-assistant-stderr".into())
            .spawn(move || {
                for line in BufReader::new(stderr)
                    .lines()
                    .map_while(std::result::Result::ok)
                {
                    if errors.send(AgentEvent::Log { line }).is_err() {
                        break;
                    }
                }
            })
            .map_err(|e| AgentError::Other(e.into()))?;
        let waiter = Arc::clone(&child);
        std::thread::Builder::new()
            .name("aurora-assistant-stdout".into())
            .spawn(move || {
                let mut parser = Parser::default();
                for line in BufReader::new(stdout)
                    .lines()
                    .map_while(std::result::Result::ok)
                {
                    for event in parser.line(&line) {
                        if tx.send(event).is_err() {
                            return;
                        }
                    }
                }
                let code = waiter
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .wait()
                    .ok()
                    .and_then(|s| s.code());
                let _ = tx.send(AgentEvent::Exited { code });
            })
            .map_err(|e| AgentError::Other(e.into()))?;
        Ok((
            Session {
                child,
                stdin: Mutex::new(Some(stdin)),
                job: Mutex::new(job),
                _config: config,
                requests: Mutex::new(0),
            },
            rx,
        ))
    }

    /// Manda uma mensagem da pessoa. A resposta vem pelos eventos.
    pub fn send(&self, text: &str) -> Result<()> {
        self.write(&json!({
            "type": "user",
            "message": { "role": "user", "content": [{ "type": "text", "text": text }] },
            "parent_tool_use_id": Value::Null,
            "session_id": "",
        }))
    }

    /// Interrompe o turno em andamento: o Claude Code para o modelo e a
    /// ferramenta que roda, e fecha o turno com um `result`.
    pub fn interrupt(&self) -> Result<()> {
        let n = {
            let mut requests = self
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *requests += 1;
            *requests
        };
        self.write(&json!({
            "type": "control_request",
            "request_id": format!("aurora-{n}"),
            "request": { "subtype": "interrupt" },
        }))
    }

    fn write(&self, message: &Value) -> Result<()> {
        let mut guard = self
            .stdin
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(stdin) = guard.as_mut() else {
            return Err(AgentError::Ended);
        };
        let mut line = message.to_string();
        line.push('\n');
        if let Err(error) = stdin
            .write_all(line.as_bytes())
            .and_then(|()| stdin.flush())
        {
            *guard = None;
            return Err(if error.kind() == std::io::ErrorKind::BrokenPipe {
                AgentError::Ended
            } else {
                AgentError::Io(error)
            });
        }
        Ok(())
    }

    /// O processo ainda roda.
    pub fn is_alive(&self) -> bool {
        self.child
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .try_wait()
            .ok()
            .flatten()
            .is_none()
    }

    /// Encerra: fecha a entrada (o Claude Code sai sozinho ao fim do turno)
    /// e, se ele não saiu em `wait`, mata o processo e o que ele iniciou.
    pub fn stop(&self, wait: Duration) {
        *self
            .stdin
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        let until = Instant::now() + wait;
        while Instant::now() < until {
            if !self.is_alive() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        if self.is_alive() {
            let _ = self
                .child
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .kill();
        }
        self.job
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .kill();
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop(Duration::from_millis(500));
    }
}

/// O `mcp-config.json`: o `aurora mcp` como servidor `aurora`.
fn mcp_config(options: &SessionOptions) -> Value {
    let mut args = vec!["mcp".to_owned()];
    if options.allow_update {
        args.push("--allow-update".to_owned());
    }
    if let Some(project) = &options.project {
        args.push("--project".to_owned());
        args.push(project.to_string());
    }
    let mut server = json!({ "command": options.aurora, "args": args });
    if let Some(toolchain) = &options.toolchain {
        server["env"] = json!({ "AURORA_TOOLCHAIN": toolchain });
    }
    json!({ "mcpServers": { "aurora": server } })
}

/// Os argumentos do Claude Code, fora o `--mcp-config` (que leva um
/// arquivo).
fn arguments(options: &SessionOptions) -> Vec<String> {
    let mut args: Vec<String> = [
        "--print",
        "--verbose",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--include-partial-messages",
        "--strict-mcp-config",
    ]
    .map(str::to_owned)
    .to_vec();
    let mut allowed = vec!["mcp__aurora", "Read", "Grep", "Glob"];
    if options.permission == Permission::Edits {
        allowed.extend(["Edit", "Write", "MultiEdit"]);
        args.push("--permission-mode".to_owned());
        args.push("acceptEdits".to_owned());
    }
    args.push("--allowedTools".to_owned());
    args.push(allowed.join(","));
    if let Some(model) = &options.model {
        args.push("--model".to_owned());
        args.push(model.clone());
    }
    if let Some(resume) = &options.resume {
        args.push("--resume".to_owned());
        args.push(resume.clone());
    }
    let project = options
        .project
        .as_ref()
        .map_or_else(|| "no project is open".to_owned(), ToString::to_string);
    let mut prompt = SYSTEM_PROMPT.replace("{project}", &project);
    if let Some(extra) = &options.system_prompt {
        prompt.push('\n');
        prompt.push_str(extra);
    }
    args.push("--append-system-prompt".to_owned());
    args.push(prompt);
    args
}

/// Traduz as linhas do Claude Code em eventos, lembrando se o texto da
/// mensagem atual já saiu em pedaços (para não repeti-lo quando a mensagem
/// inteira chega).
#[derive(Default)]
struct Parser {
    streamed: bool,
}

impl Parser {
    fn line(&mut self, line: &str) -> Vec<AgentEvent> {
        if line.trim().is_empty() {
            return Vec::new();
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return vec![AgentEvent::Log {
                line: line.to_owned(),
            }];
        };
        match value.get("type").and_then(Value::as_str) {
            Some("system") if value.get("subtype").and_then(Value::as_str) == Some("init") => {
                let tools = value
                    .get("tools")
                    .and_then(Value::as_array)
                    .map(|t| {
                        t.iter()
                            .filter_map(|v| v.as_str().map(str::to_owned))
                            .collect()
                    })
                    .unwrap_or_default();
                let aurora_connected = value
                    .get("mcp_servers")
                    .and_then(Value::as_array)
                    .and_then(|servers| {
                        servers
                            .iter()
                            .find(|s| s.get("name").and_then(Value::as_str) == Some("aurora"))
                    })
                    .map(|s| s.get("status").and_then(Value::as_str) == Some("connected"));
                vec![AgentEvent::Ready {
                    session_id: text_of(value.get("session_id")),
                    model: value
                        .get("model")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    tools,
                    aurora_connected,
                }]
            }
            Some("stream_event") => {
                let event = value.get("event").cloned().unwrap_or(Value::Null);
                match event.get("type").and_then(Value::as_str) {
                    Some("message_start") => {
                        self.streamed = false;
                        Vec::new()
                    }
                    Some("content_block_delta") => {
                        let delta = event.get("delta");
                        if delta.and_then(|d| d.get("type")).and_then(Value::as_str)
                            == Some("text_delta")
                        {
                            let text = text_of(delta.and_then(|d| d.get("text")));
                            if text.is_empty() {
                                return Vec::new();
                            }
                            self.streamed = true;
                            return vec![AgentEvent::Text { text }];
                        }
                        Vec::new()
                    }
                    _ => Vec::new(),
                }
            }
            Some("assistant") => {
                let mut events = Vec::new();
                let blocks = value
                    .pointer("/message/content")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for block in blocks {
                    match block.get("type").and_then(Value::as_str) {
                        Some("text") if !self.streamed => {
                            let text = text_of(block.get("text"));
                            if !text.is_empty() {
                                events.push(AgentEvent::Text { text });
                            }
                        }
                        Some("tool_use") => events.push(AgentEvent::ToolUse {
                            id: text_of(block.get("id")),
                            name: text_of(block.get("name")),
                            input: block.get("input").cloned().unwrap_or(Value::Null),
                        }),
                        _ => {}
                    }
                }
                self.streamed = false;
                events
            }
            Some("user") => {
                let blocks = value
                    .pointer("/message/content")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                blocks
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("tool_result"))
                    .map(|b| AgentEvent::ToolResult {
                        id: text_of(b.get("tool_use_id")),
                        content: result_text(b.get("content")),
                        is_error: b.get("is_error").and_then(Value::as_bool).unwrap_or(false),
                    })
                    .collect()
            }
            Some("result") => {
                self.streamed = false;
                let usage = value.get("usage");
                vec![AgentEvent::Done {
                    is_error: value
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    text: value
                        .get("result")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    cost_usd: value.get("total_cost_usd").and_then(Value::as_f64),
                    duration_ms: value.get("duration_ms").and_then(Value::as_u64),
                    turns: value.get("num_turns").and_then(Value::as_u64),
                    session_id: value
                        .get("session_id")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    input_tokens: usage
                        .and_then(|u| u.get("input_tokens"))
                        .and_then(Value::as_u64),
                    output_tokens: usage
                        .and_then(|u| u.get("output_tokens"))
                        .and_then(Value::as_u64),
                }]
            }
            // control_response e o que mais vier não têm o que mostrar.
            _ => Vec::new(),
        }
    }
}

fn text_of(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or_default().to_owned()
}

/// O conteúdo de um `tool_result`: texto, ou os blocos de texto juntos.
fn result_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .filter_map(|b| b.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        Some(other) if !other.is_null() => other.to_string(),
        _ => String::new(),
    }
}

/// O caminho de um projeto como vai nas instruções e nas ferramentas.
pub fn project_label(project: Option<&Utf8Path>) -> String {
    project.map_or_else(|| "no project is open".to_owned(), ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> SessionOptions {
        SessionOptions {
            claude: Claude {
                program: "claude".into(),
                prefix: Vec::new(),
            },
            cwd: "/p".into(),
            aurora: "/opt/aurora/bin/aurora".into(),
            toolchain: None,
            project: Some("/p/soma.spf".into()),
            permission: Permission::AuroraOnly,
            allow_update: false,
            model: None,
            resume: None,
            system_prompt: None,
        }
    }

    #[test]
    fn the_arguments_follow_the_permission_and_the_options() {
        let args = arguments(&options());
        assert!(args.contains(&"--print".to_owned()));
        assert!(args.contains(&"--include-partial-messages".to_owned()));
        let allowed = &args[args.iter().position(|a| a == "--allowedTools").unwrap() + 1];
        assert_eq!(allowed, "mcp__aurora,Read,Grep,Glob");
        assert!(!args.contains(&"--permission-mode".to_owned()));
        let prompt = &args[args
            .iter()
            .position(|a| a == "--append-system-prompt")
            .unwrap()
            + 1];
        assert!(prompt.contains("/p/soma.spf"));

        let mut edits = options();
        edits.permission = Permission::Edits;
        edits.model = Some("sonnet".into());
        edits.resume = Some("abc".into());
        edits.system_prompt = Some("Fale em português.".into());
        let args = arguments(&edits);
        let allowed = &args[args.iter().position(|a| a == "--allowedTools").unwrap() + 1];
        assert!(allowed.ends_with(",Edit,Write,MultiEdit"));
        assert!(
            args.windows(2)
                .any(|w| w == ["--permission-mode", "acceptEdits"])
        );
        assert!(args.windows(2).any(|w| w == ["--model", "sonnet"]));
        assert!(args.windows(2).any(|w| w == ["--resume", "abc"]));
        let prompt = &args[args
            .iter()
            .position(|a| a == "--append-system-prompt")
            .unwrap()
            + 1];
        assert!(prompt.ends_with("Fale em português."));
    }

    #[test]
    fn the_mcp_config_points_at_aurora_mcp() {
        let mut opts = options();
        opts.toolchain = Some("/bundle".into());
        opts.allow_update = true;
        let config = mcp_config(&opts);
        let aurora = &config["mcpServers"]["aurora"];
        assert_eq!(aurora["command"], "/opt/aurora/bin/aurora");
        assert_eq!(
            aurora["args"],
            json!(["mcp", "--allow-update", "--project", "/p/soma.spf"])
        );
        assert_eq!(aurora["env"]["AURORA_TOOLCHAIN"], "/bundle");
        let plain = mcp_config(&options());
        assert!(plain["mcpServers"]["aurora"].get("env").is_none());
    }

    #[test]
    fn the_stream_is_translated_into_events() {
        let mut parser = Parser::default();
        let init = parser.line(r#"{"type":"system","subtype":"init","session_id":"s1","model":"claude-x","tools":["mcp__aurora__build","Read"],"mcp_servers":[{"name":"aurora","status":"connected"}]}"#);
        assert_eq!(
            init,
            [AgentEvent::Ready {
                session_id: "s1".into(),
                model: Some("claude-x".into()),
                tools: vec!["mcp__aurora__build".into(), "Read".into()],
                aurora_connected: Some(true),
            }]
        );
        // O texto em pedaços, depois a mensagem inteira sem repetir.
        assert!(
            parser
                .line(r#"{"type":"stream_event","event":{"type":"message_start"}}"#)
                .is_empty()
        );
        assert_eq!(
            parser.line(r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Olá"}}}"#),
            [AgentEvent::Text { text: "Olá".into() }]
        );
        let whole = parser.line(r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Olá"},{"type":"tool_use","id":"t1","name":"mcp__aurora__build","input":{"processors":[]}}]}}"#);
        assert_eq!(
            whole,
            [AgentEvent::ToolUse {
                id: "t1".into(),
                name: "mcp__aurora__build".into(),
                input: json!({ "processors": [] }),
            }]
        );
        // O resultado da ferramenta, em blocos de texto.
        assert_eq!(
            parser.line(r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":[{"type":"text","text":"Build succeeded"}],"is_error":false}]}}"#),
            [AgentEvent::ToolResult {
                id: "t1".into(),
                content: "Build succeeded".into(),
                is_error: false,
            }]
        );
        // Sem pedaços (um cliente sem partial messages), o texto vem da
        // mensagem inteira.
        assert_eq!(
            parser.line(
                r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Pronto."}]}}"#
            ),
            [AgentEvent::Text {
                text: "Pronto.".into()
            }]
        );
        let done = parser.line(r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":1200,"num_turns":2,"result":"Pronto.","session_id":"s1","total_cost_usd":0.01,"usage":{"input_tokens":10,"output_tokens":5}}"#);
        assert_eq!(
            done,
            [AgentEvent::Done {
                is_error: false,
                text: Some("Pronto.".into()),
                cost_usd: Some(0.01),
                duration_ms: Some(1200),
                turns: Some(2),
                session_id: Some("s1".into()),
                input_tokens: Some(10),
                output_tokens: Some(5),
            }]
        );
        assert!(
            parser
                .line(r#"{"type":"control_response","response":{}}"#)
                .is_empty()
        );
        assert_eq!(
            parser.line("not json"),
            [AgentEvent::Log {
                line: "not json".into()
            }]
        );
    }

    #[test]
    fn events_serialize_with_a_type_tag() {
        let value = serde_json::to_value(AgentEvent::Text { text: "x".into() }).unwrap();
        assert_eq!(value, json!({ "type": "text", "text": "x" }));
        let value = serde_json::to_value(AgentEvent::Exited { code: Some(0) }).unwrap();
        assert_eq!(value["type"], "exited");
    }
}
