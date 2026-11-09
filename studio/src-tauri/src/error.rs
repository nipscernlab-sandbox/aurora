//! O erro que volta para a interface.
//!
//! Todo comando devolve `Result<T, IpcError>`. A interface recebe um objeto
//! `{ code, message, data? }`: o `code` é estável e é por ele que a interface
//! decide a dica e a tradução; a `message` é o texto do Core (em inglês) ou do
//! próprio Studio, para quando não houver tradução; `data`, quando há, são os
//! campos do erro que o servidor da AURORA mandou (os arquivos que impedem a
//! troca de ramo, o stderr do git).
//!
//! Os códigos do Core (`AuroraError::code()`, ver `docs/API.md` da raiz, seção 8)
//! passam como estão. Os do Studio estão em [`codes`].

use serde::Serialize;
use serde_json::Value;

/// Os códigos de erro do próprio Studio. Os do Core vêm de
/// `AuroraError::code()` e não se repetem aqui.
pub mod codes {
    /// Nenhum projeto aberto.
    pub const NO_PROJECT: &str = "no_project";
    /// O `.spf` do projeto aberto não existe mais (apagado depois de
    /// aberto): sem ele não há operação de projeto, mas a pasta e os
    /// arquivos continuam acessíveis.
    pub const SPF_MISSING: &str = "spf_missing";
    /// Já há uma operação rodando.
    pub const BUSY: &str = "busy";
    /// Caminho fora da pasta do projeto numa operação que só vale dentro dela.
    pub const OUTSIDE_PROJECT: &str = "outside_project";
    /// O arquivo mudou no disco desde que foi aberto.
    pub const CONFLICT: &str = "conflict";
    /// O arquivo ou a pasta já existe.
    pub const EXISTS: &str = "exists";
    /// Argumento inválido vindo da interface.
    pub const INVALID_ARGUMENT: &str = "invalid_argument";
    /// A CLI `aurora` não foi encontrada ou falhou.
    pub const CLI: &str = "cli";
    /// O terminal de shell não abriu ou não existe mais.
    pub const TERMINAL: &str = "terminal";
    /// O caminho tem um caractere de controle, que o shell leria como tecla:
    /// o `cd` automático do terminal não o digita.
    pub const UNSAFE_PATH: &str = "unsafe_path";
    /// Um programa está em primeiro plano no terminal (no Linux e no macOS):
    /// o `cd` automático iria para ele e não é digitado.
    pub const TERMINAL_BUSY: &str = "terminal_busy";
    /// Erro de leitura ou escrita do próprio Studio.
    pub const IO: &str = "io";
    /// O bundle não traz o cliente web do Surfer (a onda abre em janela).
    pub const SURFER_WEB_MISSING: &str = "surfer_web_missing";
    /// A onda é grande demais para uma aba (abre em janela).
    pub const WAVE_TOO_LARGE: &str = "wave_too_large";
    /// Falha interna (thread que morreu, JSON inválido).
    pub const INTERNAL: &str = "internal";
    /// O assistente: o Claude Code não foi achado, ou a sessão terminou.
    pub const ASSISTANT: &str = "assistant";
    /// O programa `aurora` que serve o Studio não foi achado ou não abriu.
    pub const AURORA_MISSING: &str = "aurora_missing";
    /// O `aurora` achado fala outra versão do protocolo.
    pub const AURORA_INCOMPATIBLE: &str = "aurora_incompatible";
    /// O servidor da AURORA saiu no meio de um pedido; o próximo o abre de novo.
    pub const SERVER_GONE: &str = "server_gone";
    /// O pedido foi cancelado antes de começar (`-32001` do protocolo).
    pub const CANCELLED: &str = "cancelled";
}

/// O erro de um comando, como a interface o recebe.
#[derive(Debug, Clone, Serialize)]
pub struct IpcError {
    /// Identificador estável: `AuroraError::code()` ou um de [`codes`].
    pub code: String,
    /// O texto do erro, para mostrar quando a interface não tem tradução.
    pub message: String,
    /// Os campos estruturados do erro, como o servidor da AURORA os mandou
    /// (`ErrorInfo::data`); omitido quando não há.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// O resultado de todo comando.
pub type IpcResult<T> = Result<T, IpcError>;

impl IpcError {
    /// Um erro com código e mensagem.
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        IpcError {
            code: code.to_owned(),
            message: message.into(),
            data: None,
        }
    }

    /// O mesmo erro com os campos estruturados do servidor.
    pub fn with_data(mut self, data: Option<Value>) -> Self {
        self.data = data.filter(|d| !d.is_null());
        self
    }

    /// Erro de E/S com o caminho que falhou.
    pub fn io(context: impl std::fmt::Display, error: std::io::Error) -> Self {
        IpcError::new(codes::IO, format!("{context}: {error}"))
    }
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

impl std::error::Error for IpcError {}

impl From<std::io::Error> for IpcError {
    fn from(error: std::io::Error) -> Self {
        IpcError::new(codes::IO, error.to_string())
    }
}

impl From<serde_json::Error> for IpcError {
    fn from(error: serde_json::Error) -> Self {
        IpcError::new(codes::INTERNAL, format!("JSON: {error}"))
    }
}

impl From<tauri::Error> for IpcError {
    fn from(error: tauri::Error) -> Self {
        IpcError::new(codes::INTERNAL, error.to_string())
    }
}
