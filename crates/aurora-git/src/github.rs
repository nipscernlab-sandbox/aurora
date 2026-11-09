//! A conta do GitHub: entrar pelo fluxo de dispositivo, saber quem entrou e
//! sair.
//!
//! O login segue a RFC 8628, a mesma da AURORA 7: [`login_start`] pede um
//! código, a pessoa o digita em github.com/login/device, e [`login_poll`]
//! pergunta ao GitHub, no intervalo que ele pede, se ela já autorizou. O
//! token vai para o gerenciador de credenciais do git (`git credential
//! approve`): no Windows, o Git Credential Manager, que guarda no cofre do
//! sistema; no macOS, o Keychain. A AURORA não grava o token em lugar
//! nenhum; [`account`] o pede de volta ao git (`git credential fill`) e
//! pergunta ao GitHub de quem ele é. Por isso uma conta que o git já
//! conhecia (de um `git push` antigo pelo Git Credential Manager) aparece
//! como conectada sem login nenhum.
//!
//! As chamadas ao GitHub vão pelo `curl` do sistema, com o token no stdin
//! (`curl -K -`), nunca na linha de comando, que outros processos leem.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::run::{self, Options};
use crate::{Git, parse};

/// O identificador do aplicativo OAuth "sapho", da organização nipscernlab,
/// o mesmo da AURORA 7. É público: o fluxo de dispositivo não usa segredo
/// do aplicativo.
pub const DEFAULT_CLIENT_ID: &str = "Ov23linD078LyGE5aDvg";

/// As permissões pedidas: `repo`, para enviar e trazer de repositórios
/// privados também.
pub const SCOPE: &str = "repo";

const HOST: &str = "github.com";
const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const USER_URL: &str = "https://api.github.com/user";

/// Quanto tempo o [`account`] reaproveita a resposta do GitHub.
const ACCOUNT_CACHE: Duration = Duration::from_secs(600);

/// O `curl` da máquina.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Curl {
    program: Utf8PathBuf,
}

impl Curl {
    /// O `curl` da primeira pasta de `dirs` que o tiver.
    ///
    /// # Erros
    ///
    /// [`Error::CurlMissing`].
    pub fn locate(dirs: &[Utf8PathBuf]) -> Result<Curl> {
        run::find_in(dirs, "curl")
            .map(|program| Curl { program })
            .ok_or(Error::CurlMissing)
    }

    /// Faz um pedido HTTPS e devolve o status e o corpo. `form` vai como
    /// POST `application/x-www-form-urlencoded`; `token`, no cabeçalho
    /// `Authorization`.
    fn request(&self, url: &str, form: Option<&str>, token: Option<&str>) -> Result<(u16, String)> {
        let mut config = String::new();
        let mut line = |key: &str, value: &str| {
            config.push_str(key);
            config.push_str(" = \"");
            config.push_str(&value.replace('\\', "\\\\").replace('"', "\\\""));
            config.push_str("\"\n");
        };
        line("url", url);
        line("header", "Accept: application/json");
        line("header", "User-Agent: AURORA");
        line("header", "X-GitHub-Api-Version: 2022-11-28");
        if let Some(token) = token {
            line("header", &format!("Authorization: Bearer {token}"));
        }
        if let Some(form) = form {
            line("data", form);
        }
        line("max-time", "30");
        config.push_str("silent\nshow-error\nwrite-out = \"\\n%{http_code}\"\n");

        let cwd = std::env::temp_dir();
        let cwd = Utf8PathBuf::from_path_buf(cwd).unwrap_or_else(|_| Utf8PathBuf::from("."));
        let output = run::exec(
            &self.program,
            &cwd,
            &["-K".to_owned(), "-".to_owned()],
            false,
            &Options {
                stdin: Some(config.into_bytes()),
                remote: true,
                ..Options::default()
            },
        )?;
        if !output.success() {
            return Err(Error::GitHub {
                status: None,
                message: if output.stderr.is_empty() {
                    "no answer".to_owned()
                } else {
                    output.stderr
                },
            });
        }
        let text = output.text();
        let (body, status) = text.rsplit_once('\n').unwrap_or(("", text.as_str()));
        let status = status.trim().parse().unwrap_or(0);
        Ok((status, body.to_owned()))
    }
}

/// O código do fluxo de dispositivo, de [`login_start`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct DeviceCode {
    /// O código que identifica este login em [`login_poll`]. Não é para
    /// mostrar.
    pub device_code: String,
    /// O código que a pessoa digita no GitHub (`WDJB-MJHT`).
    pub user_code: String,
    /// Onde ela digita (`https://github.com/login/device`).
    pub verification_uri: String,
    /// Em quantos segundos o código vence.
    pub expires_in: u64,
    /// Quantos segundos esperar entre duas chamadas de [`login_poll`].
    pub interval: u64,
}

