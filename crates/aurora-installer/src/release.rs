//! As releases da AURORA na rede: a última versão, e baixar um arquivo de uma
//! release conferindo o SHA-256 pelo `SHA256SUMS` dela. Também baixa o que
//! mais o `aurora install` e o `aurora update` precisam ler.
//!
//! Baixa com o `curl` do sistema, o mesmo que o `install.sh` usa (no Windows
//! 10 e 11 ele vem no `System32`): a AURORA não traz cliente HTTP próprio.
//!
//! A biblioteca não lê o ambiente por conta própria: a [`Network`] é montada
//! por quem chama (a CLI das variáveis, com [`Network::from_env`]; o Studio
//! das preferências dele). Toda espera pela rede confere o `CancelToken`:
//! cancelar mata o `curl`.

use std::io::Read;
use std::process::Stdio;
use std::time::Duration;

use anyhow::Context;
use aurora_core::CancelToken;
use camino::{Utf8Path, Utf8PathBuf};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

/// De onde vêm as releases e como baixar.
#[derive(Debug, Clone)]
pub struct Network {
    /// O repositório das releases (`dono/nome`); [`Network::DEFAULT_REPO`]
    /// por padrão.
    pub repo: String,
    /// Um espelho (`AURORA_RELEASE_URL`), já validado por
    /// [`Network::validate_mirror`]: só `https://` ou `file://`, sem a barra
    /// final. Tem uma pasta `v<versão>/` por release e, se quiser, um
    /// arquivo `latest` com a última versão.
    pub mirror: Option<String>,
    /// O `curl`; `None` é o `curl` do `PATH`. A CLI passa o do `System32` no
    /// Windows.
    pub curl: Option<Utf8PathBuf>,
    /// A base das pastas de trabalho (`aurora-install.*`): fora do `/tmp`,
    /// que há sistemas que montam sem permissão de execução, e um
    /// instalador baixado roda dela.
    pub work_base: Utf8PathBuf,
    /// Um prazo para cada transferência (`--max-time`); `None` deixa um
    /// download grande levar o tempo que leva. Conectar tem sempre 20 s.
    pub timeout: Option<Duration>,
}

impl Network {
    /// O repositório padrão das releases. Até a 1.0.0 era o ART3121/lace,
    /// que o GitHub redireciona para este.
    pub const DEFAULT_REPO: &str = "nipscernlab-sandbox/aurora";

