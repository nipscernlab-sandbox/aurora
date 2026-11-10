//! A orquestração do `aurora update`: consultar as versões, planejar a
//! atualização por componentes, aplicá-la e, quando não dá, cair para o
//! instalador inteiro. É a regra que a CLI, o Studio e o `aurora serve`
//! compartilham; cada um só põe o texto e as perguntas dele pelos
//! [`Hooks`] e mostra o progresso pelos [`Event`].
//!
//! As ferramentas de terceiros vêm no bundle, fechado e versionado por
//! release da AURORA (`bundle/versions.json` fixa a versão e o hash de cada
//! pacote). Uma versão nova de uma ferramenta só chega ao usuário num bundle
//! novo, publicado numa release nova da AURORA. Por isso a atualização não
//! instala nada direto do upstream: ela só mostra, para cada componente, a
//! versão instalada, a do bundle da última release e a última upstream.
//!
//! A atualização vai por componentes quando dá: a instalação guarda o
//! manifesto de arquivos da versão dela (`toolchain/files.json`, desde a
//! 0.7.0), a release nova publica o seu, e só os pedaços com arquivos que
//! mudaram são baixados, conferidos pelo `SHA256SUMS` dela, e trocados no
//! lugar ([`super::apply`]). Uma falha no meio devolve a instalação ao que
//! era. Um instalador no disco faz o mesmo sem rede ([`prepare_from_disk`]).
//!
//! Sem o manifesto (uma instalação anterior à 0.7.0), com `full` ou se a
//! atualização por componentes falha, quem atualiza é o instalador inteiro
//! da versão nova, baixado da release e conferido pelo `SHA256SUMS` dela
//! ([`fallback_installer`]): no Linux e no macOS, o `install` dele, que
//! troca o `bin/aurora` e o `toolchain/` inteiro; no Windows, o assistente de
//! instalação, que troca o `aurora.exe` e por isso roda depois que quem chamou
//! sai.

use std::collections::BTreeMap;
use std::io::BufRead;
use std::process::Stdio;

use anyhow::Context;
use aurora_core::{BundleManifest, CancelToken, Platform, component};
use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::add::{ChunkSource, DiskPayload, ReleasePayload};
use crate::error::{Error, Result, from_anyhow};
use crate::files::{self, FilesManifest};
use crate::install::Event;
use crate::installation;
use crate::pack;
use crate::payload::{self, Index};
use crate::release::{Bar, Network, Remote, Sources, is_newer, package};

use super::{Applied, Plan};

/// `return Err(Error::Other(anyhow!(...)))`: um erro sem código próprio.
macro_rules! fail {
    ($($arg:tt)*) => {
        return Err(Error::Other(anyhow::anyhow!($($arg)*)))
    };
}

/// O relatório do `aurora update` (`docs/schema/update.json`).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct UpdateReport {
    /// A AURORA: este e o da última release.
    pub aurora: AuroraVersions,
    /// O bundle de ferramentas: o instalado e o da última release.
    pub bundle: BundleVersions,
    /// Cada componente instalado, na ordem do manifesto do bundle. Vazio sem
    /// bundle.
    pub components: Vec<ComponentVersions>,
    /// A pasta da instalação; `null` se este `aurora` não foi instalado pelo
    /// instalador (um build em `target/`, por exemplo).
    #[schemars(with = "Option<String>")]
    pub prefix: Option<Utf8PathBuf>,
    /// O que o comando fez.
    pub action: UpdateAction,
    /// Como atualizou; `null` quando não atualizou.
    pub method: Option<UpdateMethod>,
    /// O que a atualização por componentes trocou, ou vai trocar com
    /// `--check --from`; `null` quando ela não rodou.
    pub changes: Option<UpdateChanges>,
}

/// Como `aurora update` atualizou.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UpdateMethod {
    /// Só os arquivos que mudaram, dos pedaços que a release publica.
    Components,
    /// O instalador inteiro da versão nova.
    Installer,
}

/// O que a atualização por componentes trocou.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateChanges {
    /// Os componentes com arquivos que entraram ou saíram, na ordem do
    /// bundle; `aurora` é a própria AURORA e o cabeçalho do bundle.
    pub components: Vec<String>,
    /// Bytes baixados.
    pub download_bytes: u64,
    /// Arquivos que entraram (novos ou trocados).
    pub files: usize,
    /// Arquivos que saíram.
    pub removed: usize,
}

impl UpdateChanges {
    /// O que um plano vai trocar.
    pub fn of(plan: &Plan) -> UpdateChanges {
        UpdateChanges {
            components: plan.changed.clone(),
            download_bytes: plan.download(),
            files: plan.files(),
            removed: plan.stale.len(),
        }
    }
}

/// As versões da AURORA em `aurora update`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AuroraVersions {
    /// A versão deste `aurora`.
    pub installed: String,
    /// A versão da última release no GitHub.
    pub latest: String,
    /// `latest` é mais nova que `installed`.
    pub newer: bool,
}

/// As versões do bundle em `aurora update`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BundleVersions {
    /// O bundle instalado; `null` sem bundle.
    pub installed: Option<String>,
    /// O bundle da última release; `null` se o `bundle/versions.json` dela
    /// não pôde ser lido.
    pub latest: Option<String>,
    /// `latest` é mais novo que `installed`.
    pub newer: bool,
}

/// Um componente instalado em `aurora update`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ComponentVersions {
    /// O componente (`yanc`, `icarus`, `verilator`, `yosys`, `graphviz`,
    /// `surfer-aurora`).
    pub name: String,
    /// O pacote de `bundle/versions.json` de onde ele sai (`oss-cad-suite`,
    /// `yanc`, `surfer-aurora`, `graphviz`); `null` para um componente que
    /// esta AURORA não conhece.
    pub package: Option<String>,
    /// A versão instalada.
    pub installed: String,
    /// A versão no bundle da última release; `null` se não pôde ser lida.
    pub release: Option<String>,
    /// `release` é mais nova que `installed`: chega atualizando a AURORA.
    pub release_newer: bool,
    /// A última versão upstream; `null` se a consulta falhou.
    pub upstream: Option<String>,
    /// `upstream` é mais nova que `installed` e que `release`: só chega num
    /// bundle novo, numa release nova da AURORA. Sem `release`, `false` quando
    /// há AURORA mais nova, cujo bundle pode já trazê-la.
    pub upstream_newer: bool,
}

/// O que `aurora update` fez.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UpdateAction {
    /// `--check`: só comparou as versões.
    Checked,
    /// Não há AURORA mais nova publicado: nada mudou.
    UpToDate,
    /// A instalação foi trocada (por componentes, ou pelo instalador no
    /// Linux e no macOS). O `aurora` em uso é o antigo: quem chamou termina e
    /// reinicia.
    Updated,
    /// O assistente de instalação do Windows foi aberto e termina depois que
    /// quem chamou sai.
    WizardOpened,
}

/// O que interessa do `bundle/versions.json` de uma release: o bundle e a
/// versão de cada pacote.
#[derive(Deserialize)]
pub(crate) struct ReleaseBundle {
    pub(crate) bundle: String,
    pub(crate) packages: BTreeMap<String, ReleasePackage>,
}

#[derive(Deserialize)]
pub(crate) struct ReleasePackage {
    pub(crate) version: String,
}