/// A resposta de [`login_poll`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "state")]
#[non_exhaustive]
pub enum LoginPoll {
    /// Ainda não autorizou: pergunte de novo depois de `interval` segundos.
    Pending {
        /// O intervalo, em segundos (o GitHub o aumenta se a pergunta vier
        /// cedo demais).
        interval: u64,
    },
    /// Entrou.
    Done {
        /// A conta.
        account: Account,
    },
    /// O código venceu antes da autorização.
    Expired,
    /// A pessoa recusou no GitHub.
    Denied,
}

/// A conta do GitHub conectada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct Account {
    /// O nome de usuário (`octocat`).
    pub login: String,
    /// O identificador numérico.
    pub id: u64,
    /// O nome, se a pessoa preencheu.
    pub name: Option<String>,
    /// A foto.
    pub avatar_url: String,
    /// A página do perfil.
    pub html_url: String,
    /// O e-mail para os commits que o GitHub liga à conta sem expor o
    /// e-mail real (`123+octocat@users.noreply.github.com`).
    pub commit_email: String,
    /// O gerenciador de credenciais do git guardou o token: o login vale
    /// depois de fechar a AURORA, e o `git push` do terminal também o usa.
    /// Falso quando o git desta máquina não tem gerenciador de credenciais.
    pub saved: bool,
}

/// O que se lembra da última resposta do GitHub: o hash do token e a conta.
static CACHE: Mutex<Option<(u64, Account, Instant)>> = Mutex::new(None);

fn token_hash(token: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    hasher.finish()
}

/// Começa o login: pede ao GitHub o código que a pessoa vai digitar.
///
/// # Erros
///
/// [`Error::GitHub`] sem resposta ou com o fluxo de dispositivo desligado no
/// aplicativo.
pub fn login_start(curl: &Curl, client_id: &str) -> Result<DeviceCode> {
    let form = format!("client_id={}&scope={}", encode(client_id), encode(SCOPE));
    let (status, body) = curl.request(DEVICE_CODE_URL, Some(&form), None)?;
    let json = parse_json(status, &body)?;
    if let Some(error) = json.get("error").and_then(Value::as_str) {
        return Err(github_error(status, &json, error));
    }
    let text = |key: &str| json.get(key).and_then(Value::as_str).map(str::to_owned);
    let number = |key: &str, default: u64| json.get(key).and_then(Value::as_u64).unwrap_or(default);
    match (
        text("device_code"),
        text("user_code"),
        text("verification_uri"),
    ) {
        (Some(device_code), Some(user_code), Some(verification_uri)) => Ok(DeviceCode {
            device_code,
            user_code,
            verification_uri,
            expires_in: number("expires_in", 900),
            interval: number("interval", 5),
        }),
        _ => Err(Error::GitHub {
            status: Some(status),
            message: "unexpected answer to the device code request".to_owned(),
        }),
    }
}

/// Pergunta se a pessoa já autorizou o login de [`login_start`]. Com a
/// autorização, guarda o token no gerenciador de credenciais do git e
/// devolve a conta.
///
/// # Erros
///
/// [`Error::GitHub`] sem resposta ou com um erro que não é de espera.
pub fn login_poll(git: &Git, curl: &Curl, client_id: &str, device_code: &str) -> Result<LoginPoll> {
    let form = format!(
        "client_id={}&device_code={}&grant_type={}",
        encode(client_id),
        encode(device_code),
        encode("urn:ietf:params:oauth:grant-type:device_code")
    );
    let (status, body) = curl.request(TOKEN_URL, Some(&form), None)?;
    let json = parse_json(status, &body)?;
    if let Some(error) = json.get("error").and_then(Value::as_str) {
        let interval = json.get("interval").and_then(Value::as_u64).unwrap_or(5);
        return match error {
            "authorization_pending" => Ok(LoginPoll::Pending { interval }),
            "slow_down" => Ok(LoginPoll::Pending { interval }),
            "expired_token" | "token_expired" => Ok(LoginPoll::Expired),
            "access_denied" => Ok(LoginPoll::Denied),
            other => Err(github_error(status, &json, other)),
        };
    }
    let Some(token) = json.get("access_token").and_then(Value::as_str) else {
        return Err(Error::GitHub {
            status: Some(status),
            message: "the answer has no token".to_owned(),
        });
    };
    let mut account = user(curl, token)?.ok_or_else(|| Error::GitHub {
        status: Some(401),
        message: "the new token was refused".to_owned(),
    })?;
    credential(git, "approve", &account.login, Some(token))?;
    account.saved = stored_token(git)?.is_some_and(|(_, stored)| stored == token);
    remember(token, &account);
    Ok(LoginPoll::Done { account })
}