    /// A rede como a CLI a monta: `AURORA_REPO`, `AURORA_RELEASE_URL`, o `curl`
    /// do `System32` no Windows, e a base das pastas de trabalho (o temp do
    /// sistema no Windows; `/var/tmp` como root; `XDG_CACHE_HOME` ou
    /// `~/.cache`).
    pub fn from_env() -> Result<Network> {
        let repo = std::env::var("AURORA_REPO").unwrap_or_else(|_| Self::DEFAULT_REPO.to_owned());
        let mirror = Self::validate_mirror(std::env::var("AURORA_RELEASE_URL").ok().as_deref())?;
        let curl = if cfg!(windows) {
            std::env::var("SystemRoot")
                .ok()
                .map(|root| Utf8PathBuf::from(format!("{root}\\System32\\curl.exe")))
        } else {
            None
        };
        let base = if cfg!(windows) {
            std::env::temp_dir()
        } else if std::env::var("USER").is_ok_and(|u| u == "root") {
            "/var/tmp".into()
        } else {
            std::env::var_os("XDG_CACHE_HOME")
                .filter(|v| !v.is_empty())
                .map(std::path::PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".cache"))
                })
                .unwrap_or_else(std::env::temp_dir)
        };
        let work_base = Utf8PathBuf::from_path_buf(base)
            .map_err(|p| anyhow::anyhow!("Path is not UTF-8: {}", p.display()))?;
        Ok(Network {
            repo,
            mirror,
            curl,
            work_base,
            timeout: None,
        })
    }

    /// A base de um espelho: só `https://`, ou `file://` para um espelho no
    /// disco. O `SHA256SUMS` que confere os downloads vem do mesmo lugar, e
    /// por `http://` quem estivesse na rede trocaria os dois juntos, e com
    /// eles o `aurora` do próximo `aurora update`. Vazio é nenhum.
    pub fn validate_mirror(base: Option<&str>) -> Result<Option<String>> {
        let Some(base) = base.filter(|b| !b.is_empty()) else {
            return Ok(None);
        };
        let scheme = base.split_once("://").map(|(s, _)| s.to_ascii_lowercase());
        match scheme.as_deref() {
            Some("https" | "file") => Ok(Some(base.trim_end_matches('/').to_owned())),
            _ => Err(Error::InvalidMirror {
                base: base.to_owned(),
            }),
        }
    }

    /// A URL de um arquivo da release `v<version>`: do espelho, se há, senão
    /// do GitHub.
    pub fn asset_url(&self, version: &str, file: &str) -> Result<String> {
        if !is_version(version) {
            return Err(
                anyhow::anyhow!("Not an AURORA version: {}", version.escape_debug()).into(),
            );
        }
        if !is_asset_name(file) {
            return Err(
                anyhow::anyhow!("Not a file name of a release: {}", file.escape_debug()).into(),
            );
        }
        Ok(match &self.mirror {
            Some(base) => format!("{base}/v{version}/{file}"),
            None => format!(
                "https://github.com/{}/releases/download/v{version}/{file}",
                self.repo
            ),
        })
    }

    /// Uma pasta de trabalho nova em [`Network::work_base`], depois de
    /// apagar as sobras de mais de um dia ([`WORK_DIR_LEFTOVER`]).
    pub fn work_dir(&self) -> Result<tempfile::TempDir> {
        std::fs::create_dir_all(&self.work_base)
            .with_context(|| format!("Creating {}", self.work_base))?;
        clean_old_work_dirs(self.work_base.as_std_path(), WORK_DIR_LEFTOVER);
        Ok(tempfile::Builder::new()
            .prefix(WORK_PREFIX)
            .tempdir_in(&self.work_base)
            .with_context(|| format!("Creating a temporary folder in {}", self.work_base))?)
    }

    /// O `curl`, com prazo para conectar: uma rede que pendura em vez de
    /// recusar não trava o comando. Só por `https` (e `file`, o espelho no
    /// disco), também nos redirecionamentos: um redirecionamento para `http`
    /// não troca o que a AURORA baixa no caminho.
    fn curl(&self) -> std::process::Command {
        let program = self
            .curl
            .as_ref()
            .map_or_else(|| "curl".to_owned(), ToString::to_string);
        let mut command = std::process::Command::new(program);
        command.args([
            "--connect-timeout",
            "20",
            "--proto",
            "=https,file",
            "--proto-redir",
            "=https",
        ]);
        if let Some(timeout) = self.timeout {
            command
                .arg("--max-time")
                .arg(timeout.as_secs().max(1).to_string());
        }
        aurora_core::hide_console(&mut command);
        command
    }
}

/// Uma versão como as da AURORA: começa por dígito e tem só letras, dígitos,
/// `.`, `-` e `+` (`0.9.0`, `0.9.0-rc.1`). É o que vai no caminho das URLs e
/// no nome dos arquivos baixados, e o que o terminal mostra: uma resposta do
/// espelho com `/`, `..` ou uma sequência de escape não passa.
pub fn is_version(text: &str) -> bool {
    text.len() <= 64
        && text.starts_with(|c: char| c.is_ascii_digit())
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
}

/// Um nome de arquivo de uma release (o índice de um espelho também os
/// lista): sem pasta e sem começar por `.`, com os caracteres de
/// [`is_version`] e `_`.
pub fn is_asset_name(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 255
        && !text.starts_with('.')
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'))
}

/// O hash de `file` num `SHA256SUMS` (`<hash>  <arquivo>`, com `*` antes do
/// nome no modo binário).
pub fn expected_sha256(sums: &str, file: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, name) = line.split_once(char::is_whitespace)?;
        let name = name.trim().trim_start_matches('*');
        (name == file).then(|| hash.to_ascii_lowercase())
    })
}

