//! Rodar o git (e o curl) do sistema: um processo por operação, sem janela
//! de console, sem nunca esperar alguém digitar, com cancelamento e, nas
//! operações de rede, prazo de ociosidade.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use aurora_core::{CancelToken, ProcessJob, hide_console};
use camino::{Utf8Path, Utf8PathBuf};

use crate::error::{Error, Result};

/// Quanto uma operação de rede pode ficar sem escrever nada antes de ser
/// encerrada: um remoto que aceitou a conexão e calou, um pedido de senha
/// que escapou da configuração abaixo.
pub(crate) const REMOTE_IDLE: Duration = Duration::from_secs(120);

/// O que o git recebe antes do subcomando, em toda chamada.
///
/// - `--literal-pathspecs`: um nome de arquivo com `*` ou `?` é o arquivo, não
///   um padrão.
/// - `core.quotepath=false`: caminho com acento sai como está, sem escape.
/// - `core.fsmonitor=false`: um `.git/config` de um repositório recebido de
///   alguém pode apontar o fsmonitor para um comando qualquer, que o status
///   rodaria.
/// - `credential.interactive=false`: o Git Credential Manager não abre janela;
///   quem entra no GitHub é a AURORA, pela vista Perfil.
const GLOBAL_ARGS: &[&str] = &[
    "--no-pager",
    "--literal-pathspecs",
    "-c",
    "core.quotepath=false",
    "-c",
    "color.ui=false",
    "-c",
    "core.fsmonitor=false",
    "-c",
    "log.showSignature=false",
    "-c",
    "credential.interactive=false",
];

/// Variáveis definidas para o git. O resto do ambiente é herdado, porque o
/// git precisa dele: a configuração global (`HOME`, `USERPROFILE`), o agente
/// do SSH, o proxy, o `PATH` dos hooks e do gerenciador de credenciais.
///
/// `LC_ALL=C` deixa as mensagens em inglês, as que a AURORA lê para separar
/// "senha recusada" de "ramo divergente". `GIT_TERMINAL_PROMPT=0` e
/// `GCM_INTERACTIVE=never` fazem o git falhar em vez de esperar uma senha;
/// `GIT_EDITOR=:` faz o mesmo com um editor de mensagem.
const ENV: &[(&str, &str)] = &[
    ("LC_ALL", "C"),
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GCM_INTERACTIVE", "never"),
    ("GIT_EDITOR", ":"),
    ("GIT_SEQUENCE_EDITOR", ":"),
    ("GIT_MERGE_AUTOEDIT", "no"),
];

/// Variáveis tiradas do ambiente herdado. `GIT_DIR` e companhia vêm de quem
/// abriu a AURORA de dentro de um hook e apontariam o git para outro
/// repositório; um `GIT_ASKPASS` herdado (o terminal do VS Code define um)
/// abriria uma janela de senha de outro programa.
const REMOVED_ENV: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_PREFIX",
    "GIT_ASKPASS",
    "SSH_ASKPASS",
    "LANGUAGE",
];

/// O teto do stdout guardado. Passado ele, o resto é lido e descartado, e
/// [`Output::overflow`] fica ligado.
const STDOUT_KEPT: usize = 64 * 1024 * 1024;
/// O teto do stderr guardado.
const STDERR_KEPT: usize = 256 * 1024;

/// Como rodar.
#[derive(Default)]
pub(crate) struct Options<'a> {
    /// O que vai pelo stdin.
    pub stdin: Option<Vec<u8>>,
    /// Só lê o repositório: sem as travas opcionais do índice, para não
    /// disputar com o git que a pessoa roda no terminal.
    pub read: bool,
    /// Fala com um remoto: prazo de ociosidade [`REMOTE_IDLE`].
    pub remote: bool,
    /// Encerra o processo quando o cancelamento for pedido.
    pub cancel: Option<&'a CancelToken>,
}

/// O que o processo produziu.
pub(crate) struct Output {
    /// O código de saída; `None` sem código (morto por sinal).
    pub code: Option<i32>,
    /// O stdout, em bytes.
    pub stdout: Vec<u8>,
    /// O stderr, como texto.
    pub stderr: String,
    /// O stdout passou de [`STDOUT_KEPT`].
    #[allow(dead_code)]
    pub overflow: bool,
}

impl Output {
    pub(crate) fn success(&self) -> bool {
        self.code == Some(0)
    }