/// O pacote de `bundle/versions.json` de onde sai um componente, como em
/// `bundle/components.json`: no Windows, Icarus, Verilator e cocotb vêm do
/// bloco MSYS2 do aurora-toolchain e o Graphviz do pacote oficial dele; o
/// resto, e tudo isso no Linux e no macOS, do OSS CAD Suite. O YANC, o
/// surfer-aurora, o Studio, as trilhas do `aurora learn` e o RTL viewer têm
/// pacote próprio.
pub fn package_of(name: &str, platform: &str) -> Option<&'static str> {
    let windows = platform == "windows-x64";
    match name {
        component::YANC => Some(package::YANC),
        component::SURFER_AURORA => Some(package::SURFER_AURORA),
        component::STUDIO => Some(package::STUDIO),
        component::AURORA_LEARN => Some(package::AURORA_LEARN),
        component::RTL_VIEWER => Some(package::RTL_VIEWER),
        // O cocotb do Windows sai do bloco MSYS2, com o Icarus e o Verilator.
        component::ICARUS | component::VERILATOR | component::GHDL | component::COCOTB
            if windows =>
        {
            Some(package::MSYS)
        }
        component::GRAPHVIZ if windows => Some(package::GRAPHVIZ),
        component::ICARUS
        | component::VERILATOR
        | component::GHDL
        | component::COCOTB
        | component::YOSYS
        | component::GRAPHVIZ
        | component::OPENFPGALOADER => Some(package::OSS_CAD_SUITE),
        _ => None,
    }
}

/// Consulta as versões e monta o relatório, com `action` em `Checked` e sem
/// `prefix`. Sem a última versão da AURORA é erro ([`Error::NoLatestVersion`]);
/// sem o bundle da release ou sem uma versão upstream, a coluna fica vazia.
/// Cada consulta upstream roda numa thread, junto com a do GitHub.
pub fn check(
    sources: &dyn Sources,
    installed: &str,
    manifest: Option<&BundleManifest>,
    cancel: &CancelToken,
) -> Result<UpdateReport> {
    let components = manifest.map_or(&[][..], |m| m.components.as_slice());
    let platform = manifest.map_or("", |m| m.platform.as_str());
    let packages: Vec<Option<&'static str>> = components
        .iter()
        .map(|c| package_of(&c.name, platform))
        .collect();
    let mut queried: Vec<&'static str> = packages.iter().flatten().copied().collect();
    queried.sort_unstable();
    queried.dedup();

    let (latest, upstream) = std::thread::scope(|scope| {
        let handles: Vec<_> = queried
            .iter()
            .map(|&p| (p, scope.spawn(move || sources.upstream(p, cancel))))
            .collect();
        let latest = sources.latest_aurora(cancel);
        let upstream: BTreeMap<&str, Option<String>> = handles
            .into_iter()
            .map(|(p, handle)| {
                let version = match handle.join() {
                    Ok(Ok(version)) => Some(version),
                    Ok(Err(error)) => {
                        tracing::info!("No upstream version for {p}: {error:#}");
                        None
                    }
                    Err(_) => None,
                };
                (p, version)
            })
            .collect();
        (latest, upstream)
    });
    let latest = match latest {
        Ok(latest) => latest,
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(error) => {
            tracing::info!("No latest AURORA version: {error:#}");
            return Err(Error::NoLatestVersion);
        }
    };
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }

    let release = sources
        .release_versions(&latest, cancel)
        .and_then(|text| {
            serde_json::from_str::<ReleaseBundle>(&text)
                .context("Invalid bundle/versions.json")
                .map_err(Error::Other)
        })
        .inspect_err(|error| tracing::info!("No bundle for release v{latest}: {error:#}"))
        .ok();

    let aurora_newer = is_newer(&latest, installed);
    let components = components
        .iter()
        .zip(packages)
        .map(|(c, package)| {
            let in_release = package
                .and_then(|p| release.as_ref()?.packages.get(p))
                .map(|p| p.version.clone());
            let upstream = package.and_then(|p| upstream.get(p).cloned().flatten());
            let release_newer = in_release
                .as_deref()
                .is_some_and(|r| is_newer(r, &c.version));
            // Sem o bundle da release, só dá para dizer quando não há AURORA
            // mais novo: o bundle dele pode já trazer a versão upstream.
            let upstream_newer = upstream.as_deref().is_some_and(|u| {
                is_newer(u, &c.version)
                    && match in_release.as_deref() {
                        Some(r) => is_newer(u, r),
                        None => !aurora_newer,
                    }
            });
            ComponentVersions {
                name: c.name.clone(),
                package: package.map(str::to_owned),
                installed: c.version.clone(),
                release: in_release,
                release_newer,
                upstream,
                upstream_newer,
            }
        })
        .collect();

    let installed_bundle = manifest.map(|m| m.bundle.clone());
    let latest_bundle = release.map(|r| r.bundle);
    let bundle_newer = match (&latest_bundle, &installed_bundle) {
        (Some(latest), Some(installed)) => is_newer(latest, installed),
        _ => false,
    };
    Ok(UpdateReport {
        aurora: AuroraVersions {
            installed: installed.to_owned(),
            newer: aurora_newer,
            latest,
        },
        bundle: BundleVersions {
            installed: installed_bundle,
            latest: latest_bundle,
            newer: bundle_newer,
        },
        components,
        prefix: None,
        action: UpdateAction::Checked,
        method: None,
        changes: None,
    })
}

/// As versões upstream mais novas que as do bundle, uma por pacote, com os
/// componentes instalados que saem dele, na ordem do manifesto.
pub fn upstream_news(components: &[ComponentVersions]) -> Vec<(&str, &str, Vec<&str>)> {
    let mut news: Vec<(&str, &str, Vec<&str>)> = Vec::new();
    for c in components.iter().filter(|c| c.upstream_newer) {
        let (Some(package), Some(version)) = (c.package.as_deref(), c.upstream.as_deref()) else {
            continue;
        };
        match news.iter_mut().find(|(p, _, _)| *p == package) {
            Some((_, _, names)) => names.push(&c.name),
            None => news.push((package, version, vec![&c.name])),
        }
    }
    news
}

/// A atualização por componentes, preparada: o índice e o manifesto da
/// versão nova, o plano, e de onde vêm os pedaços.
pub struct Prepared {
    /// O índice da versão nova.
    pub index: Index,
    /// O manifesto de arquivos da versão nova.
    pub manifest: FilesManifest,
    /// O que vai mudar.
    pub plan: Plan,
    /// Os pedaços vêm de um instalador no disco, não da rede.
    pub local: bool,
    source: Box<dyn ChunkSource>,
    /// A pasta dos downloads, apagada no fim.
    _work: Option<tempfile::TempDir>,
}

impl Prepared {
    /// O nome de um componente para mostrar, com os rótulos do índice novo:
    /// `AURORA` para o `aurora`.
    pub fn label(&self, name: &str) -> String {
        label_of(&self.index, name)
    }

    /// O que o plano vai trocar.
    pub fn changes(&self) -> UpdateChanges {
        UpdateChanges::of(&self.plan)
    }

    /// Nada a trocar: a instalação já tem todos os arquivos da versão nova.
    pub fn is_noop(&self) -> bool {
        self.plan.chunks.is_empty() && self.plan.stale.is_empty()
    }
}

