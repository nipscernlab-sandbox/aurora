//! `aurora serve` de ponta a ponta: um cliente de verdade falando JSON-RPC por
//! stdin e stdout com o binário.
//!
//! Os testes que rodam ferramentas precisam de um bundle em
//! `AURORA_TEST_BUNDLE`; sem ele, avisam e passam, como os de `cli.rs`.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use serde_json::{Value, json};

/// Um `aurora serve` filho, com um leitor do stdout numa thread.
struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<Value>,
    next_id: u64,
    /// Respostas que chegaram antes de serem esperadas (os pedidos rodam
    /// em paralelo e respondem fora de ordem).
    replies: std::cell::RefCell<Vec<Value>>,
}

impl Client {
    fn start(toolchain: Option<&str>) -> Client {
        Client::spawn(toolchain, None)
    }

    /// Com a pasta de configuração da AURORA (a do `experimental.json`) em
    /// `config`: o teste não lê nem grava a do usuário.
    fn start_in(config: &Utf8Path) -> Client {
        Client::spawn(None, Some(config))
    }

    fn spawn(toolchain: Option<&str>, config: Option<&Utf8Path>) -> Client {
        let exe = assert_cmd::cargo::cargo_bin("aurora");
        let mut command = Command::new(exe);
        if let Some(config) = config {
            command
                .env("APPDATA", config)
                .env("XDG_CONFIG_HOME", config)
                .env("HOME", config);
        }
        command
            .arg("serve")
            .env_remove("AURORA_TOOLCHAIN")
            .env_remove("AURORA_COMPILER")
            .env_remove("AURORA_TEST_BUNDLE")
            .env("NO_COLOR", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        if let Some(dir) = toolchain {
            command.env("AURORA_TOOLCHAIN", dir);
        }
        let mut child = command.spawn().expect("aurora serve");
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let value: Value = serde_json::from_str(&line)
                    .unwrap_or_else(|e| panic!("linha que não é JSON ({e}): {line}"));
                if tx.send(value).is_err() {
                    break;
                }
            }
        });
        Client {
            stdin: child.stdin.take(),
            child,
            lines: rx,
            next_id: 0,
            replies: std::cell::RefCell::new(Vec::new()),
        }
    }

    fn send_raw(&mut self, line: &str) {
        let stdin = self.stdin.as_mut().expect("stdin aberto");
        stdin.write_all(line.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
    }

    /// Manda um pedido e devolve o `id` dele.
    fn request(&mut self, method: &str, params: Value) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let mut message = json!({ "jsonrpc": "2.0", "id": id, "method": method });
        if !params.is_null() {
            message["params"] = params;
        }
        self.send_raw(&message.to_string());
        id
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send_raw(&json!({ "jsonrpc": "2.0", "method": method, "params": params }).to_string());
    }

    /// A próxima linha, em até `timeout`.
    fn next(&self, timeout: Duration) -> Option<Value> {
        match self.lines.recv_timeout(timeout) {
            Ok(value) => Some(value),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => None,
        }
    }

    /// Espera a resposta de `id`, guardando as notificações que vierem
    /// antes dela (as de outros pedidos também).
    fn response(&self, id: u64) -> (Value, Vec<Value>) {
        let mut notifications = Vec::new();
        if let Some(at) = self
            .replies
            .borrow()
            .iter()
            .position(|r| r["id"] == json!(id))
        {
            return (self.replies.borrow_mut().remove(at), notifications);
        }
        let until = Instant::now() + Duration::from_secs(120);
        while Instant::now() < until {
            let Some(line) = self.next(Duration::from_secs(1)) else {
                continue;
            };
            if line["id"] == json!(id) {
                return (line, notifications);
            }
            if line.get("id").is_some() {
                self.replies.borrow_mut().push(line);
            } else {
                notifications.push(line);
            }
        }
        panic!("sem resposta para {id}; notificações: {notifications:?}");
    }

    fn call(&mut self, method: &str, params: Value) -> (Value, Vec<Value>) {
        let id = self.request(method, params);
        self.response(id)
    }

    fn initialize(&mut self) -> Value {
        let (reply, _) = self.call("initialize", json!({ "client": "teste" }));
        assert!(reply.get("error").is_none(), "{reply}");
        reply["result"].clone()
    }

    /// Fecha a entrada e espera o processo sair.
    fn close_and_wait(mut self) -> (i32, Vec<Value>) {
        drop(self.stdin.take());
        let until = Instant::now() + Duration::from_secs(10);
        let mut rest = Vec::new();
        let code = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status.code().unwrap_or(-1);
            }
            assert!(
                Instant::now() < until,
                "aurora serve não saiu com a entrada fechada"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        while let Some(line) = self.next(Duration::from_millis(200)) {
            rest.push(line);
        }
        (code, rest)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn tempdir() -> (tempfile::TempDir, Utf8PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dunce::canonicalize(dir.path()).unwrap()).unwrap();
    (dir, path)
}

