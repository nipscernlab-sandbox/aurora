//! Os erros do controle de versão.

use camino::Utf8PathBuf;
use serde_json::{Value, json};

/// O resultado das funções deste crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Por que uma operação de git ou do GitHub não deu certo.
///
/// Cada variante tem um código estável ([`Error::code`]), o que uma
/// interface usa para traduzir a mensagem, e os campos em JSON
/// ([`Error::data`]).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// O git não está no `PATH`.
    #[error("Git was not found on this computer")]
    GitMissing,

    /// O git achado é antigo demais para a AURORA.
    #[error("Git {version} is too old; AURORA needs Git {minimum} or newer")]
    GitTooOld {
        /// A versão achada.
        version: String,
        /// A menor versão aceita.
        minimum: String,
    },

    /// A pasta não está em nenhum repositório git.
    #[error("{path} is not in a Git repository")]
    NotRepository {
        /// A pasta.
        path: Utf8PathBuf,
    },

    /// O repositório é de outro usuário do sistema, e o git se recusa a
    /// usá-lo (o `safe.directory` do git 2.35.2 em diante).
    #[error("Git refuses to use {path}: the folder belongs to another user")]
    DubiousOwnership {
        /// A pasta de topo do repositório.
        path: Utf8PathBuf,
    },

    /// Falta o nome ou o e-mail de quem faz o commit.
    #[error("Git does not know your name and e-mail for the commit")]
    IdentityMissing,

    /// Nada selecionado ou nada mudado para o commit.
    #[error("There is nothing to commit")]
    NothingToCommit,

    /// A mensagem do commit está vazia.
    #[error("The commit message is empty")]
    EmptyMessage,

    /// Um nome de ramo que o git não aceita.
    #[error("'{name}' is not a valid branch name")]
    InvalidBranchName {
        /// O nome pedido.
        name: String,
    },

    /// Um ramo que não existe.
    #[error("There is no branch '{name}'")]
    BranchNotFound {
        /// O nome pedido.
        name: String,
    },

    /// Um ramo com esse nome já existe.
    #[error("A branch named '{name}' already exists")]
    BranchExists {
        /// O nome pedido.
        name: String,
    },

    /// Trocar de ramo apagaria mudanças que ainda não estão em commit.
    #[error("Switching to {branch} would overwrite changes in: {}", files.join(", "))]
    SwitchBlocked {
        /// O ramo pedido.
        branch: String,
        /// Os arquivos em risco, relativos à pasta de topo.
        files: Vec<String>,
    },

    /// O repositório não tem remoto para onde enviar ou de onde trazer.
    #[error("The repository has no remote")]
    NoRemote,

    /// O ramo ainda não foi publicado: não tem ramo remoto para trazer.
    #[error("The branch {branch} is not published yet")]
    NoUpstream {
        /// O ramo atual.
        branch: String,
    },

    /// O commit atual não está em nenhum ramo (HEAD destacado).
    #[error("HEAD is not on a branch")]
    DetachedHead,

    /// O remoto recusou a autenticação, ou não havia credencial.
    #[error("{remote} refused the authentication")]
    Authentication {
        /// O remoto.
        remote: String,
        /// O que o git disse.
        stderr: String,
    },

    /// O remoto não existe nesse endereço, não responde, ou a conta não tem
    /// acesso a ele (o GitHub responde igual para os dois): o endereço pode
    /// estar errado.
    #[error("{remote} ({url}) was not found or did not answer")]
    RemoteUnreachable {
        /// O remoto.
        remote: String,
        /// O endereço, sem usuário nem senha.
        url: String,
        /// O que o git disse.
        stderr: String,
    },

    /// O remoto recusou o envio porque tem commits que o ramo local não tem.
    #[error("{remote} has commits that {branch} does not have; bring them first")]
    PushRejected {
        /// O remoto.
        remote: String,
        /// O ramo.
        branch: String,
    },

    /// O ramo local e o remoto têm commits diferentes: trazer só dá com um
    /// merge ou um rebase.
    #[error("{branch} and its remote branch have diverged")]
    Diverged {
        /// O ramo.
        branch: String,
    },

    /// Um valor recusado na entrada: caminho fora do repositório, commit
    /// que não é um identificador, endereço vazio.
    #[error("{0}")]
    InvalidArgument(String),

    /// O git rodou e falhou de um jeito sem variante própria.
    #[error("git {command} failed: {stderr}")]
    Failed {
        /// O subcomando (`commit`, `push`).
        command: String,
        /// O que o git escreveu no stderr.
        stderr: String,
    },

    /// A operação foi cancelada.
    #[error("Cancelled")]
    Cancelled,

    /// O git ficou sem responder além do prazo (um remoto que não responde).
    #[error("git {command} stopped responding")]
    TimedOut {
        /// O subcomando.
        command: String,
    },

    /// O `curl` não está no `PATH`.
    #[error("curl was not found on this computer")]
    CurlMissing,

    /// O GitHub não respondeu, ou respondeu com um erro.
    #[error("GitHub: {message}")]
    GitHub {
        /// O status HTTP, quando houve resposta.
        status: Option<u16>,
        /// O que deu errado.
        message: String,
    },

    /// Um programa não chegou a rodar.
    #[error("Could not run {program}: {source}")]
    Spawn {
        /// O programa.
        program: Utf8PathBuf,
        /// O erro do sistema.
        #[source]
        source: std::io::Error,
    },

    /// Falha de leitura ou gravação de arquivo.
    #[error("{context}: {path}: {source}")]
    Io {
        /// O que se fazia.
        context: &'static str,
        /// O caminho.
        path: Utf8PathBuf,
        /// O erro do sistema.
        #[source]
        source: std::io::Error,
    },
}