/// A conta do GitHub que o git desta máquina conhece, ou `None`.
///
/// Pergunta ao GitHub no máximo uma vez a cada 10 minutos para o mesmo
/// token. Um token que o GitHub recusa conta como nenhuma conta (e continua
/// no gerenciador do git: a AURORA não apaga credencial sem pedido).
///
/// # Erros
///
/// [`Error::GitHub`] sem resposta do GitHub.
pub fn account(git: &Git, curl: &Curl) -> Result<Option<Account>> {
    let Some((_, token)) = stored_token(git)? else {
        return Ok(None);
    };
    let hash = token_hash(&token);
    if let Ok(cache) = CACHE.lock()
        && let Some((cached, account, at)) = cache.as_ref()
        && *cached == hash
        && at.elapsed() < ACCOUNT_CACHE
    {
        return Ok(Some(account.clone()));
    }
    let Some(mut account) = user(curl, &token)? else {
        return Ok(None);
    };
    account.saved = true;
    remember(&token, &account);
    Ok(Some(account))
}

/// Sai: tira o token do GitHub do gerenciador de credenciais do git.
/// Devolve se havia um.
///
/// O `git push` do terminal, que usava o mesmo token, também deixa de ter
/// credencial para o GitHub.
///
/// # Erros
///
/// Uma falha do git.
pub fn logout(git: &Git) -> Result<bool> {
    if let Ok(mut cache) = CACHE.lock() {
        *cache = None;
    }
    let Some((login, token)) = stored_token(git)? else {
        return Ok(false);
    };
    credential(git, "reject", &login, Some(&token))?;
    Ok(true)
}

fn remember(token: &str, account: &Account) {
    if let Ok(mut cache) = CACHE.lock() {
        *cache = Some((token_hash(token), account.clone(), Instant::now()));
    }
}

/// A pasta onde rodar o `git credential`: fora de qualquer repositório,
/// para valer a configuração do usuário, não a de um projeto.
fn neutral_dir() -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap_or_else(|_| Utf8PathBuf::from("."))
}

/// O usuário e o token do GitHub que o gerenciador de credenciais do git
/// devolve, sem abrir janela nem pedir nada a ninguém.
fn stored_token(git: &Git) -> Result<Option<(String, String)>> {
    let input = format!("protocol=https\nhost={HOST}\n\n");
    let output = git.run(
        &neutral_dir(),
        &["credential", "fill"],
        &Options {
            stdin: Some(input.into_bytes()),
            ..Options::default()
        },
    )?;
    if !output.success() {
        return Ok(None);
    }
    let fields = parse::credential(&output.text());
    let get = |key: &str| {
        fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .filter(|v| !v.is_empty())
    };
    Ok(get("password").map(|token| (get("username").unwrap_or_default(), token)))
}

fn credential(git: &Git, action: &str, login: &str, token: Option<&str>) -> Result<()> {
    let mut input = format!("protocol=https\nhost={HOST}\nusername={login}\n");
    if let Some(token) = token {
        input.push_str(&format!("password={token}\n"));
    }
    input.push('\n');
    let dir = neutral_dir();
    git.run_ok(
        Utf8Path::new(&dir),
        &["credential", action],
        &Options {
            stdin: Some(input.into_bytes()),
            ..Options::default()
        },
    )?;
    Ok(())
}

/// O usuário dono do token; `None` se o GitHub o recusa.
fn user(curl: &Curl, token: &str) -> Result<Option<Account>> {
    let (status, body) = curl.request(USER_URL, None, Some(token))?;
    if status == 401 || status == 403 {
        return Ok(None);
    }
    let json = parse_json(status, &body)?;
    if status != 200 {
        let message = json
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unexpected answer")
            .to_owned();
        return Err(Error::GitHub {
            status: Some(status),
            message,
        });
    }
    let text = |key: &str| json.get(key).and_then(Value::as_str).map(str::to_owned);
    let (Some(login), Some(id)) = (text("login"), json.get("id").and_then(Value::as_u64)) else {
        return Err(Error::GitHub {
            status: Some(status),
            message: "the user answer has no login".to_owned(),
        });
    };
    Ok(Some(Account {
        commit_email: format!("{id}+{login}@users.noreply.github.com"),
        avatar_url: text("avatar_url").unwrap_or_default(),
        html_url: text("html_url").unwrap_or_else(|| format!("https://github.com/{login}")),
        name: text("name").filter(|n| !n.trim().is_empty()),
        login,
        id,
        saved: false,
    }))
}

fn parse_json(status: u16, body: &str) -> Result<Value> {
    serde_json::from_str(body).map_err(|_| Error::GitHub {
        status: Some(status),
        message: format!("unexpected answer (HTTP {status})"),
    })
}

fn github_error(status: u16, json: &Value, error: &str) -> Error {
    let detail = json
        .get("error_description")
        .and_then(Value::as_str)
        .unwrap_or(error);
    Error::GitHub {
        status: Some(status),
        message: detail.to_owned(),
    }
}

/// Codifica um valor de formulário (`application/x-www-form-urlencoded`).
fn encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_form_values() {
        assert_eq!(encode("repo read:org"), "repo%20read%3Aorg");
        assert_eq!(
            encode("urn:ietf:params:oauth:grant-type:device_code"),
            "urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code"
        );
    }
}