/// O SHA-256 de um arquivo, em hexadecimal.
pub fn sha256_of(path: &Utf8Path) -> Result<String> {
    let mut file = std::fs::File::open(path).with_context(|| format!("Opening {path}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 16];
    loop {
        let n = file
            .read(&mut buffer)
            .with_context(|| format!("Reading {path}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// O começo do nome de cada pasta de [`Network::work_dir`].
pub const WORK_PREFIX: &str = "aurora-install.";

/// Uma pasta de trabalho mais velha que isso é sobra: o instalador completo
/// que o `aurora update` deixa para o assistente do Windows rodar depois que
/// ele sai, ou a de um `aurora` encerrado no meio de um download. Uma de outra
/// operação que ainda roda é nova.
pub const WORK_DIR_LEFTOVER: Duration = Duration::from_secs(24 * 60 * 60);

/// Apaga as pastas de trabalho de `base` mais velhas que `older_than`. Um
/// arquivo em uso (o assistente aberto) fica.
pub fn clean_old_work_dirs(base: &std::path::Path, older_than: Duration) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > older_than);
        if old
            && entry.file_name().to_string_lossy().starts_with(WORK_PREFIX)
            && entry.file_type().is_ok_and(|t| t.is_dir())
        {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Como mostrar o progresso de um download de arquivo inteiro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bar {
    /// A barra do próprio `curl`, no stderr (a CLI num terminal).
    Curl,
    /// Sem barra: os bytes já baixados vão para quem chamou.
    Quiet,
}

/// De onde vêm as versões e os arquivos das releases: a rede, em
/// [`Remote`]; um substituto nos testes.
pub trait Sources: Send + Sync {
    /// A última versão da AURORA publicada (a tag sem o `v`).
    fn latest_aurora(&self, cancel: &CancelToken) -> Result<String>;
    /// O `bundle/versions.json` da release `v<version>` da AURORA.
    fn release_versions(&self, version: &str, cancel: &CancelToken) -> Result<String>;
    /// A última versão upstream de um pacote de `bundle/versions.json`.
    fn upstream(&self, package: &str, cancel: &CancelToken) -> Result<String>;
    /// Um arquivo da release `v<version>` em `dir`, conferido pelo
    /// `SHA256SUMS` dela (baixado uma vez para `dir`). `downloaded` recebe
    /// os bytes já baixados enquanto o download roda.
    fn release_file(
        &self,
        version: &str,
        file: &str,
        dir: &Utf8Path,
        downloaded: &mut dyn FnMut(u64),
        cancel: &CancelToken,
    ) -> Result<Utf8PathBuf>;
}

/// Os pacotes de `bundle/versions.json`, de onde saem os componentes.
pub mod package {
    /// O OSS CAD Suite.
    pub const OSS_CAD_SUITE: &str = "oss-cad-suite";
    /// O YANC.
    pub const YANC: &str = "yanc";
    /// O surfer-aurora.
    pub const SURFER_AURORA: &str = "surfer-aurora";
    /// O Graphviz (o pacote oficial, no Windows).
    pub const GRAPHVIZ: &str = "graphviz";
    /// O bloco MSYS2 do aurora-toolchain (Windows).
    pub const MSYS: &str = "msys";
    /// A AURORA IDE, pronta da release do repositório aurora-ide, com versão
    /// própria.
    pub const STUDIO: &str = "studio";
    /// As trilhas do `aurora learn`, do próprio repositório: a versão é a do
    /// AURORA.
    pub const AURORA_LEARN: &str = "aurora-learn";
    /// O modelo do RTL viewer, do próprio repositório: a versão é a da AURORA.
    pub const RTL_VIEWER: &str = "rtl-viewer";
}

/// O repositório do bloco de Windows (o pacote `msys`).
const AURORA_TOOLCHAIN: &str = "nipscernlab-sandbox/aurora-toolchain";

/// O repositório da AURORA IDE (o pacote `studio`).
const AURORA_IDE: &str = "nipscernlab-sandbox/aurora-ide";

/// As tags do surfer-aurora no GitLab.
const SURFER_AURORA_TAGS: &str =
    "https://gitlab.com/api/v4/projects/nips-cern%2Fsurfer-aurora/repository/tags?per_page=20";
/// A última release do Graphviz no GitLab (o projeto `graphviz/graphviz`).
const GRAPHVIZ_RELEASES: &str = "https://gitlab.com/api/v4/projects/4207231/releases?per_page=1";

/// As fontes reais, pela internet.
#[derive(Debug, Clone)]
pub struct Remote(pub Network);

impl Remote {
    /// A tag da última release de um repositório do GitHub, sem passar pela
    /// API (que limita as consultas): `/releases/latest` redireciona para
    /// `/releases/tag/<tag>`. Uma tag fora dos caracteres de um nome de
    /// arquivo de release é recusada.
    pub fn latest_github_tag(&self, repo: &str, cancel: &CancelToken) -> Result<String> {
        let url = format!("https://github.com/{repo}/releases/latest");
        let mut command = self.0.curl();
        command
            .args(["-fsSIL", "-o", null_device(), "-w", "%{url_effective}"])
            .arg(&url)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::piped());
        let output = run_curl(command, &mut || {}, cancel)?;
        let effective = String::from_utf8_lossy(&output.stdout);
        match effective.rsplit_once("/releases/tag/") {
            Some((_, tag)) if output.status.success() && is_asset_name(tag.trim()) => {
                Ok(tag.trim().to_owned())
            }
            _ => Err(Error::network(
                url.clone(),
                format!("Could not look up the latest release at {url}"),
            )),
        }
    }

    /// O conteúdo de `url`, como texto.
    pub fn fetch_text(&self, url: &str, cancel: &CancelToken) -> Result<String> {
        let mut command = self.0.curl();
        command
            .args(["-fsSL", url])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::piped());
        let output = run_curl(command, &mut || {}, cancel)?;
        if !output.status.success() {
            return Err(Error::network(url, format!("Could not download {url}")));
        }
        Ok(String::from_utf8(output.stdout).with_context(|| format!("{url} is not UTF-8 text"))?)
    }

    /// Baixa `url` para `dest`, com a barra do `curl` no stderr ou em
    /// silêncio avisando os bytes baixados.
    pub fn download(
        &self,
        url: &str,
        dest: &Utf8Path,
        bar: Bar,
        downloaded: &mut dyn FnMut(u64),
        cancel: &CancelToken,
    ) -> Result<()> {
        let mut command = self.0.curl();
        command
            .arg("-fL")
            .arg(match bar {
                Bar::Curl => "--progress-bar",
                Bar::Quiet => "-sS",
            })
            .arg("-o")
            .arg(dest.as_std_path())
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null());
        if bar == Bar::Quiet {
            command.stderr(Stdio::null());
        }
        let output = run_curl(
            command,
            &mut || {
                if let Ok(meta) = std::fs::metadata(dest) {
                    downloaded(meta.len());
                }
            },
            cancel,
        )?;
        if !output.status.success() {
            return Err(Error::network(url, format!("Could not download {url}")));
        }
        Ok(())
    }

    /// Como [`Sources::release_file`], com a barra do `curl` no stderr.
    pub fn release_file_with_bar(
        &self,
        version: &str,
        file: &str,
        dir: &Utf8Path,
        cancel: &CancelToken,
    ) -> Result<Utf8PathBuf> {
        self.release_file_as(version, file, dir, Bar::Curl, &mut |_| {}, cancel)
    }

    fn release_file_as(
        &self,
        version: &str,
        file: &str,
        dir: &Utf8Path,
        bar: Bar,
        downloaded: &mut dyn FnMut(u64),
        cancel: &CancelToken,
    ) -> Result<Utf8PathBuf> {
        let sums = dir.join("SHA256SUMS");
        if !sums.is_file() {
            self.download(
                &self.0.asset_url(version, "SHA256SUMS")?,
                &sums,
                Bar::Quiet,
                &mut |_| {},
                cancel,
            )?;
        }
        let url = self.0.asset_url(version, file)?;
        let dest = dir.join(file);
        self.download(&url, &dest, bar, downloaded, cancel)?;
        let sums = std::fs::read_to_string(&sums).with_context(|| format!("Reading {sums}"))?;
        let expected = expected_sha256(&sums, file).with_context(|| {
            format!("The SHA256SUMS of release v{version} does not list {file}")
        })?;
        if sha256_of(&dest)? != expected {
            return Err(Error::ChecksumMismatch {
                file: file.to_owned(),
                version: version.to_owned(),
            });
        }
        Ok(dest)
    }
}

impl Sources for Remote {
    /// Com um espelho, o arquivo `latest` dele, com a versão numa linha, se
    /// houver; sem ele, a do GitHub.
    fn latest_aurora(&self, cancel: &CancelToken) -> Result<String> {
        if let Some(base) = &self.0.mirror {
            let url = format!("{base}/latest");
            match self.fetch_text(&url, cancel) {
                Ok(text) if !text.trim().is_empty() => {
                    let version = text.trim().trim_start_matches('v');
                    if !is_version(version) {
                        return Err(anyhow::anyhow!("{url} does not hold an AURORA version").into());
                    }
                    return Ok(version.to_owned());
                }
                Err(Error::Cancelled) => return Err(Error::Cancelled),
                _ => tracing::info!("No {url}: the latest version comes from GitHub"),
            }
        }
        let tag = self.latest_github_tag(&self.0.repo, cancel)?;
        let version = tag.trim_start_matches('v');
        if !is_version(version) {
            return Err(anyhow::anyhow!(
                "The latest release of {} is not tagged with an AURORA version",
                self.0.repo
            )
            .into());
        }
        Ok(version.to_owned())
    }

    /// Pelo `raw.githubusercontent.com` do repositório, e não pelo espelho.
    fn release_versions(&self, version: &str, cancel: &CancelToken) -> Result<String> {
        self.fetch_text(
            &format!(
                "https://raw.githubusercontent.com/{}/v{version}/bundle/versions.json",
                self.0.repo
            ),
            cancel,
        )
    }

    fn upstream(&self, name: &str, cancel: &CancelToken) -> Result<String> {
        match name {
            package::OSS_CAD_SUITE => self.latest_github_tag("YosysHQ/oss-cad-suite-build", cancel),
            package::MSYS => self.latest_github_tag(AURORA_TOOLCHAIN, cancel),
            package::YANC => self.latest_github_tag("nipscernlab/yanc", cancel),
            // A IDE tem releases próprias, com a versão na tag (v8.1.0).
            package::STUDIO => Ok(self
                .latest_github_tag(AURORA_IDE, cancel)?
                .trim_start_matches('v')
                .to_owned()),
            // As trilhas e o RTL viewer saem com a AURORA: o mais novo é o da
            // última release.
            package::AURORA_LEARN | package::RTL_VIEWER => self.latest_aurora(cancel),
            package::SURFER_AURORA => {
                let text = self.fetch_text(SURFER_AURORA_TAGS, cancel)?;
                let tags: Vec<GitlabTag> = serde_json::from_str(&text)
                    .with_context(|| format!("Invalid response from {SURFER_AURORA_TAGS}"))?;
                Ok(newest(tags.into_iter().map(|t| t.name))
                    .with_context(|| format!("No versioned tag in {SURFER_AURORA_TAGS}"))?)
            }
            package::GRAPHVIZ => {
                let text = self.fetch_text(GRAPHVIZ_RELEASES, cancel)?;
                let releases: Vec<GitlabRelease> = serde_json::from_str(&text)
                    .with_context(|| format!("Invalid response from {GRAPHVIZ_RELEASES}"))?;
                Ok(releases
                    .into_iter()
                    .next()
                    .map(|r| r.tag_name)
                    .with_context(|| format!("No release in {GRAPHVIZ_RELEASES}"))?)
            }
            _ => Err(anyhow::anyhow!("No upstream source known for {name}").into()),
        }
    }

    fn release_file(
        &self,
        version: &str,
        file: &str,
        dir: &Utf8Path,
        downloaded: &mut dyn FnMut(u64),
        cancel: &CancelToken,
    ) -> Result<Utf8PathBuf> {
        self.release_file_as(version, file, dir, Bar::Quiet, downloaded, cancel)
    }
}

/// Uma tag na API do GitLab.
#[derive(serde::Deserialize)]
struct GitlabTag {
    name: String,
}

/// Uma release na API do GitLab.
#[derive(serde::Deserialize)]
struct GitlabRelease {
    tag_name: String,
}

/// A chave de ordem de uma versão: os grupos de dígitos, como números, na
/// ordem em que aparecem. Serve aos formatos do bundle (`2026-09-29`, `v5.6`,
/// `v0.7.0-nips.10`, `16.1.0`) e ao da AURORA (`0.1.0`). Um sufixo com dígitos
/// fica depois da versão sem ele (`v0.7.0` < `v0.7.0-nips.1`), como nas tags
/// do surfer-aurora; numa pré-versão no estilo do semver (`0.2.0-rc.1`), essa
/// seria a ordem errada.
fn version_key(version: &str) -> Vec<u64> {
    version
        .split(|c: char| !c.is_ascii_digit())
        .filter(|group| !group.is_empty())
        .map(|group| group.parse().unwrap_or(u64::MAX))
        .collect()
}

/// `candidate` é mais nova que `current`. Uma versão sem dígitos (a tag
/// `test`) não se compara: nunca é mais nova, nem mais velha.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    let (candidate, current) = (version_key(candidate), version_key(current));
    !candidate.is_empty() && !current.is_empty() && candidate > current
}

/// A maior versão de uma lista de tags; as sem dígitos ficam de fora.
pub fn newest(tags: impl IntoIterator<Item = String>) -> Option<String> {
    tags.into_iter()
        .map(|tag| (version_key(&tag), tag))
        .filter(|(key, _)| !key.is_empty())
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, tag)| tag)
}