fn copy_tree(from: &Utf8Path, to: &Utf8Path, depth: usize) {
    std::fs::create_dir_all(to).unwrap();
    for entry in from.read_dir_utf8().unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let output_folder = depth == 1 && from.file_name() == Some(name);
        if name == ".aurora8" || output_folder {
            continue;
        }
        let target = to.join(name);
        if entry.file_type().unwrap().is_dir() {
            copy_tree(entry.path(), &target, depth + 1);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn example(name: &str) -> (tempfile::TempDir, Utf8PathBuf) {
    let (guard, dir) = tempdir();
    let from = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    let root = dir.join(name);
    copy_tree(&from, &root, 0);
    (guard, root)
}

fn env_or_skip(var: &str) -> Option<String> {
    let value = std::env::var(var).ok();
    if value.is_none() {
        assert!(std::env::var_os("CI").is_none(), "CI sem {var}");
        eprintln!("PULADO: defina {var}");
    }
    value
}

#[test]
fn handshake_errors_and_exit_on_eof() {
    let mut client = Client::start(None);
    // Antes do initialize só o initialize.
    let (reply, _) = client.call("toolchain.info", Value::Null);
    assert_eq!(reply["error"]["code"], -32002, "{reply}");
    let info = client.initialize();
    assert_eq!(info["aurora"], env!("CARGO_PKG_VERSION"));
    assert_eq!(info["protocol"], 2);
    assert!(info["methods"].as_array().unwrap().len() > 30);
    // JSON inválido e pedidos malformados não derrubam a sessão.
    client.send_raw("isto nao e json");
    let line = client.next(Duration::from_secs(5)).unwrap();
    assert_eq!(line["error"]["code"], -32700, "{line}");
    assert!(line["id"].is_null());
    let (reply, _) = client.call("nothing.here", Value::Null);
    assert_eq!(reply["error"]["code"], -32601, "{reply}");
    let (reply, _) = client.call("project.status", json!({}));
    assert_eq!(reply["error"]["code"], -32602, "{reply}");
    // Um erro da AURORA: o ErrorInfo em data, com o código do core.
    let (_guard, dir) = tempdir();
    let missing = dir.join("nao-existe");
    let (reply, _) = client.call("project.status", json!({ "project": missing }));
    assert_eq!(reply["error"]["code"], -32000, "{reply}");
    assert!(reply["error"]["data"]["code"].is_string(), "{reply}");
    // Só leitura, sem bundle: funciona.
    let (reply, _) = client.call("project.check_name", json!({ "name": "bom_nome" }));
    assert_eq!(reply["result"]["ok"], true, "{reply}");
    let (reply, _) = client.call("project.check_name", json!({ "name": "1 ruim" }));
    assert_eq!(reply["result"]["ok"], false, "{reply}");
    assert_eq!(reply["result"]["error"]["code"], "invalid_name", "{reply}");
    let (reply, _) = client.call("experimental.list", Value::Null);
    assert!(reply["result"]["components"].is_array(), "{reply}");
    // Fechar a entrada encerra, com a notificação shutdown antes.
    let (code, rest) = client.close_and_wait();
    assert_eq!(code, 0);
    let shutdown = rest.iter().find(|l| l["method"] == "shutdown");
    assert!(shutdown.is_some(), "{rest:?}");
    assert_eq!(shutdown.unwrap()["params"]["reason"], "requested");
}

/// Espera o processo sair e devolve o código.
fn wait_exit(client: &mut Client) -> Option<i32> {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = client.child.try_wait().unwrap() {
            return status.code();
        }
        assert!(Instant::now() < until, "não saiu");
        std::thread::sleep(Duration::from_millis(20));
    }
}

const MCP_INITIALIZE: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"teste"}}}"#;