impl std::fmt::Debug for Prepared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Prepared")
            .field("plan", &self.plan)
            .field("local", &self.local)
            .finish_non_exhaustive()
    }
}

fn label_of(index: &Index, name: &str) -> String {
    if name == super::AURORA {
        return "AURORA".to_owned();
    }
    index
        .component(name)
        .map_or_else(|| name.to_owned(), |c| c.label.clone())
}

/// O manifesto de arquivos da instalação; `None` numa instalação anterior à
/// 0.7.0, que não o tem.
pub fn installed_manifest(prefix: &Utf8Path) -> Result<Option<FilesManifest>> {
    let path = prefix.join(files::INSTALLED);
    if !path.is_file() {
        tracing::info!("{path} does not exist: the installation is older than AURORA 0.7.0");
        return Ok(None);
    }
    Ok(FilesManifest::load(path.as_std_path()).map(Some)?)
}

fn utf8_dir(work: &tempfile::TempDir) -> Result<Utf8PathBuf> {
    Ok(Utf8Path::from_path(work.path())
        .context("Temporary folder path is not UTF-8")?
        .to_owned())
}

/// Prepara a atualização por componentes para a release `v<version>`:
/// baixa o índice e o manifesto de arquivos dela e compara com a
/// instalação. `None` quando não dá (a instalação ou a release sem o
/// manifesto): quem chama atualiza com o instalador inteiro.
pub fn prepare(
    sources: &std::sync::Arc<dyn Sources>,
    network: &Network,
    version: &str,
    prefix: &Utf8Path,
    cancel: &CancelToken,
) -> Result<Option<Prepared>> {
    let Some(installed) = installed_manifest(prefix)? else {
        return Ok(None);
    };
    let platform = Platform::current()
        .context("Na AURORA release for this platform")?
        .as_str();
    let work = network.work_dir()?;
    let dir = utf8_dir(&work)?;
    let get = |file: &str| -> Result<String> {
        let name = payload::release_asset(version, platform, file);
        let path = sources.release_file(version, &name, &dir, &mut |_| {}, cancel)?;
        Ok(std::fs::read_to_string(&path).with_context(|| format!("Reading {path}"))?)
    };
    let index = Index::parse(&get(payload::INDEX_FILE)?)
        .with_context(|| format!("Invalid index in release v{version}"))?;
    if index.aurora_version != version {
        fail!(
            "The index of release v{version} is for AURORA {}",
            index.aurora_version
        );
    }
    let new = match get(files::FILES_FILE) {
        Ok(text) => FilesManifest::parse(&text)
            .with_context(|| format!("Invalid file manifest in release v{version}"))?,
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(error) => {
            tracing::info!("Release v{version} has no file manifest: {error:#}");
            return Ok(None);
        }
    };
    let plan = super::plan(prefix.as_std_path(), &installed, &index, &new)?;
    Ok(Some(Prepared {
        index,
        manifest: new,
        plan,
        source: Box::new(ReleasePayload::new(
            sources.clone(),
            version.to_owned(),
            platform.to_owned(),
            dir,
            cancel.clone(),
        )),
        local: false,
        _work: Some(work),
    }))
}

/// A atualização por componentes a partir de um instalador no disco, até
/// o plano, com o relatório que `--check --from` devolve.
pub struct PreparedFromDisk {
    /// A atualização preparada.
    pub prepared: Prepared,
    /// O relatório, com `action: checked`, `prefix` e `changes` preenchidos.
    pub report: UpdateReport,
}

/// `aurora update --from`: a versão do instalador em `path`, por componentes e
/// sem rede. Recusa um instalador mais velho que esta AURORA.
pub fn prepare_from_disk(
    network: &Network,
    prefix: &Utf8Path,
    path: &Utf8Path,
    this_version: &str,
    manifest: Option<&BundleManifest>,
    cancel: &CancelToken,
) -> Result<PreparedFromDisk> {
    let work = network.work_dir()?;
    let work_dir = utf8_dir(&work)?;
    let disk = DiskPayload::find(path, &work_dir, cancel.clone())?;
    let index = disk.index()?;
    let new = disk.files()?.with_context(|| {
        format!(
            "The installer in {path} has no file manifest (files.json): it is older than AURORA \
             0.7.0, and updates only with the installer itself"
        )
    })?;
    let installed = installed_manifest(prefix)?.with_context(|| {
        format!(
            "The installation in {prefix} has no file manifest (it is older than AURORA 0.7.0): \
             update it with the installer, or with aurora update --full"
        )
    })?;
    if is_newer(this_version, &index.aurora_version) {
        fail!(
            "The installer in {path} is AURORA {}, older than this AURORA ({this_version})",
            index.aurora_version
        );
    }
    let plan = super::plan(prefix.as_std_path(), &installed, &index, &new)?;
    let report = UpdateReport {
        aurora: AuroraVersions {
            installed: this_version.to_owned(),
            latest: index.aurora_version.clone(),
            newer: is_newer(&index.aurora_version, this_version),
        },
        bundle: BundleVersions {
            installed: manifest.map(|m| m.bundle.clone()),
            latest: Some(index.bundle.clone()),
            newer: manifest.is_some_and(|m| is_newer(&index.bundle, &m.bundle)),
        },
        components: manifest
            .map_or(&[][..], |m| m.components.as_slice())
            .iter()
            .map(|c| {
                let release = index.component(&c.name).map(|i| i.version.clone());
                ComponentVersions {
                    name: c.name.clone(),
                    package: package_of(&c.name, &index.platform).map(str::to_owned),
                    installed: c.version.clone(),
                    release_newer: release.as_deref().is_some_and(|r| is_newer(r, &c.version)),
                    release,
                    upstream: None,
                    upstream_newer: false,
                }
            })
            .collect(),
        prefix: Some(prefix.to_owned()),
        action: UpdateAction::Checked,
        method: None,
        changes: Some(UpdateChanges::of(&plan)),
    };
    Ok(PreparedFromDisk {
        prepared: Prepared {
            index,
            manifest: new,
            plan,
            local: true,
            source: Box::new(disk),
            _work: Some(work),
        },
        report,
    })
}

/// O que a atualização por componentes deixou.
#[derive(Debug, Clone)]
pub struct AppliedUpdate {
    /// Os arquivos trocados e conferidos.
    pub applied: Applied,
    /// O que mudou, para o relatório.
    pub changes: UpdateChanges,
    /// A lista de programas instalados do Windows não pôde ser atualizada
    /// (a AURORA em si está atualizado); `None` quando foi, ou fora do
    /// Windows.
    pub registry_warning: Option<String>,
}

/// Faz a atualização por componentes preparada, e registra a versão nova na
/// lista de programas do Windows (`reg` é o `reg.exe`; o registro nunca
/// falha a atualização, só avisa).
pub fn apply(
    prepared: &Prepared,
    prefix: &Utf8Path,
    reg: Option<&Utf8Path>,
    on: &mut dyn FnMut(Event),
    cancel: &CancelToken,
) -> Result<AppliedUpdate> {
    let applied = super::apply_with(
        &*prepared.source,
        &prepared.plan,
        &prepared.manifest,
        prefix.as_std_path(),
        &mut *on,
        cancel,
    )?;
    let registry_warning = match installation::register_version(prefix, &prepared.plan.to, reg) {
        Ok(true) => None,
        Ok(false) => {
            tracing::info!("No entry of this installation in the list of installed apps");
            None
        }
        Err(error) => {
            tracing::info!("The list of installed apps: {error:#}");
            Some(format!(
                "The list of installed apps still shows the previous version ({error:#}); \
                 AURORA itself is updated"
            ))
        }
    };
    Ok(AppliedUpdate {
        applied,
        changes: prepared.changes(),
        registry_warning,
    })
}