    /// O stdout como texto (o git escreve UTF-8).
    pub(crate) fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
}

/// Roda `program` com `args` em `cwd`. `git` liga os argumentos e o
/// ambiente do git; sem ele (o curl), o ambiente vai herdado.
pub(crate) fn exec(
    program: &Utf8Path,
    cwd: &Utf8Path,
    args: &[String],
    git: bool,
    options: &Options<'_>,
) -> Result<Output> {
    let mut command = Command::new(program.as_std_path());
    if git {
        command.args(GLOBAL_ARGS);
        for (key, value) in ENV {
            command.env(key, value);
        }
        for key in REMOVED_ENV {
            command.env_remove(key);
        }
        if options.read {
            command.env("GIT_OPTIONAL_LOCKS", "0");
        }
    }
    command
        .args(args)
        .current_dir(cwd.as_std_path())
        .stdin(if options.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_console(&mut command);

    tracing::debug!(program = %program, ?args, cwd = %cwd, "Running");
    let mut job = ProcessJob::new(false);
    let mut child = command.spawn().map_err(|source| Error::Spawn {
        program: program.to_owned(),
        source,
    })?;
    job.adopt(&child);

    if let (Some(input), Some(mut stdin)) = (options.stdin.clone(), child.stdin.take()) {
        thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
    }

    let started = Instant::now();
    // Milissegundos desde `started` da última saída, de qualquer pipe.
    let activity = Arc::new(AtomicU64::new(0));
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        unreachable!("stdout and stderr are pipes");
    };
    let out = read_all(stdout, STDOUT_KEPT, started, activity.clone());
    let err = read_all(stderr, STDERR_KEPT, started, activity.clone());

    let mut stopped = None;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|source| Error::Spawn {
            program: program.to_owned(),
            source,
        })? {
            break Some(status);
        }
        if stopped.is_none() {
            if options.cancel.is_some_and(|c| c.is_cancelled()) {
                stopped = Some(Error::Cancelled);
            } else if options.remote {
                let last = Duration::from_millis(activity.load(Ordering::Relaxed));
                if started.elapsed().saturating_sub(last) > REMOTE_IDLE {
                    stopped = Some(Error::TimedOut {
                        command: subcommand(args),
                    });
                }
            }
            if stopped.is_some() {
                job.kill();
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
        // Começa rápido (um status leva dezenas de milissegundos) e espaça
        // depois, para um push longo não girar à toa.
        let wait = if started.elapsed() < Duration::from_millis(200) {
            Duration::from_millis(2)
        } else {
            Duration::from_millis(25)
        };
        thread::sleep(wait);
    };
    let (stdout, overflow) = out.join().unwrap_or_default();
    let (stderr, _) = err.join().unwrap_or_default();
    if let Some(error) = stopped {
        return Err(error);
    }
    Ok(Output {
        code: status.and_then(|s| s.code()),
        stdout,
        stderr: String::from_utf8_lossy(&stderr).trim_end().to_owned(),
        overflow,
    })
}

/// Lê o pipe até o fim, guardando até `kept` bytes.
fn read_all(
    mut pipe: impl Read + Send + 'static,
    kept: usize,
    started: Instant,
    activity: Arc<AtomicU64>,
) -> thread::JoinHandle<(Vec<u8>, bool)> {
    thread::spawn(move || {
        let mut data = Vec::new();
        let mut overflow = false;
        let mut buffer = [0u8; 16 * 1024];
        loop {
            match pipe.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    activity.store(started.elapsed().as_millis() as u64, Ordering::Relaxed);
                    let room = kept.saturating_sub(data.len());
                    if n > room {
                        overflow = true;
                    }
                    data.extend_from_slice(&buffer[..n.min(room)]);
                }
            }
        }
        (data, overflow)
    })
}

/// O subcomando de uma linha de argumentos (`push`), para as mensagens.
pub(crate) fn subcommand(args: &[String]) -> String {
    args.iter()
        .find(|a| !a.starts_with('-'))
        .cloned()
        .unwrap_or_default()
}

/// O executável `name` na primeira pasta de `dirs` que o tiver.
pub(crate) fn find_in(dirs: &[Utf8PathBuf], name: &str) -> Option<Utf8PathBuf> {
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    dirs.iter()
        .map(|dir| dir.join(&file))
        .find(|path| path.is_file())
}