#[test]
fn the_same_server_answers_mcp_when_the_client_speaks_it() {
    let (_guard, config) = tempdir();
    // O idioma MCP é o do assistente, experimental: desligado, o
    // initialize volta com o erro e a dica, e o servidor sai.
    let mut client = Client::start_in(&config);
    client.send_raw(MCP_INITIALIZE);
    let reply = client.next(Duration::from_secs(10)).expect("a resposta");
    assert_eq!(reply["id"], 1, "{reply}");
    assert_eq!(
        reply["error"]["data"]["code"], "experimental_disabled",
        "{reply}"
    );
    assert!(
        reply["error"]["message"]
            .as_str()
            .unwrap()
            .contains("aurora experimental enable assistant"),
        "{reply}"
    );
    drop(client.stdin.take());
    assert_eq!(wait_exit(&mut client), Some(2));

    // Ligado, o mesmo comando responde em MCP.
    let enabled = Command::new(assert_cmd::cargo::cargo_bin("aurora"))
        .args(["experimental", "enable", "assistant"])
        .env("APPDATA", &config)
        .env("XDG_CONFIG_HOME", &config)
        .env("HOME", &config)
        .output()
        .unwrap();
    assert!(enabled.status.success(), "{enabled:?}");
    let mut client = Client::start_in(&config);
    client.send_raw(MCP_INITIALIZE);
    let reply = client.next(Duration::from_secs(10)).expect("a resposta");
    assert_eq!(reply["result"]["protocolVersion"], "2025-06-18", "{reply}");
    assert_eq!(reply["result"]["serverInfo"]["name"], "aurora", "{reply}");
    client.send_raw(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
    let reply = client.next(Duration::from_secs(10)).expect("a lista");
    let tools = reply["result"]["tools"].as_array().expect("tools");
    assert!(tools.iter().any(|t| t["name"] == "build"), "{reply}");
    // Sem --allow-update, nada que mude a instalação.
    assert!(
        !tools.iter().any(|t| t["name"] == "update_apply"),
        "{reply}"
    );
    let (code, _) = client.close_and_wait();
    assert_eq!(code, 0);

    // E o protocolo da AURORA continua no mesmo comando.
    let mut client = Client::start_in(&config);
    let info = client.initialize();
    assert_eq!(info["protocol"], 2, "{info}");
    let (code, _) = client.close_and_wait();
    assert_eq!(code, 0);
}

#[test]
fn shutdown_request_ends_the_session() {
    let mut client = Client::start(None);
    client.initialize();
    let (reply, _) = client.call("shutdown", Value::Null);
    assert!(reply["result"].is_null(), "{reply}");
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = client.child.try_wait().unwrap() {
            assert_eq!(status.code(), Some(0));
            break;
        }
        assert!(Instant::now() < until, "não saiu depois do shutdown");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn project_methods_create_add_and_report_without_a_bundle() {
    let mut client = Client::start(None);
    client.initialize();
    let (_guard, dir) = tempdir();
    let (reply, _) = client.call("project.create", json!({ "parent": dir, "name": "meu" }));
    assert!(reply.get("error").is_none(), "{reply}");
    let root = dir.join("meu");
    assert_eq!(reply["result"]["name"], "meu");
    assert!(root.join("meu.spf").is_file());
    let file = root.join("contador.v");
    let (reply, _) = client.call(
        "project.add",
        json!({ "project": root, "files": [file], "testbench": false }),
    );
    assert!(reply.get("error").is_none(), "{reply}");
    assert!(file.is_file(), "o arquivo novo é criado do modelo");
    let (reply, _) = client.call("project.status", json!({ "project": root }));
    let status = &reply["result"];
    assert_eq!(status["name"], "meu");
    assert!(status["top_level"].is_string(), "{status}");
    let (reply, _) = client.call("project.top", json!({ "project": root }));
    assert_eq!(reply["result"]["module"], "contador", "{reply}");
    let (reply, _) = client.call("history.list", json!({ "project": root }));
    assert_eq!(reply["result"], json!([]));
    let (reply, _) = client.call(
        "project.remove",
        json!({ "project": root, "files": [file] }),
    );
    assert_eq!(
        reply["result"]["removed"].as_array().unwrap().len(),
        1,
        "{reply}"
    );
    assert!(file.is_file(), "tirar do projeto não apaga");
    // Sem bundle, um fluxo não roda, mas o erro é um erro, não uma queda.
    let (reply, _) = client.call(
        "flow.run",
        json!({ "project": root, "request": { "flow": "build" } }),
    );
    if reply.get("error").is_some() {
        assert_eq!(reply["error"]["code"], -32000, "{reply}");
    }
    let (code, _) = client.close_and_wait();
    assert_eq!(code, 0);
}

#[test]
fn flows_notify_progress_and_cancel_returns_a_cancelled_outcome() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let mut client = Client::start(Some(&tc));
    client.initialize();
    let (_guard, root) = example("soma");
    // Um build inteiro: fases, resultados por processador e o outcome.
    let (reply, notifications) = client.call(
        "flow.run",
        json!({ "project": root, "request": { "flow": "build" } }),
    );
    assert!(reply.get("error").is_none(), "{reply}");
    let outcome = &reply["result"];
    assert_eq!(outcome["status"], "succeeded", "{outcome}");
    assert_eq!(outcome["flow"], "build");
    assert_eq!(
        outcome["builds"].as_array().map(Vec::len),
        Some(2),
        "{outcome}"
    );
    let methods: Vec<&str> = notifications
        .iter()
        .filter_map(|n| n["method"].as_str())
        .collect();
    assert!(methods.contains(&"phase"), "{methods:?}");
    assert!(methods.contains(&"build"), "{methods:?}");
    assert!(
        notifications.iter().all(|n| n["params"]["id"] == 1 + 1),
        "toda notificação leva o id do pedido: {notifications:?}"
    );
    // O status do projeto em paralelo com um fluxo do mesmo projeto: só
    // leitura não espera a fila.
    let sim = client.request(
        "flow.run",
        json!({ "project": root, "request": { "flow": "simulate", "simulator": "icarus" }, "output": true }),
    );
    let status = client.request("project.status", json!({ "project": root }));
    let (reply, _) = client.response(status);
    assert_eq!(reply["result"]["name"], "soma", "{reply}");
    // Cancelar o que roda: o resultado volta com status cancelled (ou o
    // fluxo já tinha acabado, e aí veio inteiro).
    client.notify("$/cancel", json!({ "id": sim }));
    let (reply, _) = client.response(sim);
    assert!(reply.get("error").is_none(), "{reply}");
    assert!(
        matches!(
            reply["result"]["status"].as_str(),
            Some("cancelled" | "succeeded" | "failed")
        ),
        "{reply}"
    );
    // Dois pedidos do mesmo projeto, o segundo cancelado na fila.
    let first = client.request(
        "flow.run",
        json!({ "project": root, "request": { "flow": "build" } }),
    );
    let second = client.request(
        "flow.run",
        json!({ "project": root, "request": { "flow": "check" } }),
    );
    client.notify("$/cancel", json!({ "id": second }));
    let (reply, _) = client.response(first);
    assert!(reply.get("error").is_none(), "{reply}");
    let (reply, _) = client.response(second);
    // Cancelado antes de começar (-32001) ou, se já tinha começado, o
    // outcome com cancelled.
    if let Some(error) = reply.get("error") {
        assert_eq!(error["code"], -32001, "{reply}");
    } else {
        assert_eq!(reply["result"]["status"], "cancelled", "{reply}");
    }
    let (reply, _) = client.call("history.list", json!({ "project": root }));
    assert!(!reply["result"].as_array().unwrap().is_empty(), "{reply}");
    let (reply, _) = client.call("toolchain.info", Value::Null);
    assert_eq!(reply["result"]["found"], true, "{reply}");
    let (code, _) = client.close_and_wait();
    assert_eq!(code, 0);
}

#[test]
fn schema_flag_prints_both_views() {
    let out = assert_cmd::Command::cargo_bin("aurora")
        .unwrap()
        .args(["serve", "--schema"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&out).unwrap();
    assert!(value["request"]["oneOf"].is_array());
    assert!(value["response"]["$defs"]["FlowOutcome"].is_object());
}

#[test]
fn version_control_creates_commits_and_switches_through_the_server() {
    if Command::new("git").arg("--version").output().is_err() {
        eprintln!("no git, skipping");
        return;
    }
    let (_config_guard, config) = tempdir();
    let (_guard, dir) = tempdir();
    let mut client = Client::start_in(&config);
    client.initialize();
    let (reply, _) = client.call("project.create", json!({ "parent": dir, "name": "versao" }));
    assert!(reply.get("error").is_none(), "{reply}");
    let project = dir.join("versao");

    // Desligado, o recurso experimental recusa.
    let (reply, _) = client.call("git.status", json!({ "project": project }));
    assert_eq!(
        reply["error"]["data"]["code"], "experimental_disabled",
        "{reply}"
    );
    let (reply, _) = client.call(
        "experimental.set",
        json!({ "feature": "git", "enabled": true }),
    );
    assert!(reply["result"].is_object(), "{reply}");

    let (reply, _) = client.call("git.status", json!({ "project": project }));
    assert!(reply["result"]["version"].is_string(), "{reply}");
    assert!(reply["result"]["repository"].is_null(), "{reply}");

    let (reply, _) = client.call("git.init", json!({ "project": project }));
    assert_eq!(reply["result"]["ignored"][0], ".aurora8/", "{reply}");
    let (reply, _) = client.call(
        "git.set_identity",
        json!({ "project": project, "name": "Ana", "email": "ana@example.com" }),
    );
    assert!(reply["error"].is_null(), "{reply}");

    let (reply, _) = client.call("git.status", json!({ "project": project }));
    let files = reply["result"]["repository"]["files"]
        .as_array()
        .expect("files");
    assert!(!files.is_empty(), "{reply}");
    assert!(files.iter().all(|f| f["in_project"] == true), "{reply}");

    let (reply, _) = client.call(
        "git.commit",
        json!({ "project": project, "summary": "Projeto novo", "description": "Feito pela AURORA" }),
    );
    assert_eq!(reply["result"]["summary"], "Projeto novo", "{reply}");
    let (reply, _) = client.call("git.log", json!({ "project": project }));
    let commits = reply["result"]["commits"].as_array().expect("commits");
    assert_eq!(commits.len(), 1, "{reply}");
    assert_eq!(commits[0]["local"], true, "{reply}");
    let oid = commits[0]["oid"].as_str().unwrap().to_owned();
    let (reply, _) = client.call(
        "git.commit_files",
        json!({ "project": project, "oid": oid }),
    );
    assert!(reply["result"]["parent"].is_null(), "{reply}");
    assert!(
        !reply["result"]["files"].as_array().unwrap().is_empty(),
        "{reply}"
    );

    let (reply, _) = client.call(
        "git.switch",
        json!({ "project": project, "branch": "experimento", "create": true }),
    );
    assert_eq!(reply["result"], "experimento", "{reply}");
    let (reply, _) = client.call("git.branches", json!({ "project": project }));
    assert_eq!(reply["result"]["current"], "experimento", "{reply}");
    // Sem remoto: publicar diz isso.
    let (reply, _) = client.call("git.push", json!({ "project": project }));
    assert_eq!(reply["error"]["data"]["code"], "git_no_remote", "{reply}");
    let (reply, _) = client.call(
        "git.read_file",
        json!({ "project": project, "revision": { "kind": "head" }, "path": "../fora" }),
    );
    assert_eq!(
        reply["error"]["data"]["code"], "invalid_argument",
        "{reply}"
    );
}