/// Quem o instalador abre de novo quando termina (o assistente do Windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reopen {
    /// A AURORA IDE, que fechou para a atualização.
    Studio,
}

/// Para onde vai a saída de um processo filho que a biblioteca roda (o
/// `install` do instalador no Linux e no macOS, o `uninstall.sh`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildOutput {
    /// Herda o stdout de quem chamou (a CLI em texto).
    Inherit,
    /// O stdout do filho vai para o stderr de quem chamou (a CLI com
    /// `--json`, cujo stdout é só do objeto JSON).
    ToStderr,
    /// Cada linha vira [`Event::InstallerOutput`] (o Studio).
    Capture,
}

/// Como atualizar pelo instalador inteiro.
#[derive(Debug, Clone)]
pub struct Fallback<'a> {
    /// A versão nova.
    pub version: &'a str,
    /// Os componentes do bundle instalado (Windows: vão no `/COMPONENTS=`
    /// do assistente; no Linux e no macOS a lista vem do recibo).
    pub installed: &'a [String],
    /// Quem abrir de novo no fim.
    pub reopen: Option<Reopen>,
    /// A barra do download do instalador.
    pub bar: Bar,
    /// A saída do `install` filho (Linux, macOS).
    pub child_output: ChildOutput,
}

/// No Linux e no macOS, o instalador da versão nova, sem perguntar, com os
/// componentes, a pasta e o atalho do recibo: devolve `Updated`. No
/// Windows, o assistente de instalação da versão nova, que lembra a pasta,
/// com os componentes instalados marcados: devolve `WizardOpened`, e o
/// assistente roda depois que quem chamou sai.
pub fn fallback_installer(
    remote: &Remote,
    prefix: &Utf8Path,
    options: &Fallback<'_>,
    on: &mut dyn FnMut(Event),
    cancel: &CancelToken,
) -> Result<UpdateAction> {
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    if cfg!(windows) {
        windows(remote, options, on, cancel)
    } else {
        unix(remote, prefix, options, on, cancel)
    }
}

fn download_installer(
    remote: &Remote,
    version: &str,
    file: &str,
    dir: &Utf8Path,
    bar: Bar,
    on: &mut dyn FnMut(Event),
    cancel: &CancelToken,
) -> Result<Utf8PathBuf> {
    on(Event::FallbackDownload {
        file: file.to_owned(),
        url: remote.0.asset_url(version, file)?,
    });
    match bar {
        Bar::Curl => remote.release_file_with_bar(version, file, dir, cancel),
        Bar::Quiet => remote.release_file(
            version,
            file,
            dir,
            &mut |bytes| on(Event::Downloading { bytes }),
            cancel,
        ),
    }
}

fn unix(
    remote: &Remote,
    prefix: &Utf8Path,
    options: &Fallback<'_>,
    on: &mut dyn FnMut(Event),
    cancel: &CancelToken,
) -> Result<UpdateAction> {
    let version = options.version;
    let receipt = installation::receipt(prefix)?;
    // `--components` não aceita uma lista vazia, e sem ele o instalador
    // instalaria os recomendados.
    if receipt.components.is_empty() {
        fail!(
            "The installation in {prefix} has no components, and the installer does not reinstall \
             without any: download the installer of release v{version} and run ./install --prefix {prefix}"
        );
    }
    let work = remote.0.work_dir()?;
    let work_path = utf8_dir(&work)?;
    let file = format!("aurora-{version}-{}.tar.gz", receipt.platform);
    let archive = download_installer(remote, version, &file, &work_path, options.bar, on, cancel)?;
    let installer = extract(&archive, &work_path)?;
    let program = installer.join("install");
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }

    let mut command = std::process::Command::new(program.as_std_path());
    command
        .arg("--yes")
        .arg("--components")
        .arg(receipt.components.join(","))
        .arg("--prefix")
        .arg(prefix.as_std_path());
    match &receipt.link {
        Some(link) => command.arg("--link").arg(link),
        None => command.arg("--no-link"),
    };
    let status = match options.child_output {
        ChildOutput::Inherit => command
            .status()
            .with_context(|| format!("Running {program}"))?,
        ChildOutput::ToStderr => command
            .stdout(Stdio::from(std::io::stderr()))
            .status()
            .with_context(|| format!("Running {program}"))?,
        ChildOutput::Capture => {
            let mut child = command
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .with_context(|| format!("Running {program}"))?;
            let stdout = child.stdout.take().expect("piped stdout");
            let stderr = child.stderr.take().expect("piped stderr");
            let (tx, rx) = std::sync::mpsc::channel::<String>();
            for stream in [
                Box::new(stdout) as Box<dyn std::io::Read + Send>,
                Box::new(stderr),
            ] {
                let tx = tx.clone();
                std::thread::spawn(move || {
                    for line in std::io::BufReader::new(stream)
                        .lines()
                        .map_while(std::result::Result::ok)
                    {
                        let _ = tx.send(line);
                    }
                });
            }
            drop(tx);
            for line in rx {
                on(Event::InstallerOutput { line });
            }
            child.wait().with_context(|| format!("Running {program}"))?
        }
    };
    if !status.success() {
        return Err(Error::InstallerFailed {
            code: status
                .code()
                .map_or_else(|| "?".to_owned(), |c| c.to_string()),
        });
    }
    Ok(UpdateAction::Updated)
}

fn windows(
    remote: &Remote,
    options: &Fallback<'_>,
    on: &mut dyn FnMut(Event),
    cancel: &CancelToken,
) -> Result<UpdateAction> {
    let version = options.version;
    // Fica depois que quem chamou sai: o assistente roda dela.
    let work = remote.0.work_dir()?.keep();
    let work = Utf8PathBuf::from_path_buf(work)
        .map_err(|p| anyhow::anyhow!("Temporary folder path is not UTF-8: {}", p.display()))?;
    let file = format!("aurora-{version}-windows-x64-setup.exe");
    let setup = download_installer(remote, version, &file, &work, options.bar, on, cancel)?;
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let mut command = std::process::Command::new(setup.as_std_path());
    // O assistente só lembra a seleção feita nele, e refaz o `toolchain\`
    // com ela: sem a lista, o que o `aurora install` acrescentou sairia.
    match wizard_components(remote, version, options.installed, &work, cancel) {
        Ok(components) => {
            command
                .arg(format!("/TYPE={}", pack::CUSTOM_TYPE))
                .arg(format!("/COMPONENTS={}", components.join(",")));
            on(Event::WizardSelection {
                selected: true,
                warning: None,
            });
        }
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(error) => {
            tracing::warn!("Could not list the components of the new setup: {error:#}");
            on(Event::WizardSelection {
                selected: false,
                warning: Some(format!(
                    "Could not read the components of release v{version} ({error:#}); \
                     in the setup wizard, check that every app you use is selected"
                )),
            });
        }
    }
    if options.reopen == Some(Reopen::Studio) {
        command.arg("/REOPENIDE=1");
    }
    installation::open_wizard(&mut command).map_err(|error| Error::WizardFailed {
        reason: format!("Opening {setup}: {error}"),
    })?;
    Ok(UpdateAction::WizardOpened)
}