/// Roda o `curl` até o fim, chamando `tick` a cada volta, e junta o que ele
/// escreveu no stdout. Com o pedido de cancelamento, encerra-o: o Ctrl+C do
/// terminal chega a ele também, mas um pedido que vem de dentro do processo
/// (o Studio que cancela) não, e ele ficava baixando sozinho.
fn run_curl(
    mut command: std::process::Command,
    tick: &mut dyn FnMut(),
    cancel: &CancelToken,
) -> Result<std::process::Output> {
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Error::CurlMissing
        } else {
            Error::Other(anyhow::Error::new(error).context("Running curl"))
        }
    })?;
    // O stdout, quando há, é lido numa thread: um `wait` com o pipe cheio
    // travaria os dois.
    let stdout = child.stdout.take().map(|mut out| {
        std::thread::spawn(move || {
            let mut all = Vec::new();
            let _ = out.read_to_end(&mut all);
            all
        })
    });
    let status = loop {
        if let Some(status) = child.try_wait().context("Waiting for curl")? {
            break status;
        }
        if cancel.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::Cancelled);
        }
        tick();
        std::thread::sleep(Duration::from_millis(100));
    };
    let stdout = stdout
        .map(|reader| reader.join().unwrap_or_default())
        .unwrap_or_default();
    Ok(std::process::Output {
        status,
        stdout,
        stderr: Vec::new(),
    })
}