impl Error {
    /// O identificador estável da variante, em `snake_case`.
    pub fn code(&self) -> &'static str {
        match self {
            Error::GitMissing => "git_missing",
            Error::GitTooOld { .. } => "git_too_old",
            Error::NotRepository { .. } => "git_not_repository",
            Error::DubiousOwnership { .. } => "git_dubious_ownership",
            Error::IdentityMissing => "git_identity_missing",
            Error::NothingToCommit => "git_nothing_to_commit",
            Error::EmptyMessage => "git_empty_message",
            Error::InvalidBranchName { .. } => "git_invalid_branch_name",
            Error::BranchNotFound { .. } => "git_branch_not_found",
            Error::BranchExists { .. } => "git_branch_exists",
            Error::SwitchBlocked { .. } => "git_switch_blocked",
            Error::NoRemote => "git_no_remote",
            Error::NoUpstream { .. } => "git_no_upstream",
            Error::DetachedHead => "git_detached_head",
            Error::Authentication { .. } => "git_authentication",
            Error::RemoteUnreachable { .. } => "git_remote_unreachable",
            Error::PushRejected { .. } => "git_push_rejected",
            Error::Diverged { .. } => "git_diverged",
            Error::InvalidArgument(_) => "invalid_argument",
            Error::Failed { .. } => "git_failed",
            Error::Cancelled => "cancelled",
            Error::TimedOut { .. } => "git_timed_out",
            Error::CurlMissing => "curl_missing",
            Error::GitHub { .. } => "github_error",
            Error::Spawn { .. } => "spawn_failed",
            Error::Io { .. } => "io",
        }
    }

    /// Os campos da variante em JSON, para a interface; `None` quando não há.
    pub fn data(&self) -> Option<Value> {
        let data = match self {
            Error::GitTooOld { version, minimum } => {
                json!({ "version": version, "minimum": minimum })
            }
            Error::NotRepository { path } | Error::DubiousOwnership { path } => {
                json!({ "path": path })
            }
            Error::InvalidBranchName { name }
            | Error::BranchNotFound { name }
            | Error::BranchExists { name } => json!({ "name": name }),
            Error::SwitchBlocked { branch, files } => json!({ "branch": branch, "files": files }),
            Error::NoUpstream { branch } | Error::Diverged { branch } => {
                json!({ "branch": branch })
            }
            Error::Authentication { remote, stderr } => {
                json!({ "remote": remote, "stderr": stderr })
            }
            Error::RemoteUnreachable {
                remote,
                url,
                stderr,
            } => json!({ "remote": remote, "url": url, "stderr": stderr }),
            Error::PushRejected { remote, branch } => json!({ "remote": remote, "branch": branch }),
            Error::Failed { command, stderr } => json!({ "command": command, "stderr": stderr }),
            Error::TimedOut { command } => json!({ "command": command }),
            Error::GitHub { status, .. } => json!({ "status": status }),
            Error::Spawn { program, .. } => json!({ "program": program }),
            Error::Io { context, path, .. } => json!({ "context": context, "path": path }),
            Error::GitMissing
            | Error::IdentityMissing
            | Error::NothingToCommit
            | Error::EmptyMessage
            | Error::NoRemote
            | Error::DetachedHead
            | Error::InvalidArgument(_)
            | Error::Cancelled
            | Error::CurlMissing => return None,
        };
        Some(data)
    }

    pub(crate) fn io(
        context: &'static str,
        path: impl Into<Utf8PathBuf>,
    ) -> impl FnOnce(std::io::Error) -> Error {
        let path = path.into();
        move |source| Error::Io {
            context,
            path,
            source,
        }
    }
}