/// Os nomes que o `/COMPONENTS=` do assistente de `version` aceita para os
/// componentes `installed`, tirados do índice que a release publica
/// (`aurora-<versão>-windows-x64-index.json`), com a regra do `aurora-pack`
/// ([`pack::inno_component_names`]): `icarus\cocotb`, `surfer_aurora`. Um
/// componente que a versão nova não tem fica de fora; o `aurora` vai sempre.
fn wizard_components(
    remote: &Remote,
    version: &str,
    installed: &[String],
    work: &Utf8Path,
    cancel: &CancelToken,
) -> Result<Vec<String>> {
    // Sem a lista (o bundle não abriu), `/COMPONENTS=aurora` desmarcaria
    // todas as ferramentas: melhor deixar a seleção que o assistente lembra.
    if installed.is_empty() {
        fail!("the installed components are unknown");
    }
    let platform = Platform::current()
        .context("Na AURORA installer for this platform")?
        .as_str();
    let file = payload::release_asset(version, platform, payload::INDEX_FILE);
    let path = remote.release_file(version, &file, work, &mut |_| {}, cancel)?;
    let text = std::fs::read_to_string(&path).with_context(|| format!("Reading {path}"))?;
    let index = Index::parse(&text).with_context(|| format!("Invalid {file} in the release"))?;
    selected_components(&index, installed)
}

/// Os nomes do `/COMPONENTS=` do assistente para os componentes `installed`,
/// sobre um índice já lido (ver [`fallback_installer`]).
pub fn selected_components(index: &Index, installed: &[String]) -> Result<Vec<String>> {
    let pairs: Vec<(&str, &[String])> = index
        .components
        .iter()
        .map(|c| (c.name.as_str(), c.requires.as_slice()))
        .collect();
    let names = pack::inno_component_names(&pairs)?;
    let mut selected = vec!["aurora".to_owned()];
    selected.extend(
        installed
            .iter()
            .filter_map(|c| names.get(c.as_str()).cloned()),
    );
    Ok(selected)
}

/// Extrai o `.tar.gz` da release em `dir` e devolve a pasta extraída que tem
/// o programa `install` (o arquivo traz uma, `aurora-<versão>-<plataforma>/`).
fn extract(archive: &Utf8Path, dir: &Utf8Path) -> Result<Utf8PathBuf> {
    let file = std::fs::File::open(archive).with_context(|| format!("Opening {archive}"))?;
    tar::Archive::new(flate2::read::GzDecoder::new(file))
        .unpack(dir)
        .with_context(|| format!("Extracting {archive}"))?;
    for entry in dir
        .read_dir_utf8()
        .with_context(|| format!("Reading {dir}"))?
    {
        let path = entry
            .with_context(|| format!("Reading {dir}"))?
            .path()
            .to_owned();
        if path.join("install").is_file() {
            return Ok(path);
        }
    }
    fail!("{archive} has na AURORA installer (a folder with the install program)")
}

/// Desfaz a atualização que morreu no meio da troca dos arquivos (o Studio
/// fechado, um Ctrl+C duplo, falta de energia), antes de mexer na
/// instalação ([`super::recover`]). A instalação volta a ser a que era, mas
/// quem roda agora pode ser o `aurora` novo, que a troca já tinha posto no
/// lugar: a operação para aí ([`Error::InterruptedUpdateUndone`]), pedindo
/// para rodar de novo, com o `aurora` que voltou. Uma troca que não pôde ser
/// desfeita é [`Error::Other`]; quem só consulta pode seguir avisando.
pub fn recover_interrupted(prefix: &Utf8Path) -> Result<()> {
    match super::recover(prefix.as_std_path()) {
        Ok(Some(to)) => Err(Error::InterruptedUpdateUndone {
            to,
            prefix: prefix.to_owned(),
        }),
        Ok(None) => Ok(()),
        Err(error) => Err(from_anyhow(error)),
    }
}

/// O que quem chama decide e mostra no meio da atualização. Cada método tem
/// um padrão que segue em frente sem perguntar (o Studio e o `aurora serve`,
/// que confirmam antes de pedir); a CLI imprime e pergunta.
pub trait Hooks {
    /// As versões consultadas, antes de decidir (a tabela da CLI, o aviso
    /// diário).
    fn report(&mut self, report: &UpdateReport) {
        let _ = report;
    }
    /// A operação termina sem atualizar: `--check`, nada mais novo, ou um
    /// plano vazio.
    fn finished_without_update(&mut self, report: &UpdateReport) {
        let _ = report;
    }
    /// A atualização por componentes planejada, antes de perguntar.
    fn plan(&mut self, prepared: &Prepared) {
        let _ = prepared;
    }
    /// Pode trocar os arquivos do plano? `Err` para parar (a CLI devolve a
    /// resposta "não" como erro).
    fn confirm_plan(&mut self, prepared: &Prepared, prefix: &Utf8Path) -> Result<()> {
        let _ = (prepared, prefix);
        Ok(())
    }
    /// Pode rodar o instalador inteiro? Perguntado também quando o plano já
    /// tinha sido aceito e falhou: é outra operação, maior.
    fn confirm_installer(&mut self, aurora: &AuroraVersions, prefix: &Utf8Path) -> Result<()> {
        let _ = (aurora, prefix);
        Ok(())
    }
    /// A atualização por componentes terminou.
    fn applied(&mut self, prepared: &Prepared, applied: &AppliedUpdate, prefix: &Utf8Path) {
        let _ = (prepared, applied, prefix);
    }
}

/// Hooks que seguem em frente sem perguntar nem mostrar.
#[derive(Debug, Default, Clone, Copy)]
pub struct Silent;

impl Hooks for Silent {}

/// As opções do `aurora update`.
#[derive(Debug, Clone)]
pub struct RunOptions<'a> {
    /// Só comparar as versões (`--check`).
    pub check: bool,
    /// O instalador inteiro, sem tentar por componentes (`--full`).
    pub full: bool,
    /// Um instalador no disco, sem rede (`--from`).
    pub from: Option<&'a Utf8Path>,
    /// Quem o assistente do Windows abre de novo no fim.
    pub reopen: Option<Reopen>,
    /// A barra dos downloads do instalador inteiro.
    pub bar: Bar,
    /// A saída do `install` filho (Linux, macOS).
    pub child_output: ChildOutput,
    /// O `reg.exe` do Windows, para a lista de programas; `None`, o do
    /// `PATH`.
    pub reg: Option<&'a Utf8Path>,
}