fn null_device() -> &'static str {
    if cfg!(windows) { "NUL" } else { "/dev/null" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn network() -> Network {
        Network {
            repo: Network::DEFAULT_REPO.to_owned(),
            mirror: None,
            curl: None,
            work_base: Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap(),
            timeout: None,
        }
    }

    #[test]
    fn only_old_work_folders_go() {
        let base = tempfile::tempdir().unwrap();
        let base = base.path();
        for dir in ["aurora-install.a", "aurora-install.b", "outra"] {
            std::fs::create_dir(base.join(dir)).unwrap();
        }
        std::fs::write(base.join("aurora-install.c"), "um arquivo").unwrap();
        // Nenhuma tem um dia.
        clean_old_work_dirs(base, WORK_DIR_LEFTOVER);
        assert!(base.join("aurora-install.a").is_dir());
        // Todas são velhas: só as pastas de trabalho saem.
        std::thread::sleep(Duration::from_millis(20));
        clean_old_work_dirs(base, Duration::ZERO);
        assert!(!base.join("aurora-install.a").exists());
        assert!(!base.join("aurora-install.b").exists());
        assert!(base.join("outra").is_dir());
        assert!(base.join("aurora-install.c").is_file());
    }

    #[test]
    fn sha256sums_lines_are_read_in_both_modes() {
        let sums = "\
AAAA  aurora-0.2.0-linux-x64.tar.gz
bbbb *aurora-0.2.0-windows-x64-setup.exe
";
        assert_eq!(
            expected_sha256(sums, "aurora-0.2.0-linux-x64.tar.gz").as_deref(),
            Some("aaaa")
        );
        assert_eq!(
            expected_sha256(sums, "aurora-0.2.0-windows-x64-setup.exe").as_deref(),
            Some("bbbb")
        );
        assert_eq!(
            expected_sha256(sums, "aurora-0.2.0-darwin-arm64.tar.gz"),
            None
        );
    }

    #[test]
    fn versions_and_file_names_cannot_leave_the_release() {
        for ok in ["0.8.0", "0.9.0-rc.1", "1.0.0+build.5", "2026.10.13"] {
            assert!(is_version(ok), "{ok}");
        }
        for bad in [
            "",
            "v0.8.0",
            "0.8.0/../../x",
            "0.8\x1b[2J",
            "0.8.0 x",
            "..",
            "0.8.0\n",
        ] {
            assert!(!is_version(bad), "{bad:?}");
        }
        for ok in [
            "SHA256SUMS",
            "aurora-0.8.0-windows-x64-index.json",
            "aurora-0.8.0_x.tar.gz",
        ] {
            assert!(is_asset_name(ok), "{ok}");
        }
        for bad in ["", "../x", "a/b", "a\\b", ".hidden", "x\x1b]0;t\x07"] {
            assert!(!is_asset_name(bad), "{bad:?}");
        }
        let network = network();
        assert!(network.asset_url("0.8.0/../..", "SHA256SUMS").is_err());
        assert!(network.asset_url("0.8.0", "../aurora.exe").is_err());
        assert_eq!(
            network.asset_url("0.8.0", "SHA256SUMS").unwrap(),
            "https://github.com/nipscernlab-sandbox/aurora/releases/download/v0.8.0/SHA256SUMS"
        );
        let mirrored = Network {
            mirror: Some("https://lab.example/aurora".into()),
            ..network
        };
        assert_eq!(
            mirrored.asset_url("0.8.0", "SHA256SUMS").unwrap(),
            "https://lab.example/aurora/v0.8.0/SHA256SUMS"
        );
    }

    #[test]
    fn a_mirror_is_https_or_on_disk() {
        assert_eq!(Network::validate_mirror(None).unwrap(), None);
        assert_eq!(Network::validate_mirror(Some("")).unwrap(), None);
        assert_eq!(
            Network::validate_mirror(Some("https://lab.example/aurora/"))
                .unwrap()
                .as_deref(),
            Some("https://lab.example/aurora")
        );
        assert!(
            Network::validate_mirror(Some("HTTPS://lab.example"))
                .unwrap()
                .is_some()
        );
        assert!(
            Network::validate_mirror(Some("file:///srv/aurora"))
                .unwrap()
                .is_some()
        );
        for bad in [
            "http://lab.example/aurora",
            "ftp://lab.example",
            "lab.example/aurora",
        ] {
            let error = Network::validate_mirror(Some(bad)).unwrap_err();
            assert_eq!(error.code(), "invalid_mirror", "{bad}");
            assert!(error.to_string().contains(bad));
        }
    }

    #[test]
    fn sha256_of_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().join("abc")).unwrap();
        std::fs::write(&path, "abc").unwrap();
        assert_eq!(
            sha256_of(&path).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn versions_compare_by_their_digit_groups() {
        for (older, newer) in [
            ("2026-09-29", "2026-10-01"),
            ("2026-09-29", "2027-01-01"),
            ("v5.6", "v5.7"),
            ("v5.6", "v5.6.1"),
            ("v5.9", "v5.10"),
            ("v0.7.0-nips.9", "v0.7.0-nips.10"),
            ("v0.7.0", "v0.7.0-nips.1"),
            ("v0.6.0", "v0.7.0"),
            ("16.1.0", "16.10.0"),
            ("0.1.0", "0.2.0"),
            ("2026.09.29", "2026.10.15"),
        ] {
            assert!(is_newer(newer, older), "{newer} > {older}");
            assert!(!is_newer(older, newer), "{older} < {newer}");
        }
        for (a, b) in [("v5.6", "v5.6"), ("0.1.0", "v0.1.0"), ("test", "v5.6")] {
            assert!(!is_newer(a, b) && !is_newer(b, a), "{a} e {b}");
        }
    }

    #[test]
    fn the_newest_tag_wins_whatever_the_order() {
        // As tags do surfer-aurora como a API do GitLab as devolve.
        let tags = [
            "v0.7.0-nips.10",
            "v0.7.0-nips.9",
            "v0.7.0-nips.2",
            "v0.7.0-nips.1",
            "v0.7.0",
            "v0.6.0",
            "0.1.0",
            "v0.1.0",
            "test",
        ];
        let mut shuffled = tags.map(str::to_owned).to_vec();
        shuffled.reverse();
        assert_eq!(newest(shuffled).as_deref(), Some("v0.7.0-nips.10"));
        assert_eq!(newest(["test".to_owned()]), None);
    }

    #[test]
    fn a_cancelled_token_stops_before_running_curl() {
        let cancel = CancelToken::new();
        cancel.cancel();
        let remote = Remote(network());
        assert!(matches!(
            remote.fetch_text("https://example.invalid/x", &cancel),
            Err(Error::Cancelled)
        ));
    }
}