/// O `aurora update` inteiro, na ordem da CLI: desfaz uma atualização
/// interrompida; com `from`, planeja do disco; senão consulta as versões
/// ([`check`]) e, com AURORA mais nova e uma instalação, tenta por
/// componentes e cai para o instalador inteiro quando não dá. `prefix` é a
/// instalação de onde o `aurora` roda, ou o erro de não haver uma: a consulta
/// funciona sem ela (`prefix: null` no relatório), atualizar não.
///
/// Cancelar vale até a troca dos arquivos: um download interrompido pelo
/// cancelamento é [`Error::Cancelled`], e não leva ao instalador inteiro.
#[allow(clippy::too_many_arguments)]
pub fn run(
    remote: &std::sync::Arc<Remote>,
    this_version: &str,
    manifest: Option<&BundleManifest>,
    prefix: Result<Utf8PathBuf>,
    options: &RunOptions<'_>,
    hooks: &mut dyn Hooks,
    on: &mut dyn FnMut(Event),
    cancel: &CancelToken,
) -> Result<UpdateReport> {
    if let Ok(prefix) = &prefix {
        match recover_interrupted(prefix) {
            Ok(()) => {}
            // Na busca, uma troca que não pôde ser desfeita (sem permissão
            // na pasta, por exemplo) só avisa.
            Err(Error::Other(error)) if options.check => on(Event::Warning {
                message: format!("{error:#}"),
            }),
            Err(error) => return Err(error),
        }
    }
    if let Some(path) = options.from {
        let prefix = prefix?;
        super::clean_leftovers(prefix.as_std_path());
        let PreparedFromDisk {
            prepared,
            mut report,
        } = prepare_from_disk(&remote.0, &prefix, path, this_version, manifest, cancel)?;
        hooks.plan(&prepared);
        if prepared.is_noop() {
            report.action = if options.check {
                UpdateAction::Checked
            } else {
                UpdateAction::UpToDate
            };
            report.changes = None;
            hooks.finished_without_update(&report);
            return Ok(report);
        }
        if options.check {
            hooks.finished_without_update(&report);
            return Ok(report);
        }
        hooks.confirm_plan(&prepared, &prefix)?;
        let applied = apply(&prepared, &prefix, options.reg, on, cancel)?;
        hooks.applied(&prepared, &applied, &prefix);
        report.changes = Some(applied.changes);
        report.action = UpdateAction::Updated;
        report.method = Some(UpdateMethod::Components);
        return Ok(report);
    }

    let sources: std::sync::Arc<dyn Sources> = remote.clone();
    let mut report = check(&*sources, this_version, manifest, cancel)?;
    report.prefix = prefix.as_ref().ok().cloned();
    hooks.report(&report);
    if options.check || !report.aurora.newer {
        report.action = if options.check {
            UpdateAction::Checked
        } else {
            UpdateAction::UpToDate
        };
        hooks.finished_without_update(&report);
        return Ok(report);
    }

    let prefix = prefix?;
    super::clean_leftovers(prefix.as_std_path());
    let version = report.aurora.latest.clone();
    if !options.full {
        let prepared = match prepare(&sources, &remote.0, &version, &prefix, cancel) {
            Ok(prepared) => prepared,
            Err(Error::Cancelled) => return Err(Error::Cancelled),
            Err(error) => {
                on(Event::Fallback {
                    reason: format!(
                        "Could not prepare the update by components ({error:#}); \
                         updating with the installer of AURORA {version}"
                    ),
                });
                None
            }
        };
        if let Some(prepared) = prepared {
            hooks.plan(&prepared);
            hooks.confirm_plan(&prepared, &prefix)?;
            match apply(&prepared, &prefix, options.reg, on, cancel) {
                Ok(applied) => {
                    hooks.applied(&prepared, &applied, &prefix);
                    report.action = UpdateAction::Updated;
                    report.method = Some(UpdateMethod::Components);
                    report.changes = Some(applied.changes);
                    return Ok(report);
                }
                Err(Error::Cancelled) => return Err(Error::Cancelled),
                Err(error) => on(Event::Fallback {
                    reason: format!(
                        "Could not update by components ({error:#}); the installation was \
                         left as it was. The installer of AURORA {version} can update instead"
                    ),
                }),
            }
        }
    }

    // O instalador inteiro pergunta antes, também quando a pessoa já tinha
    // aceitado a atualização por partes, que falhou: é outra operação, maior.
    hooks.confirm_installer(&report.aurora, &prefix)?;
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let installed: Vec<String> = manifest
        .map(|m| m.components.iter().map(|c| c.name.clone()).collect())
        .unwrap_or_default();
    report.method = Some(UpdateMethod::Installer);
    report.action = fallback_installer(
        remote,
        &prefix,
        &Fallback {
            version: &version,
            installed: &installed,
            reopen: options.reopen,
            bar: options.bar,
            child_output: options.child_output,
        },
        on,
        cancel,
    )?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use aurora_core::BundleComponent;
    use serde_json::json;

    use super::*;

    /// O `bundle/versions.json` de uma release de exemplo, no formato do
    /// versionado, com o que não interessa ao `update` encurtado.
    pub(crate) const VERSIONS: &str = r#"{
  "bundle": "2026.10.15",
  "packages": {
    "oss-cad-suite": {
      "version": "2026-10-15",
      "assets": {
        "linux-x64": { "url": "https://example.com/oss-cad-suite.tgz", "sha256": "00" }
      }
    },
    "yanc": {
      "version": "v5.7",
      "repository": "https://github.com/nipscernlab/yanc",
      "commit": "0000000000000000000000000000000000000000"
    },
    "surfer-aurora": {
      "version": "v0.7.0-nips.10",
      "repository": "https://gitlab.com/nips-cern/surfer-aurora.git",
      "commit": "0000000000000000000000000000000000000000"
    },
    "graphviz": {
      "version": "16.1.0",
      "platforms": ["windows-x64"],
      "assets": {
        "windows-x64": { "url": "https://example.com/graphviz.zip", "sha256": "00" }
      }
    },
    "msys": {
      "version": "ucrt64-v2",
      "platforms": ["windows-x64"],
      "assets": {
        "windows-x64": { "url": "https://example.com/lace-msys.zip", "sha256": "00" }
      }
    }
  }
}"#;

    /// Fontes sem rede. `None` faz a consulta falhar; `asked` guarda os
    /// pacotes consultados upstream.
    pub(crate) struct Fake {
        aurora: Option<&'static str>,
        versions: Option<&'static str>,
        upstream: &'static [(&'static str, &'static str)],
        asked: Mutex<Vec<String>>,
    }

    impl Fake {
        pub(crate) fn new(
            aurora: Option<&'static str>,
            versions: Option<&'static str>,
            upstream: &'static [(&'static str, &'static str)],
        ) -> Fake {
            Fake {
                aurora,
                versions,
                upstream,
                asked: Mutex::new(Vec::new()),
            }
        }

        fn asked(&self) -> Vec<String> {
            let mut asked = self.asked.lock().unwrap().clone();
            asked.sort();
            asked
        }
    }

    impl Sources for Fake {
        fn latest_aurora(&self, _cancel: &CancelToken) -> Result<String> {
            self.aurora
                .map(str::to_owned)
                .ok_or_else(|| Error::network("x", "sem rede"))
        }

        fn release_versions(&self, version: &str, _cancel: &CancelToken) -> Result<String> {
            assert_eq!(Some(version), self.aurora, "o bundle é o da última release");
            self.versions
                .map(str::to_owned)
                .ok_or_else(|| Error::network("x", "sem rede"))
        }

        fn upstream(&self, package: &str, _cancel: &CancelToken) -> Result<String> {
            self.asked.lock().unwrap().push(package.to_owned());
            self.upstream
                .iter()
                .find(|(p, _)| *p == package)
                .map(|(_, v)| (*v).to_owned())
                .ok_or_else(|| Error::network("x", "fora do ar"))
        }

        fn release_file(
            &self,
            _version: &str,
            _file: &str,
            _dir: &Utf8Path,
            _downloaded: &mut dyn FnMut(u64),
            _cancel: &CancelToken,
        ) -> Result<Utf8PathBuf> {
            Err(Error::network("x", "sem rede"))
        }
    }

    /// Um manifesto instalado, como o que `Toolchain::open` lê.
    pub(crate) fn manifest(platform: &str, components: &[(&str, &str)]) -> BundleManifest {
        let mut manifest: BundleManifest = serde_json::from_value(json!({
            "schema": 2,
            "bundle": "2026.09.29",
            "platform": platform,
        }))
        .unwrap();
        manifest.components = components
            .iter()
            .map(|(name, version)| {
                serde_json::from_value::<BundleComponent>(json!({
                    "name": name,
                    "version": version,
                    "dir": name,
                    "source": "https://example.com",
                }))
                .unwrap()
            })
            .collect();
        manifest
    }

    fn row<'a>(report: &'a UpdateReport, name: &str) -> &'a ComponentVersions {
        report
            .components
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("{name} não está no relatório"))
    }

    fn never() -> CancelToken {
        CancelToken::new()
    }

    #[test]
    fn components_come_from_the_packages_of_their_platform() {
        assert_eq!(package_of("icarus", "linux-x64"), Some("oss-cad-suite"));
        assert_eq!(
            package_of("verilator", "darwin-arm64"),
            Some("oss-cad-suite")
        );
        assert_eq!(package_of("icarus", "windows-x64"), Some("msys"));
        assert_eq!(package_of("verilator", "windows-x64"), Some("msys"));
        assert_eq!(package_of("ghdl", "windows-x64"), Some("msys"));
        assert_eq!(package_of("ghdl", "linux-x64"), Some("oss-cad-suite"));
        assert_eq!(package_of("ghdl", "darwin-arm64"), Some("oss-cad-suite"));
        assert_eq!(package_of("yosys", "windows-x64"), Some("oss-cad-suite"));
        assert_eq!(package_of("cocotb", "linux-x64"), Some("oss-cad-suite"));
        assert_eq!(package_of("cocotb", "darwin-arm64"), Some("oss-cad-suite"));
        assert_eq!(package_of("cocotb", "windows-x64"), Some("msys"));
        assert_eq!(package_of("yosys", "darwin-arm64"), Some("oss-cad-suite"));
        assert_eq!(package_of("graphviz", "linux-x64"), Some("oss-cad-suite"));
        assert_eq!(
            package_of("graphviz", "darwin-arm64"),
            Some("oss-cad-suite")
        );
        assert_eq!(package_of("graphviz", "windows-x64"), Some("graphviz"));
        assert_eq!(package_of("yanc", "linux-x64"), Some("yanc"));
        assert_eq!(
            package_of("surfer-aurora", "windows-x64"),
            Some("surfer-aurora")
        );
        assert_eq!(package_of("studio", "darwin-arm64"), Some("studio"));
        assert_eq!(
            package_of("aurora-learn", "windows-x64"),
            Some("aurora-learn")
        );
        assert_eq!(package_of("rtl-viewer", "linux-x64"), Some("rtl-viewer"));
        assert_eq!(package_of("outro", "linux-x64"), None);
        // Todo componente que a AURORA conhece sai de algum pacote.
        for name in component::ALL {
            for platform in ["linux-x64", "darwin-arm64", "windows-x64"] {
                assert!(package_of(name, platform).is_some(), "{name} em {platform}");
            }
        }
    }

    #[test]
    fn the_report_compares_installed_release_and_upstream() {
        let sources = Fake::new(
            Some("0.2.0"),
            Some(VERSIONS),
            &[
                ("oss-cad-suite", "2026-10-20"),
                ("yanc", "v5.7"),
                ("surfer-aurora", "v0.7.0-nips.11"),
                ("graphviz", "16.2.0"),
            ],
        );
        let installed = manifest(
            "linux-x64",
            &[
                ("yanc", "v5.6"),
                ("icarus", "2026-09-29"),
                ("graphviz", "2026-09-29"),
                ("surfer-aurora", "v0.7.0-nips.9"),
            ],
        );
        let report = check(&sources, "0.1.0", Some(&installed), &never()).unwrap();

        assert_eq!(report.aurora.installed, "0.1.0");
        assert_eq!(report.aurora.latest, "0.2.0");
        assert!(report.aurora.newer);
        assert_eq!(report.bundle.installed.as_deref(), Some("2026.09.29"));
        assert_eq!(report.bundle.latest.as_deref(), Some("2026.10.15"));
        assert!(report.bundle.newer);
        assert_eq!(report.action, UpdateAction::Checked);
        assert_eq!(report.prefix, None);
        let names: Vec<&str> = report.components.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["yanc", "icarus", "graphviz", "surfer-aurora"]);

        // A release traz um YANC mais novo, e é o último upstream.
        let yanc = row(&report, "yanc");
        assert_eq!(yanc.package.as_deref(), Some("yanc"));
        assert_eq!(yanc.release.as_deref(), Some("v5.7"));
        assert!(yanc.release_newer);
        assert_eq!(yanc.upstream.as_deref(), Some("v5.7"));
        assert!(!yanc.upstream_newer);

        // O upstream do OSS CAD Suite é mais novo que o do bundle da release.
        for name in ["icarus", "graphviz"] {
            let c = row(&report, name);
            assert_eq!(c.package.as_deref(), Some("oss-cad-suite"), "{name}");
            assert_eq!(c.installed, "2026-09-29");
            assert_eq!(c.release.as_deref(), Some("2026-10-15"));
            assert!(c.release_newer);
            assert_eq!(c.upstream.as_deref(), Some("2026-10-20"));
            assert!(c.upstream_newer, "{name}");
        }

        let surfer = row(&report, "surfer-aurora");
        assert_eq!(surfer.release.as_deref(), Some("v0.7.0-nips.10"));
        assert!(surfer.release_newer);
        assert!(surfer.upstream_newer);

        // Cada pacote é consultado uma vez, e só os dos componentes
        // instalados: no Linux, o Graphviz vem do OSS CAD Suite.
        assert_eq!(sources.asked(), ["oss-cad-suite", "surfer-aurora", "yanc"]);

        let news = upstream_news(&report.components);
        assert_eq!(
            news,
            [
                ("oss-cad-suite", "2026-10-20", vec!["icarus", "graphviz"]),
                ("surfer-aurora", "v0.7.0-nips.11", vec!["surfer-aurora"]),
            ]
        );
    }

    #[test]
    fn upstream_newer_is_relative_to_the_release_bundle() {
        // A última release é a instalada; o upstream do OSS CAD Suite andou,
        // o do YANC não, e o do Graphviz no Windows é o do bundle.
        let sources = Fake::new(
            Some("0.1.0"),
            Some(VERSIONS),
            &[
                ("oss-cad-suite", "2026-10-15"),
                ("yanc", "v5.7"),
                ("graphviz", "16.1.0"),
            ],
        );
        let installed = manifest(
            "windows-x64",
            &[
                ("yanc", "v5.7"),
                ("yosys", "2026-10-15"),
                ("graphviz", "16.1.0"),
            ],
        );
        let report = check(&sources, "0.1.0", Some(&installed), &never()).unwrap();
        assert!(!report.aurora.newer);
        for c in &report.components {
            assert!(!c.release_newer, "{}", c.name);
            assert!(!c.upstream_newer, "{}", c.name);
        }
        assert_eq!(
            row(&report, "graphviz").package.as_deref(),
            Some("graphviz")
        );
        assert!(upstream_news(&report.components).is_empty());

        // Upstream mais nova que a instalada, mas não que a da release: o
        // que falta chega atualizando a AURORA, e não é marcado.
        let sources = Fake::new(
            Some("0.2.0"),
            Some(VERSIONS),
            &[("oss-cad-suite", "2026-10-15")],
        );
        let installed = manifest("linux-x64", &[("yosys", "2026-09-29")]);
        let report = check(&sources, "0.1.0", Some(&installed), &never()).unwrap();
        let yosys = row(&report, "yosys");
        assert!(yosys.release_newer);
        assert!(!yosys.upstream_newer);
    }

    #[test]
    fn windows_simulators_follow_the_lace_toolchain() {
        let sources = Fake::new(Some("0.1.0"), Some(VERSIONS), &[("msys", "ucrt64-v3")]);
        let installed = manifest(
            "windows-x64",
            &[("icarus", "ucrt64-v1"), ("verilator", "ucrt64-v1")],
        );
        let report = check(&sources, "0.1.0", Some(&installed), &never()).unwrap();
        for name in ["icarus", "verilator"] {
            let c = row(&report, name);
            assert_eq!(c.package.as_deref(), Some("msys"));
            assert_eq!(c.release.as_deref(), Some("ucrt64-v2"));
            assert_eq!(c.upstream.as_deref(), Some("ucrt64-v3"));
            assert!(c.release_newer && c.upstream_newer, "{name}");
        }
    }

    #[test]
    fn a_failed_source_only_empties_its_column() {
        let sources = Fake::new(Some("0.1.0"), None, &[("yanc", "v5.7")]);
        let installed = manifest("linux-x64", &[("yanc", "v5.6"), ("icarus", "2026-09-29")]);
        let report = check(&sources, "0.1.0", Some(&installed), &never()).unwrap();
        assert_eq!(report.bundle.installed.as_deref(), Some("2026.09.29"));
        assert_eq!(report.bundle.latest, None);
        assert!(!report.bundle.newer);

        let icarus = row(&report, "icarus");
        assert_eq!(icarus.release, None);
        assert_eq!(icarus.upstream, None);
        assert!(!icarus.release_newer && !icarus.upstream_newer);

        // Sem o bundle da release e sem AURORA mais nova, a upstream se
        // compara com a instalada.
        let yanc = row(&report, "yanc");
        assert_eq!(yanc.release, None);
        assert_eq!(yanc.upstream.as_deref(), Some("v5.7"));
        assert!(yanc.upstream_newer);

        // Com AURORA mais nova, não dá para saber: o bundle dele pode já
        // trazer o YANC novo.
        let sources = Fake::new(Some("0.2.0"), None, &[("yanc", "v5.7")]);
        let newer = check(&sources, "0.1.0", Some(&installed), &never()).unwrap();
        assert!(newer.aurora.newer);
        assert!(!row(&newer, "yanc").upstream_newer);

        // `null` no JSON, não ausente.
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(value["bundle"]["latest"], serde_json::Value::Null);
        assert_eq!(value["components"][1]["upstream"], serde_json::Value::Null);
        assert_eq!(value["action"], "checked");
    }

    #[test]
    fn without_a_bundle_only_the_aurora_is_compared() {
        let sources = Fake::new(Some("0.2.0"), Some(VERSIONS), &[]);
        let report = check(&sources, "0.1.0", None, &never()).unwrap();
        assert!(report.aurora.newer);
        assert_eq!(report.bundle.installed, None);
        assert_eq!(report.bundle.latest.as_deref(), Some("2026.10.15"));
        assert!(!report.bundle.newer);
        assert!(report.components.is_empty());
        assert!(sources.asked().is_empty());
    }

    #[test]
    fn without_github_it_is_a_typed_error() {
        let sources = Fake::new(None, Some(VERSIONS), &[("yanc", "v5.7")]);
        let installed = manifest("linux-x64", &[("yanc", "v5.6")]);
        let error = check(&sources, "0.1.0", Some(&installed), &never()).unwrap_err();
        assert_eq!(error.code(), "no_latest_version");
        let message = error.to_string();
        assert!(message.contains("latest AURORA version"), "{message}");
        assert!(message.contains("GitHub"), "{message}");
    }

    #[test]
    fn a_cancelled_check_stops() {
        let sources = Fake::new(Some("0.2.0"), Some(VERSIONS), &[]);
        let cancel = CancelToken::new();
        cancel.cancel();
        assert!(matches!(
            check(&sources, "0.1.0", None, &cancel),
            Err(Error::Cancelled)
        ));
    }

    #[test]
    fn the_bundle_versions_file_of_this_tree_is_read() {
        let text = std::fs::read_to_string(
            Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bundle/versions.json"),
        )
        .unwrap();
        let bundle: ReleaseBundle = serde_json::from_str(&text).unwrap();
        for name in component::ALL {
            for platform in ["linux-x64", "darwin-arm64", "windows-x64"] {
                let package = package_of(name, platform).unwrap();
                assert!(
                    bundle.packages.contains_key(package),
                    "{package} ({name} em {platform}) não está em bundle/versions.json"
                );
            }
        }
    }

    #[test]
    fn the_wizard_gets_every_installed_component_by_its_inno_name() {
        let component = |name: &str, requires: &[&str]| payload::ComponentInfo {
            name: name.into(),
            label: name.into(),
            description: String::new(),
            recommended: true,
            requires: requires.iter().map(|r| (*r).to_owned()).collect(),
            version: "1".into(),
        };
        let index = Index {
            schema: payload::INDEX_SCHEMA,
            aurora_version: "0.3.0".into(),
            bundle: "2026.10.01".into(),
            platform: "windows-x64".into(),
            profiles: Vec::new(),
            components: vec![
                component("yanc", &[]),
                component("icarus", &[]),
                component("verilator", &[]),
                component("cocotb", &["icarus"]),
                component("yosys", &[]),
                component("graphviz", &["yosys"]),
                component("surfer-aurora", &[]),
                component("studio", &[]),
            ],
            chunks: Vec::new(),
        };
        // O Verilator e o cocotb entraram depois, pelo `aurora install`; um
        // componente que a versão nova não tem mais fica de fora.
        let installed: Vec<String> = [
            "yanc",
            "icarus",
            "verilator",
            "cocotb",
            "surfer-aurora",
            "antigo",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(
            selected_components(&index, &installed).unwrap(),
            [
                "aurora",
                "yanc",
                "icarus",
                "verilator",
                "icarus\\cocotb",
                "surfer_aurora"
            ]
        );
    }

    #[test]
    fn recover_on_a_clean_folder_is_fine_and_an_undone_update_is_typed() {
        let dir = tempfile::tempdir().unwrap();
        let prefix = Utf8PathBuf::from_path_buf(dir.path().to_owned()).unwrap();
        assert!(recover_interrupted(&prefix).is_ok());
    }
}
