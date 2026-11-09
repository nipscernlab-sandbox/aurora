//! O bundle de ferramentas instalado com a AURORA, e só ele.
//!
//! A AURORA não executa programa do `PATH` nem de um caminho que o usuário
//! configure. Toda ferramenta sai de um **bundle**: um diretório instalado com
//! a AURORA, com um manifesto que fixa a versão exata de cada componente, e um
//! executável num caminho fixo por plataforma.
//!
//! ```text
//! <instalação>/
//!   bin/aurora[.exe]
//!   toolchain/                 o bundle
//!     bundle.json              cabeçalho: formato, versão do bundle, plataforma
//!     components/<nome>.json   um por componente instalado: versão, origem, hashes
//!     yanc/                    bin/ SAPHO/ Macros/ Header/
//!     oss-cad-suite/           a parte do OSS CAD Suite de cada ferramenta instalada
//!     msys/                    só no Windows: o bloco MSYS2 UCRT64 do aurora-toolchain
//!                              (ucrt64/, usr/bin/) com o Icarus e o Verilator
//!     surfer-aurora/           o fork do Surfer da AURORA
//!     graphviz/                só no Windows, onde o OSS CAD Suite não traz o dot
//! ```
//!
//! O instalador deixa o usuário escolher os componentes. Um componente que
//! não foi instalado não tem arquivo em `components/`, e as ferramentas dele
//! dão [`AuroraError::ComponentMissing`].
//!
//! O Verilator compila o modelo com um compilador C++, chamado pelo `make`,
//! e o script dele roda pelo Perl ([`SystemCompiler`]). No Windows os três
//! vêm no bundle, com o Verilator, e a AURORA roda o `verilator_bin` sem o
//! script ([`Toolchain::invocation`]). No Linux e no macOS vêm do sistema, a
//! única exceção à regra do bundle, procurados em locais padrão fixos, nunca
//! no `PATH`.
//!
//! # Uso
//!
//! ```no_run
//! use aurora_core::{Tool, Toolchain};
//!
//! // O bundle ao lado do executável: <exe>/../toolchain ou <exe>/toolchain.
//! let exe = camino::Utf8PathBuf::try_from(std::env::current_exe()?)?;
//! let toolchain = Toolchain::locate(&exe)?;
//! println!("bundle {}", toolchain.manifest().bundle);
//! let yosys = toolchain.tool(Tool::Yosys)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::io::Read;

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AuroraError, Result};
use crate::paths;
use crate::process::{GUI_ENV, Invocation, make_path};

/// Versão do formato do manifesto que esta AURORA entende.
pub const BUNDLE_SCHEMA: u32 = 2;

/// Nome do cabeçalho do manifesto dentro do diretório do bundle.
pub const MANIFEST_FILE: &str = "bundle.json";

/// Diretório, dentro do bundle, com um `<nome>.json` por componente
/// instalado.
pub const COMPONENTS_DIR: &str = "components";

/// Os componentes que um bundle pode ter, pelo nome usado no manifesto.
///
/// No Linux e no macOS, Icarus, Verilator, cocotb, Yosys e o `dot` vêm da
/// mesma release do OSS CAD Suite e dividem o diretório `oss-cad-suite/`:
/// cada um traz a sua parte do pacote, e as bibliotecas em comum vêm com
/// qualquer um deles. No Windows, Icarus, Verilator e cocotb dividem do mesmo
/// jeito o `msys/`, o bloco MSYS2 do aurora-toolchain; o Yosys continua vindo
/// do OSS CAD Suite. A AURORA IDE (`studio`) vem pronta do repositório aurora-ide.
pub mod component {
    /// Os compiladores YANC e a biblioteca SAPHO.
    pub const YANC: &str = "yanc";
    /// Icarus Verilog (`iverilog`, `vvp`).
    pub const ICARUS: &str = "icarus";
    /// Verilator, com o Python do pacote que o `make` dele chama (e, no
    /// Windows, com o compilador, o `make` e o Perl).
    pub const VERILATOR: &str = "verilator";
    /// cocotb, para testbenches em Python, com o Python que o roda: no Linux
    /// e no macOS, o do OSS CAD Suite; no Windows, o do `msys/`, com a VPI do
    /// Verilator que o aurora-toolchain compila. A AURORA roda o cocotb com o
    /// Icarus ([`cocotb`](crate::cocotb)); com o Verilator, ainda não.
    pub const COCOTB: &str = "cocotb";
    /// Yosys.
    pub const YOSYS: &str = "yosys";
    /// O `dot` do Graphviz, para o esquemático: do OSS CAD Suite no Linux e no
    /// macOS, do pacote oficial do Graphviz no Windows.
    pub const GRAPHVIZ: &str = "graphviz";
    /// O fork do Surfer da AURORA.
    pub const SURFER_AURORA: &str = "surfer-aurora";
    /// O openFPGALoader, que grava as placas FPGA pelo cabo USB delas
    /// ([`crate::fpga`]), do OSS CAD Suite nas três plataformas.
    pub const OPENFPGALOADER: &str = "openfpgaloader";
    /// A AURORA IDE, o ambiente gráfico (o repositório aurora-ide). Não é
    /// ferramenta que a AURORA roda: vai no bundle para o instalador e o
    /// `aurora install` o oferecerem como os outros componentes, e o Studio
    /// instalado acha o bundle por estar dentro dele.
    pub const STUDIO: &str = "studio";
    /// As trilhas de exercícios do `aurora learn` (`aurora-learn/` no
    /// repositório). Só dados: o código que as corrige está na AURORA e no
    /// Studio, no crate `aurora-learn`.
    pub const AURORA_LEARN: &str = "aurora-learn";
    /// O RTL viewer fora do Studio (`packages/rtl-viewer/` no repositório):
    /// o modelo do HTML autocontido que o `aurora synth --open` preenche com
    /// o netlist ([`crate::rtl_viewer`]). Só dados.
    pub const RTL_VIEWER: &str = "rtl-viewer";

    /// Todos, na ordem em que a AURORA os lista.
    pub const ALL: &[&str] = &[
        YANC,
        ICARUS,
        VERILATOR,
        COCOTB,
        YOSYS,
        GRAPHVIZ,
        SURFER_AURORA,
        OPENFPGALOADER,
        STUDIO,
        AURORA_LEARN,
        RTL_VIEWER,
    ];

    /// O primeiro de `names` que não é um componente do bundle, com a
    /// mensagem que quem instala mostra antes de qualquer download:
    /// `Unknown app: nope (bundle apps: yanc, icarus, ...)`.
    pub fn unknown_message(names: &[String]) -> Option<String> {
        names
            .iter()
            .find(|name| !ALL.contains(&name.as_str()))
            .map(|name| format!("Unknown app: {name} (bundle apps: {})", ALL.join(", ")))
    }
}

/// Uma plataforma suportada, com o nome que o OSS CAD Suite usa nos pacotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Platform {
    /// Linux em x86-64.
    #[serde(rename = "linux-x64")]
    LinuxX64,
    /// macOS em Apple Silicon.
    #[serde(rename = "darwin-arm64")]
    MacosArm64,
    /// Windows 10 e 11 em x86-64.
    #[serde(rename = "windows-x64")]
    WindowsX64,
}

impl Platform {
    /// A plataforma em que a AURORA está rodando, se for suportada.
    pub fn current() -> Option<Platform> {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => Some(Platform::LinuxX64),
            ("macos", "aarch64") => Some(Platform::MacosArm64),
            ("windows", "x86_64") => Some(Platform::WindowsX64),
            _ => None,
        }
    }

    /// O nome no manifesto e nos pacotes (`linux-x64`, `darwin-arm64`,
    /// `windows-x64`).
    pub fn as_str(self) -> &'static str {
        match self {
            Platform::LinuxX64 => "linux-x64",
            Platform::MacosArm64 => "darwin-arm64",
            Platform::WindowsX64 => "windows-x64",
        }
    }

    /// O contrário de [`Platform::as_str`].
    pub fn from_name(name: &str) -> Option<Platform> {
        [
            Platform::LinuxX64,
            Platform::MacosArm64,
            Platform::WindowsX64,
        ]
        .into_iter()
        .find(|p| p.as_str() == name)
    }

    pub(crate) fn exe(self) -> &'static str {
        if self == Platform::WindowsX64 {
            ".exe"
        } else {
            ""
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// As ferramentas que a AURORA sabe orquestrar.
///
/// Em JSON e em [`Display`](fmt::Display), cada uma aparece pelo nome do
/// programa ([`Tool::binary_name`]).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Tool {
    /// Compilador C± do YANC: `.cmm` para `.asm`.
    Cmmcomp,
    /// Pré-assembler do YANC até a 6.x, que contava instruções e variáveis
    /// do `.asm`. Na 7.1 é a primeira passada do [`Tool::Asmcomp`]; fica aqui
    /// para os relatórios de builds antigos.
    Appcomp,
    /// Assembler do YANC: `.asm` para o Verilog do processador, as memórias
    /// `.mif` e o testbench.
    Asmcomp,
    /// Pré-processador do fluxo C do YANC até a 6.x (`#include`,
    /// `#define`). Na 7.1 faz parte do [`Tool::Cppcomp`]; fica aqui para os
    /// relatórios de builds antigos.
    Cpppp,
    /// Compilador C do YANC: `.cpp` para `.asm`, com o pré-processador.
    Cppcomp,
    /// Compilador do Icarus Verilog.
    Iverilog,
    /// Simulador do Icarus: executa o `.vvp` que o `iverilog` gera.
    Vvp,
    /// Verilator: compila o Verilog para um executável C++ e o roda como
    /// simulação, com o compilador de [`SystemCompiler`].
    Verilator,
    /// Yosys: sintetiza Verilog para netlist e gera o grafo do esquemático.
    Yosys,
    /// `dot` do Graphviz: desenha o grafo do esquemático em SVG.
    Dot,
    /// O visualizador de ondas surfer-aurora. Aplicação gráfica: abre e fica
    /// aberta.
    Surfer,
    /// O openFPGALoader: grava um bitstream (`.rbf`, `.svf`) na placa pelo
    /// cabo USB dela.
    #[serde(rename = "openfpgaloader")]
    OpenFpgaLoader,
    /// O Quartus Prime do sistema, que compila para as placas Intel
    /// ([`crate::fpga`]). Cada passo roda um programa de linha de comando
    /// dele (`quartus_map`, `quartus_fit`, `quartus_asm`, `quartus_sta`);
    /// não vem no bundle ([`crate::fpga::Quartus`]).
    Quartus,
    /// O Perl que executa o script `verilator`, parte do [`SystemCompiler`]:
    /// do sistema no Linux e no macOS, do bundle no Windows.
    Perl,
}

impl Tool {
    /// Nome do programa, sem extensão. O do Quartus é `quartus`: cada passo
    /// roda um programa dele, e o [`Toolchain::tool`] dá o `quartus_sh`.
    pub fn binary_name(self) -> &'static str {
        match self {
            Tool::Cmmcomp => "cmmcomp",
            Tool::Appcomp => "appcomp",
            Tool::Asmcomp => "asmcomp",
            Tool::Cpppp => "cpppp",
            Tool::Cppcomp => "cppcomp",
            Tool::Iverilog => "iverilog",
            Tool::Vvp => "vvp",
            Tool::Verilator => "verilator",
            Tool::Yosys => "yosys",
            Tool::Dot => "dot",
            Tool::Surfer => "surfer-aurora",
            Tool::OpenFpgaLoader => "openFPGALoader",
            Tool::Quartus => "quartus",
            Tool::Perl => "perl",
        }
    }

    /// Faz parte do YANC?
    pub fn is_yanc(self) -> bool {
        matches!(
            self,
            Tool::Cmmcomp | Tool::Appcomp | Tool::Asmcomp | Tool::Cpppp | Tool::Cppcomp
        )
    }

    /// Vem do sistema, e não de um componente do bundle? O Perl, parte do
    /// [`SystemCompiler`] (que no Windows vem do bundle: ver
    /// [`SystemCompiler::bundled`]), e o Quartus.
    pub fn is_system(self) -> bool {
        matches!(self, Tool::Perl | Tool::Quartus)
    }

    /// Todas as ferramentas, na ordem da declaração.
    pub fn all() -> &'static [Tool] {
        &[
            Tool::Cmmcomp,
            Tool::Appcomp,
            Tool::Asmcomp,
            Tool::Cpppp,
            Tool::Cppcomp,
            Tool::Iverilog,
            Tool::Vvp,
            Tool::Verilator,
            Tool::Yosys,
            Tool::Dot,
            Tool::Surfer,
            Tool::OpenFpgaLoader,
            Tool::Quartus,
            Tool::Perl,
        ]
    }

    /// O contrário de [`Tool::binary_name`].
    pub fn from_name(name: &str) -> Option<Tool> {
        Tool::all()
            .iter()
            .copied()
            .find(|t| t.binary_name() == name)
    }

    /// O componente do bundle de que a ferramenta faz parte (nomes em
    /// [`component`]). `None` para as do sistema ([`Tool::is_system`]).
    pub fn component(self) -> Option<&'static str> {
        Some(match self {
            Tool::Cmmcomp | Tool::Appcomp | Tool::Asmcomp | Tool::Cpppp | Tool::Cppcomp => {
                component::YANC
            }
            Tool::Iverilog | Tool::Vvp => component::ICARUS,
            Tool::Verilator => component::VERILATOR,
            Tool::Yosys => component::YOSYS,
            Tool::Dot => component::GRAPHVIZ,
            Tool::Surfer => component::SURFER_AURORA,
            Tool::OpenFpgaLoader => component::OPENFPGALOADER,
            Tool::Quartus | Tool::Perl => return None,
        })
    }

    /// Componente do bundle e caminho do executável dentro do diretório dele.
    /// `None` para as do sistema.
    fn location(self, platform: Platform) -> Option<(&'static str, String)> {
        let exe = platform.exe();
        let windows = platform == Platform::WindowsX64;
        let rel = match self {
            // No Windows, Icarus e Verilator vêm do bloco MSYS2 do
            // aurora-toolchain (`msys/`), com os programas em `ucrt64/bin`.
            Tool::Iverilog | Tool::Vvp if windows => {
                format!("ucrt64/bin/{}{exe}", self.binary_name())
            }
            // O `verilator` é um script Perl, sem extensão também no Windows.
            Tool::Verilator if windows => "ucrt64/bin/verilator".to_owned(),
            Tool::Verilator => "bin/verilator".to_owned(),
            Tool::Surfer => format!("surfer-aurora{exe}"),
            // No Windows o `dot` vem do Graphviz (`graphviz/bin/dot.exe`) e o
            // Yosys do OSS CAD Suite; nos outros, os dois do OSS CAD Suite. O
            // diretório vem do manifesto do componente.
            _ => format!("bin/{}{exe}", self.binary_name()),
        };
        Some((self.component()?, rel))
    }
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.binary_name())
    }
}

/// O manifesto do bundle, escrito pelo empacotamento (`scripts/bundle.py`)
/// e pelos instaladores, e só lido pela AURORA: o cabeçalho em `bundle.json` e
/// um arquivo por componente instalado em `components/`.
///
/// ```json
/// // bundle.json
/// { "schema": 2, "bundle": "2026.09.29", "platform": "linux-x64" }
///
/// // components/icarus.json
/// { "name": "icarus", "version": "2026-09-29", "dir": "oss-cad-suite",
///   "source": "https://github.com/YosysHQ/oss-cad-suite-build/releases/download/...",
///   "sha256": "eba89154...",
///   "files": { "oss-cad-suite/bin/iverilog": "<sha256>", "...": "..." } }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BundleManifest {
    /// Versão do formato ([`BUNDLE_SCHEMA`]).
    pub schema: u32,
    /// Identificador da versão do bundle.
    pub bundle: String,
    /// Plataforma para a qual o bundle foi montado ([`Platform::as_str`]).
    pub platform: String,
    /// Os componentes instalados, na ordem de [`component::ALL`] (um nome
    /// que esta AURORA não conhece vai para o fim). Lidos de `components/`,
    /// não do `bundle.json`.
    #[serde(default, skip_deserializing)]
    pub components: Vec<BundleComponent>,
}

/// Um componente instalado (`components/<nome>.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[non_exhaustive]
pub struct BundleComponent {
    /// Nome (ver o módulo [`component`]); é também o nome do arquivo.
    pub name: String,
    /// Versão exata: data da release do OSS CAD Suite, tag do YANC, do
    /// surfer-aurora, do Graphviz.
    pub version: String,
    /// Diretório do componente, relativo ao bundle. Componentes do OSS CAD
    /// Suite dividem o mesmo.
    #[schemars(with = "String")]
    pub dir: Utf8PathBuf,
    /// De onde veio: URL do pacote, ou `git+<repositório>@<commit>` quando o
    /// empacotamento compilou do fonte.
    pub source: String,
    /// SHA-256 do pacote de origem, quando foi baixado pronto.
    #[serde(default)]
    pub sha256: Option<String>,
    /// SHA-256 dos executáveis do componente que a AURORA roda, por caminho
    /// relativo ao bundle, conferido por [`Toolchain::verify`].
    #[serde(default)]
    #[schemars(with = "std::collections::BTreeMap<String, String>")]
    pub files: BTreeMap<Utf8PathBuf, String>,
}

/// O compilador C++, o `make` e o Perl que o Verilator usa.
///
/// No Windows vêm do bundle, no componente verilator (`msys/ucrt64/bin` e
/// `msys/usr/bin`), e [`SystemCompiler::bundled`] é verdadeiro. No Linux e no
/// macOS vêm do sistema, a exceção à regra do bundle, procurados em locais
/// fixos ([`SystemCompiler::detect`]), nunca no `PATH`: `/usr/bin` no Linux;
/// `/usr/bin` com as Command Line Tools do Xcode no macOS. Ou num diretório
/// declarado ([`SystemCompiler::in_dir`], e [`SystemCompiler::in_msys2`]
/// para um MSYS2 no Windows), que substitui o padrão.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct SystemCompiler {
    /// O Perl que executa o script `verilator`.
    #[schemars(with = "String")]
    pub perl: Utf8PathBuf,
    /// O `make` que o Verilator chama.
    #[schemars(with = "String")]
    pub make: Utf8PathBuf,
    /// O compilador C++ que o `make` chama.
    #[schemars(with = "String")]
    pub cxx: Utf8PathBuf,
    /// Os diretórios postos no `PATH` do Verilator para ele achar `make` e o
    /// compilador.
    #[schemars(with = "Vec<String>")]
    pub path: Vec<Utf8PathBuf>,
    /// Veio do bundle (Windows), e não do sistema.
    pub bundled: bool,
}

impl SystemCompiler {
    /// Procura nos locais padrão do sistema na plataforma atual. `None` se
    /// algum dos três programas faltar, e sempre no Windows, onde o
    /// compilador vem com o componente verilator do bundle.
    pub fn detect() -> Option<SystemCompiler> {
        match Platform::current()? {
            Platform::LinuxX64 => Self::in_dir("/usr/bin".into()),
            Platform::MacosArm64 => {
                // Os `/usr/bin/clang++` e `/usr/bin/make` existem sempre no
                // macOS, mas são atalhos que só funcionam com as Command Line
                // Tools ou o Xcode instalados.
                let installed = [
                    "/Library/Developer/CommandLineTools/usr/bin/clang++",
                    "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/clang++",
                ]
                .iter()
                .any(|p| Utf8Path::new(p).is_file());
                installed.then(|| Self::in_dir("/usr/bin".into())).flatten()
            }
            Platform::WindowsX64 => None,
        }
    }

    /// Compilador, `make` e Perl num único diretório (`/usr/bin`).
    pub fn in_dir(dir: Utf8PathBuf) -> Option<SystemCompiler> {
        let exe = Platform::current().map_or("", Platform::exe);
        let find = |names: &[&str]| {
            names
                .iter()
                .map(|n| dir.join(format!("{n}{exe}")))
                .find(|p| p.is_file())
        };
        Some(SystemCompiler {
            perl: find(&["perl"])?,
            make: find(&["make"])?,
            cxx: find(&["g++", "c++", "clang++"])?,
            path: vec![dir.clone()],
            bundled: false,
        })
    }

    /// Uma instalação do MSYS2 (Windows) declarada pelo usuário: `make` em
    /// `usr/bin`, compilador em `ucrt64/bin` ou `mingw64/bin`, Perl em
    /// `usr/bin` ou ao lado do compilador.
    pub fn in_msys2(root: &Utf8Path) -> Option<SystemCompiler> {
        let mut found = Self::msys_tree(root, &["ucrt64/bin", "mingw64/bin"])?;
        found.bundled = false;
        Some(found)
    }

    /// O do bloco MSYS2 do bundle (`msys/`, o diretório do componente
    /// verilator no Windows): tudo em `ucrt64/bin`, menos o `make` e o `sh`
    /// que ele roda, em `usr/bin`.
    fn in_bundle(msys: &Utf8Path) -> Option<SystemCompiler> {
        let mut found = Self::msys_tree(msys, &["ucrt64/bin"])?;
        found.bundled = true;
        Some(found)
    }

    fn msys_tree(root: &Utf8Path, compiler_dirs: &[&str]) -> Option<SystemCompiler> {
        let usr = root.join("usr/bin");
        let make = usr.join("make.exe");
        let cxx_dir = compiler_dirs
            .iter()
            .map(|d| root.join(d))
            .find(|d| d.join("g++.exe").is_file())?;
        let perl = [usr.join("perl.exe"), cxx_dir.join("perl.exe")]
            .into_iter()
            .find(|p| p.is_file())?;
        make.is_file().then(|| SystemCompiler {
            perl,
            make,
            cxx: cxx_dir.join("g++.exe"),
            path: vec![cxx_dir, usr],
            bundled: false,
        })
    }
}

/// Um executável do bundle cujo hash não bate com o manifesto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct FileMismatch {
    /// O componente que lista o arquivo.
    pub component: String,
    /// Caminho relativo ao bundle.
    #[schemars(with = "String")]
    pub path: Utf8PathBuf,
    /// O hash do manifesto.
    pub expected: String,
    /// O hash do arquivo, ou `None` se ele não existe.
    pub actual: Option<String>,
}

/// O bundle resolvido: manifesto lido, plataforma conferida, diretórios dos
/// componentes presentes. Construa com [`Toolchain::open`] ou
/// [`Toolchain::locate`]; é barata de clonar e pode ser compartilhada entre
/// threads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Toolchain {
    root: Utf8PathBuf,
    platform: Platform,
    manifest: BundleManifest,
    system: Option<SystemCompiler>,
    quartus: Option<crate::fpga::Quartus>,
}

impl Toolchain {
    /// Abre o bundle em `dir`: lê o `bundle.json` e os manifestos em
    /// `components/`, confere o formato e a plataforma, e que o diretório de
    /// cada componente existe. Acha o [`SystemCompiler`]: no Windows, o do
    /// componente verilator; nos outros, o dos locais padrão do sistema.
    ///
    /// Um bundle pode ter só parte dos componentes (os que o usuário escolheu
    /// no instalador, ou um bundle de desenvolvimento só com o YANC): a
    /// ferramenta de um componente ausente dá erro só quando for usada.
    ///
    /// # Erros
    ///
    /// - [`AuroraError::UnsupportedPlatform`] fora de Linux x64, macOS arm64 e
    ///   Windows x64;
    /// - [`AuroraError::InvalidBundle`] sem `bundle.json`, com JSON inválido,
    ///   formato desconhecido, plataforma diferente da atual, manifesto de
    ///   componente ilegível ou com outro nome, ou diretório de componente
    ///   ausente ou fora do bundle (por `..` ou por symlink).
    pub fn open(dir: impl AsRef<Utf8Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let platform = Platform::current().ok_or_else(|| AuroraError::UnsupportedPlatform {
            os: std::env::consts::OS.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
        })?;
        let invalid = |reason: String| AuroraError::InvalidBundle {
            path: dir.to_owned(),
            reason,
        };
        if !dir.is_dir() {
            return Err(invalid("Does not exist".into()));
        }
        let root = paths::canonicalize(dir)?;
        let manifest_path = root.join(MANIFEST_FILE);
        let text = std::fs::read_to_string(&manifest_path)
            .map_err(|e| invalid(format!("Could not read {MANIFEST_FILE}: {e}")))?;
        let mut manifest: BundleManifest =
            serde_json::from_str(&text).map_err(|e| invalid(format!("{MANIFEST_FILE}: {e}")))?;
        if manifest.schema != BUNDLE_SCHEMA {
            return Err(invalid(format!(
                "Manifest format {} is not supported; this AURORA reads format {BUNDLE_SCHEMA}",
                manifest.schema
            )));
        }
        if manifest.platform != platform.as_str() {
            return Err(invalid(format!(
                "Bundle built for {}, but AURORA is running on {platform}",
                manifest.platform
            )));
        }
        manifest.components = read_components(&root.join(COMPONENTS_DIR)).map_err(invalid)?;
        for c in &manifest.components {
            if !paths::is_contained(&c.dir) || !c.files.keys().all(|f| paths::is_contained(f)) {
                return Err(invalid(format!(
                    "Component {} points outside the bundle",
                    c.name
                )));
            }
            let dir = root.join(&c.dir);
            if !dir.is_dir() {
                return Err(invalid(format!(
                    "Missing directory for component {} ({})",
                    c.name, c.dir
                )));
            }
            // Um symlink não pode levar o componente para fora do bundle.
            if !paths::canonicalize(&dir)?.starts_with(&root) {
                return Err(invalid(format!(
                    "Component {} points outside the bundle",
                    c.name
                )));
            }
        }
        let system = match platform {
            Platform::WindowsX64 => manifest
                .components
                .iter()
                .find(|c| c.name == component::VERILATOR)
                .and_then(|c| SystemCompiler::in_bundle(&root.join(&c.dir))),
            _ => SystemCompiler::detect(),
        };
        let toolchain = Toolchain {
            root,
            platform,
            manifest,
            system,
            quartus: None,
        };
        tracing::debug!(root = %toolchain.root, bundle = %toolchain.manifest.bundle, "Bundle opened");
        Ok(toolchain)
    }

    /// Acha o bundle instalado com o executável da AURORA: `<dir do
    /// executável>/../toolchain` (instalação com `bin/`) ou `<dir do
    /// executável>/toolchain`, com os symlinks do caminho resolvidos.
    ///
    /// # Erros
    ///
    /// [`AuroraError::BundleNotFound`] se nenhum dos dois tiver um
    /// `bundle.json`; os de [`Toolchain::open`] se o achado for inválido.
    pub fn locate(executable: &Utf8Path) -> Result<Self> {
        // O executável pode ser um symlink (`~/.local/bin/aurora` apontando
        // para a instalação): o bundle fica ao lado do arquivo de verdade.
        let executable = paths::canonicalize(executable).unwrap_or_else(|_| executable.to_owned());
        let dir = executable.parent().unwrap_or(Utf8Path::new("."));
        let candidates: Vec<Utf8PathBuf> = [
            dir.parent().map(|p| p.join("toolchain")),
            Some(dir.join("toolchain")),
        ]
        .into_iter()
        .flatten()
        .collect();
        match candidates.iter().find(|c| c.join(MANIFEST_FILE).is_file()) {
            Some(found) => Self::open(found),
            None => Err(AuroraError::BundleNotFound {
                searched: candidates,
            }),
        }
    }

    /// Troca o [`SystemCompiler`] achado por um declarado.
    pub fn with_system_compiler(mut self, compiler: Option<SystemCompiler>) -> Self {
        self.system = compiler;
        self
    }

    /// O diretório do bundle, canonicalizado.
    pub fn root(&self) -> &Utf8Path {
        &self.root
    }

    /// A plataforma do bundle (a atual).
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// O manifesto.
    pub fn manifest(&self) -> &BundleManifest {
        &self.manifest
    }

    /// Um componente do manifesto, pelo nome.
    pub fn component(&self, name: &str) -> Option<&BundleComponent> {
        self.manifest.components.iter().find(|c| c.name == name)
    }

    /// O compilador do Verilator, se foi encontrado.
    pub fn system_compiler(&self) -> Option<&SystemCompiler> {
        self.system.as_ref()
    }

    /// Troca o Quartus Prime do sistema, que compila para as placas Intel
    /// ([`crate::fpga`]). O [`Toolchain::open`] não o procura: quem abre o
    /// bundle passa o de [`crate::fpga::Quartus::detect`] ou o de uma pasta
    /// declarada.
    pub fn with_quartus(mut self, quartus: Option<crate::fpga::Quartus>) -> Self {
        self.quartus = quartus;
        self
    }

    /// O Quartus Prime, se foi encontrado.
    pub fn quartus(&self) -> Option<&crate::fpga::Quartus> {
        self.quartus.as_ref()
    }

    /// O compilador do Verilator, ou o erro que diz por que falta: no Linux e
    /// no macOS, o do sistema; no Windows, o componente verilator ausente ou
    /// incompleto.
    fn compiler(&self) -> Result<&SystemCompiler> {
        if let Some(compiler) = &self.system {
            return Ok(compiler);
        }
        if self.platform != Platform::WindowsX64 {
            return Err(AuroraError::SystemCompilerMissing);
        }
        Err(AuroraError::ToolchainIncomplete {
            what: "g++, make and Perl for Verilator".into(),
            path: self.component_dir(component::VERILATOR)?,
        })
    }

    /// O cliente web do Surfer (`surfer-aurora/web/`: `index.html`,
    /// `surfer.js`, `surfer_bg.wasm`), o mesmo Surfer compilado para
    /// WebAssembly, que mostra uma onda numa página (a aba de onda da AURORA
    /// Studio). Vem no bundle junto do executável, da mesma versão: o
    /// `.surf.ron` do layout segue o formato dessa versão.
    ///
    /// # Erros
    ///
    /// - [`AuroraError::ComponentMissing`] sem o componente surfer-aurora;
    /// - [`AuroraError::ToolchainIncomplete`] se ele não traz o cliente web
    ///   (bundles anteriores a ele).
    pub fn surfer_web_dir(&self) -> Result<Utf8PathBuf> {
        let dir = self.component_dir(component::SURFER_AURORA)?;
        for file in ["web/index.html", "web/surfer.js", "web/surfer_bg.wasm"] {
            self.bundled_file(&dir, file, "Surfer web client")?;
        }
        Ok(dir.join("web"))
    }

    fn component_dir(&self, name: &str) -> Result<Utf8PathBuf> {
        self.component(name)
            .map(|c| self.root.join(&c.dir))
            .ok_or_else(|| AuroraError::ComponentMissing(name.to_owned()))
    }

    fn yanc_dir(&self, sub: &str) -> Result<Utf8PathBuf> {
        let dir = self.component_dir(component::YANC)?.join(sub);
        if dir.is_dir() {
            Ok(dir)
        } else {
            Err(AuroraError::ToolchainIncomplete {
                what: format!("YANC {sub} directory"),
                path: dir,
            })
        }
    }

    /// Biblioteca SAPHO (`yanc/SAPHO`): `-d` do `asmcomp`, `-y` do Icarus e
    /// do Verilator. Até a 5.6, o YANC copiava a mesma pasta também como
    /// `HDL/`, para a AURORA; a 6.0 só traz `SAPHO/`, o nome que a AURORA usa.
    pub fn hdl_dir(&self) -> Result<Utf8PathBuf> {
        self.yanc_dir("SAPHO")
    }

    /// A biblioteca SAPHO para o `-y` de simulação, verificação e síntese de
    /// um projeto com processadores (`with_processors`). Sem processadores,
    /// `None`: um projeto só de Verilog não usa a biblioteca, e um módulo do
    /// usuário com o nome de um módulo dela (`processor`, `core`) não colide.
    ///
    /// # Erros
    ///
    /// [`AuroraError::ComponentMissing`] com processadores e sem o YANC.
    pub fn sapho_library(&self, with_processors: bool) -> Result<Option<Utf8PathBuf>> {
        if !with_processors {
            return Ok(None);
        }
        if self.component(component::YANC).is_none() {
            return Err(AuroraError::ComponentMissing(component::YANC.into()));
        }
        self.hdl_dir().map(Some)
    }

    /// Macros do assembler (`yanc/Macros`).
    pub fn macros_dir(&self) -> Result<Utf8PathBuf> {
        self.yanc_dir("Macros")
    }

    /// Headers do fluxo C (`yanc/Header`).
    pub fn headers_dir(&self) -> Result<Utf8PathBuf> {
        self.yanc_dir("Header")
    }

    /// Caminho absoluto do executável de `tool`, conferido no disco agora.
    ///
    /// # Erros
    ///
    /// - [`AuroraError::ComponentMissing`] se o componente da ferramenta não
    ///   está neste bundle;
    /// - [`AuroraError::ToolchainIncomplete`] se está, mas o executável não
    ///   (para o Verilator, também sem o `verilator_bin` que o script Perl
    ///   chama: sem ele, o script iria buscá-lo no `PATH`);
    /// - [`AuroraError::InvalidBundle`] se o executável é um symlink que sai
    ///   do bundle;
    /// - para o [`Tool::Perl`] sem compilador, [`AuroraError::SystemCompilerMissing`]
    ///   no Linux e no macOS, e no Windows o erro do componente verilator;
    /// - para o [`Tool::Quartus`] (o `quartus_sh`), [`AuroraError::QuartusMissing`]
    ///   sem Quartus ([`Toolchain::with_quartus`]).
    pub fn tool(&self, tool: Tool) -> Result<Utf8PathBuf> {
        if tool == Tool::Quartus {
            return self
                .quartus
                .as_ref()
                .map(|q| q.program("quartus_sh"))
                .ok_or(AuroraError::QuartusMissing);
        }
        let Some((component, rel)) = tool.location(self.platform) else {
            return self.compiler().map(|c| c.perl.clone());
        };
        let dir = self.component_dir(component)?;
        let path = self.bundled_file(&dir, &rel, &format!("{tool} executable"))?;
        if tool == Tool::Verilator {
            // O script Perl roda o `verilator_bin` ao lado dele e, se não
            // achar, procura no PATH. Sem o do bundle, a AURORA recusa antes.
            let companion = format!("{rel}_bin{}", self.platform.exe());
            self.bundled_file(&dir, &companion, "verilator_bin for Verilator")?;
        }
        Ok(path)
    }

    /// `dir/rel` se for um arquivo que, resolvido, continua dentro do bundle.
    fn bundled_file(&self, dir: &Utf8Path, rel: &str, what: &str) -> Result<Utf8PathBuf> {
        let path = dir.join(rel);
        let incomplete = || AuroraError::ToolchainIncomplete {
            what: what.to_owned(),
            path: path.clone(),
        };
        if !path.is_file() {
            return Err(incomplete());
        }
        match paths::canonicalize(&path) {
            Ok(real) if real.starts_with(&self.root) => Ok(path),
            _ => Err(AuroraError::InvalidBundle {
                path: path.clone(),
                reason: format!("{what} points outside the bundle"),
            }),
        }
    }

    /// Quais ferramentas este bundle (e o compilador, para o Perl) tem agora.
    pub fn available(&self) -> Vec<Tool> {
        self.tools()
            .into_iter()
            .filter(|&t| self.tool(t).is_ok())
            .collect()
    }

    /// As ferramentas que a AURORA roda, na ordem de [`Tool::all`]: todas,
    /// menos o `appcomp` e o `cpppp`, que o YANC 7.1 não tem mais (o
    /// `asmcomp` faz as duas passadas e o `cppcomp` pré-processa). É a lista
    /// que um relatório do bundle mostra; uma ferramenta dela que
    /// [`Toolchain::tool`] não acha está faltando.
    pub fn tools(&self) -> Vec<Tool> {
        Tool::all()
            .iter()
            .copied()
            .filter(|t| !matches!(t, Tool::Appcomp | Tool::Cpppp))
            .collect()
    }

    /// Confere o SHA-256 de cada executável listado no manifesto. Lê os
    /// arquivos inteiros; é para instalação e diagnóstico, não para cada
    /// operação.
    pub fn verify(&self) -> Result<Vec<FileMismatch>> {
        let mut mismatches = Vec::new();
        for component in &self.manifest.components {
            for (rel, expected) in &component.files {
                let path = self.root.join(rel);
                let actual = match std::fs::File::open(&path) {
                    Ok(file) => {
                        Some(sha256(file).map_err(AuroraError::io("Verifying bundle", &path))?)
                    }
                    Err(_) => None,
                };
                if actual.as_deref() != Some(expected.as_str()) {
                    mismatches.push(FileMismatch {
                        component: component.name.clone(),
                        path: rel.clone(),
                        expected: expected.clone(),
                        actual,
                    });
                }
            }
        }
        Ok(mismatches)
    }

    /// Quantos executáveis os manifestos listam (o que [`Toolchain::verify`]
    /// confere).
    pub fn verified_files(&self) -> usize {
        self.manifest.components.iter().map(|c| c.files.len()).sum()
    }

    /// Invocação de `tool` com CWD `cwd`, com o interpretador na frente e o
    /// ambiente que a ferramenta precisa, e nada além.
    ///
    /// - Linux e macOS: os programas do OSS CAD Suite em `bin/` são lançadores
    ///   em bash que carregam o binário de `libexec/` com as bibliotecas do
    ///   próprio pacote (no Linux, pelo `ld-linux` do pacote). A AURORA roda o
    ///   lançador do bundle pelo `/bin/bash` do sistema, com `PATH` fixo em
    ///   `/usr/bin:/bin` para o `dirname` e o `readlink` que ele usa.
    /// - Windows: não há lançadores. A AURORA roda o `.exe` com `PATH` no
    ///   diretório dos programas e das DLLs do pacote (`ucrt64/bin` no
    ///   `msys/`; `bin;lib` no OSS CAD Suite, como o `environment.bat` faz) e
    ///   `%SystemRoot%\System32` no fim, para o `cmd.exe` do `system()`.
    /// - Verilator: no Linux e no macOS, o script Perl, rodado pelo Perl do
    ///   [`SystemCompiler`]; no Windows, o `verilator_bin.exe` que o script
    ///   rodaria, direto. Nos dois, com o `PATH` do `make` e do compilador
    ///   depois do diretório do Verilator.
    pub(crate) fn invocation(&self, tool: Tool, cwd: impl Into<Utf8PathBuf>) -> Result<Invocation> {
        let program = self.tool(tool)?;
        let windows = self.platform == Platform::WindowsX64;
        let invocation = match tool {
            Tool::Verilator => {
                let system = self.compiler()?;
                let dir = self.component_dir(component::VERILATOR)?;
                // O diretório do Verilator primeiro (o `verilator_bin`),
                // depois o make e o compilador.
                let mut path = program_dirs(&dir, tool, self.platform);
                path.extend(system.path.iter().cloned());
                path.extend(base_path(self.platform));
                // O `sh` do MSYS que o `make` roda avisa "could not find /tmp"
                // a cada chamada quando não há `tmp/` na raiz do MSYS, e o
                // bloco do aurora-toolchain só traz arquivos de pacotes. Sem
                // permissão de escrita (instalação do sistema), fica o aviso.
                if windows && system.bundled {
                    let _ = std::fs::create_dir_all(dir.join("tmp"));
                }
                let invocation = if windows {
                    // O script roda o `verilator_bin` por `system()`, que no
                    // Windows passa pelo `cmd.exe`: o escape de shell Unix
                    // que ele põe em cada argumento (`\ ` no espaço, `\ç`)
                    // chega inteiro ao `verilator_bin`, e um fonte numa pasta
                    // com espaço ou acento vira outro caminho ("Cannot find
                    // file containing module: 'espaco\sp\a.v'"). Direto, os
                    // argumentos chegam como a AURORA os montou. O resto do que
                    // o script faz não vale no Windows ou a AURORA já faz: o
                    // `ulimit -s` só existe num shell Unix, e o
                    // VERILATOR_ROOT, que ele deduz do próprio caminho, vai
                    // aqui no ambiente, pelo nome que o `make` aceita
                    // ([`make_path`]). E o `verilator_bin` anota o próprio
                    // nome, com o `.exe`, entre as entradas do modelo
                    // (`__verFiles.dat`), e o `--skip-identical` reconhece um
                    // modelo em dia: chamado pelo script sem o `.exe`, cada
                    // simulação gerava o C++ de novo e recompilava tudo
                    // (medido em 2026-10-07: 9 s, contra 0,4 s, num contador).
                    // Numa pasta com caractere fora do ASCII, ele nasce no
                    // nome curto dela ([`Invocation::ascii_cwd`]).
                    let bin =
                        program.with_file_name(format!("verilator_bin{}", self.platform.exe()));
                    let root = program
                        .parent()
                        .and_then(Utf8Path::parent)
                        .map(|prefix| prefix.join("share/verilator"))
                        .unwrap_or_else(|| dir.join("ucrt64/share/verilator"));
                    Invocation::new(bin, cwd)
                        .env("VERILATOR_ROOT", make_path(&root))
                        .ascii_cwd()
                } else {
                    Invocation::new(system.perl.clone(), cwd).path_arg(&program)
                };
                invocation.env("LC_ALL", "C").search_path(&path)
            }
            Tool::Iverilog | Tool::Vvp | Tool::Yosys | Tool::Dot | Tool::OpenFpgaLoader
                if windows =>
            {
                let dir = self.component_dir(tool.component().expect("Tool from the bundle"))?;
                let mut path = program_dirs(&dir, tool, self.platform);
                path.extend(base_path(self.platform));
                Invocation::new(program, cwd).search_path(&path)
            }
            Tool::Iverilog | Tool::Vvp | Tool::Yosys | Tool::Dot | Tool::OpenFpgaLoader => {
                Invocation::new(UNIX_SHELL, cwd)
                    .path_arg(&program)
                    .search_path(&unix_base_path())
            }
            Tool::Surfer => Invocation::new(program, cwd).inherit(GUI_ENV),
            Tool::Quartus => self
                .quartus
                .as_ref()
                .ok_or(AuroraError::QuartusMissing)?
                .invocation("quartus_sh", cwd)?,
            _ => Invocation::new(program, cwd),
        };
        Ok(invocation)
    }

    /// O Python do componente verilator, para o `make` dele (`PYTHON3=`): sem
    /// isso ele chamaria o `python3` do sistema. O do `msys/` no Windows, o
    /// do OSS CAD Suite nos outros.
    pub(crate) fn bundled_python(&self) -> Result<Utf8PathBuf> {
        let dir = self.component_dir(component::VERILATOR)?;
        let path = match self.platform {
            Platform::WindowsX64 => dir.join("ucrt64/bin/python.exe"),
            _ => dir.join("bin/tabbypy3"),
        };
        if path.is_file() {
            Ok(path)
        } else {
            Err(AuroraError::ToolchainIncomplete {
                what: "Python for Verilator".into(),
                path,
            })
        }
    }

    /// O Python do componente cocotb rodando `script` em `cwd`: o do `msys/`
    /// no Windows, com o `bin` dele no `PATH` (as DLLs do Python); o
    /// lançador `tabbypy3` do OSS CAD Suite nos outros, pelo `bash`, como as
    /// outras ferramentas do pacote.
    pub(crate) fn cocotb_python(&self, script: &Utf8Path, cwd: &Utf8Path) -> Result<Invocation> {
        let dir = self.component_dir(component::COCOTB)?;
        let (program, path) = match self.platform {
            Platform::WindowsX64 => (
                dir.join("ucrt64/bin/python.exe"),
                vec![dir.join("ucrt64/bin")],
            ),
            _ => (dir.join("bin/tabbypy3"), Vec::new()),
        };
        if !program.is_file() {
            return Err(AuroraError::ToolchainIncomplete {
                what: "Python for cocotb".into(),
                path: program,
            });
        }
        let mut path = path;
        path.extend(base_path(self.platform));
        let invocation = match self.platform {
            Platform::WindowsX64 => Invocation::new(program, cwd),
            _ => Invocation::new(UNIX_SHELL, cwd).path_arg(&program),
        };
        Ok(invocation.path_arg(script).search_path(&path))
    }

    /// Ambiente de fontes do `dot`. Sem configuração, o fontconfig do pacote
    /// lê a do sistema (`/etc/fonts`) e o desenho muda de máquina para
    /// máquina. O pacote de Linux traz fontes e um modelo de `fonts.conf`,
    /// que os lançadores gráficos dele (`xdot`, `gtkwave`) preenchem com o
    /// caminho do pacote; a AURORA faz o mesmo, gravando `fonts.conf` e o
    /// cache do fontconfig em `work`. Os pacotes de macOS e o Graphviz de
    /// Windows não trazem fontes: lá o `dot` usa as do sistema, e esta função
    /// não devolve nada.
    pub(crate) fn dot_fonts(&self, work: &Utf8Path) -> Result<Vec<(&'static str, String)>> {
        if self.platform == Platform::WindowsX64 {
            return Ok(Vec::new());
        }
        let oss = self.component_dir(component::GRAPHVIZ)?;
        let etc = oss.join("etc/fonts");
        let template = etc.join("fonts.conf.template");
        let Ok(text) = std::fs::read_to_string(&template) else {
            return Ok(Vec::new());
        };
        let cache = work.join("fontconfig-cache");
        let text = with_cachedir(
            &text.replace("TARGET_DIR", &xml_escaped(oss.as_str())),
            &xml_escaped(cache.as_str()),
        );
        let config = work.join("fonts.conf");
        std::fs::write(&config, text)
            .map_err(AuroraError::io("Writing font configuration", &config))?;
        Ok(vec![
            ("FONTCONFIG_FILE", config.into_string()),
            ("FONTCONFIG_PATH", etc.into_string()),
        ])
    }

    /// O `PATH` para rodar um programa que o Verilator gerou (o modelo
    /// compilado precisa das bibliotecas do compilador no Windows).
    pub(crate) fn verilated_model_path(&self) -> Vec<Utf8PathBuf> {
        self.system
            .as_ref()
            .map(|s| s.path.clone())
            .unwrap_or_default()
    }
}

/// O shell que roda os lançadores do OSS CAD Suite no Linux e no macOS.
/// Parte do sistema base nos dois.
const UNIX_SHELL: &str = "/bin/bash";

/// Os manifestos em `components/`, na ordem de [`component::ALL`]. Sem o
/// diretório, nenhum componente.
fn read_components(dir: &Utf8Path) -> std::result::Result<Vec<BundleComponent>, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("Could not read {COMPONENTS_DIR}/: {e}")),
    };
    let mut components = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|e| format!("Could not read {COMPONENTS_DIR}/: {e}"))?
            .path();
        let Some(path) = Utf8Path::from_path(&path) else {
            continue;
        };
        if path.extension() != Some("json") {
            continue;
        }
        let file = format!("{COMPONENTS_DIR}/{}", path.file_name().unwrap_or_default());
        let text = std::fs::read_to_string(path).map_err(|e| format!("{file}: {e}"))?;
        let c: BundleComponent = serde_json::from_str(&text).map_err(|e| format!("{file}: {e}"))?;
        if path.file_stem() != Some(c.name.as_str()) {
            return Err(format!("{file} describes component {}", c.name));
        }
        components.push(c);
    }
    let rank = |name: &str| {
        component::ALL
            .iter()
            .position(|c| *c == name)
            .unwrap_or(usize::MAX)
    };
    components.sort_by(|a, b| (rank(&a.name), &a.name).cmp(&(rank(&b.name), &b.name)));
    Ok(components)
}

/// Troca o `<cachedir>` do modelo do OSS CAD Suite (um caminho fixo em
/// `/tmp`) por `dir`, ou acrescenta um se o modelo não tiver.
fn with_cachedir(config: &str, dir: &str) -> String {
    let element = format!("<cachedir>{dir}</cachedir>");
    match (config.find("<cachedir>"), config.find("</cachedir>")) {
        (Some(start), Some(end)) if start < end => {
            format!(
                "{}{element}{}",
                &config[..start],
                &config[end + "</cachedir>".len()..]
            )
        }
        _ => config.replacen("</fontconfig>", &format!("\t{element}\n</fontconfig>"), 1),
    }
}

fn xml_escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// `PATH` dos lançadores: só os utilitários do sistema base (`dirname`,
/// `readlink`).
fn unix_base_path() -> Vec<Utf8PathBuf> {
    vec!["/usr/bin".into(), "/bin".into()]
}

/// O sistema base no fim do `PATH` das ferramentas do OSS CAD Suite: no
/// Linux e no macOS, `/usr/bin:/bin` (o `dirname` e o `readlink` dos
/// lançadores); no Windows, `%SystemRoot%\System32` (o `cmd.exe` que o
/// `system()` da biblioteca C usa).
pub(crate) fn base_path(platform: Platform) -> Vec<Utf8PathBuf> {
    match platform {
        Platform::WindowsX64 => {
            let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
            vec![Utf8PathBuf::from(root).join("System32")]
        }
        _ => unix_base_path(),
    }
}

/// Os diretórios dos programas de `tool` (e, no Windows, das DLLs deles)
/// dentro do diretório do componente: `ucrt64/bin` no `msys/` do Windows;
/// `bin` e `lib` no OSS CAD Suite do Windows e no Graphviz, como o
/// `environment.bat` monta; `bin` nos outros.
fn program_dirs(dir: &Utf8Path, tool: Tool, platform: Platform) -> Vec<Utf8PathBuf> {
    match (tool, platform) {
        (Tool::Iverilog | Tool::Vvp | Tool::Verilator, Platform::WindowsX64) => {
            vec![dir.join("ucrt64/bin")]
        }
        (_, Platform::WindowsX64) => vec![dir.join("bin"), dir.join("lib")],
        _ => vec![dir.join("bin")],
    }
}

pub(crate) fn sha256(mut reader: impl Read) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(components: &[(&str, &str)], platform: &str) -> (tempfile::TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dunce::canonicalize(dir.path()).unwrap()).unwrap();
        let header =
            serde_json::json!({ "schema": BUNDLE_SCHEMA, "bundle": "teste", "platform": platform });
        std::fs::write(root.join(MANIFEST_FILE), header.to_string()).unwrap();
        for (name, d) in components {
            if !d.contains("..") {
                std::fs::create_dir_all(root.join(d)).unwrap();
            }
            write_component(
                &root,
                serde_json::json!({ "name": name, "version": "1", "dir": d, "source": "teste" }),
            );
        }
        (dir, root)
    }

    fn write_component(root: &Utf8Path, manifest: serde_json::Value) {
        let dir = root.join(COMPONENTS_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        let name = manifest["name"].as_str().unwrap().to_owned();
        std::fs::write(dir.join(format!("{name}.json")), manifest.to_string()).unwrap();
    }

    fn current() -> &'static str {
        Platform::current()
            .expect("testes rodam numa plataforma suportada")
            .as_str()
    }

    #[test]
    fn tools_come_only_from_the_bundle() {
        let (_guard, root) = bundle(&[("yanc", "yanc")], current());
        let tc = Toolchain::open(&root).unwrap();
        // Componente presente, executável ausente.
        assert!(matches!(
            tc.tool(Tool::Cmmcomp),
            Err(AuroraError::ToolchainIncomplete { .. })
        ));
        // Componente não instalado.
        assert!(
            matches!(tc.tool(Tool::Yosys), Err(AuroraError::ComponentMissing(c)) if c == "yosys")
        );

        let exe = Platform::current().unwrap().exe();
        std::fs::create_dir_all(root.join("yanc/bin")).unwrap();
        std::fs::write(root.join(format!("yanc/bin/cmmcomp{exe}")), "").unwrap();
        assert_eq!(
            tc.tool(Tool::Cmmcomp).unwrap(),
            root.join(format!("yanc/bin/cmmcomp{exe}"))
        );
    }

    #[test]
    fn rejects_wrong_platform_schema_and_escapes() {
        let other = if current() == "linux-x64" {
            "windows-x64"
        } else {
            "linux-x64"
        };
        let (_g, root) = bundle(&[], other);
        assert!(
            matches!(Toolchain::open(&root), Err(AuroraError::InvalidBundle { reason, .. }) if reason.contains("built for"))
        );

        let (_g, root) = bundle(&[("yanc", "../fora")], current());
        assert!(
            matches!(Toolchain::open(&root), Err(AuroraError::InvalidBundle { reason, .. }) if reason.contains("outside the bundle"))
        );

        let (_g, root) = bundle(&[], current());
        std::fs::write(
            root.join(MANIFEST_FILE),
            r#"{"schema": 99, "bundle": "x", "platform": "x"}"#,
        )
        .unwrap();
        assert!(
            matches!(Toolchain::open(&root), Err(AuroraError::InvalidBundle { reason, .. }) if reason.contains("format 99"))
        );

        // O arquivo do componente precisa ter o nome dele.
        let (_g, root) = bundle(&[("yanc", "yanc")], current());
        std::fs::rename(
            root.join("components/yanc.json"),
            root.join("components/yosys.json"),
        )
        .unwrap();
        assert!(
            matches!(Toolchain::open(&root), Err(AuroraError::InvalidBundle { reason, .. }) if reason.contains("describes component yanc"))
        );
    }

    #[test]
    fn components_share_a_directory_and_come_in_a_fixed_order() {
        let (_g, root) = bundle(
            &[
                ("yosys", "oss-cad-suite"),
                ("icarus", "oss-cad-suite"),
                ("yanc", "yanc"),
            ],
            current(),
        );
        let tc = Toolchain::open(&root).unwrap();
        let names: Vec<_> = tc
            .manifest()
            .components
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(names, ["yanc", "icarus", "yosys"]);
        // O Verilator usa o mesmo diretório, mas não foi instalado.
        std::fs::create_dir_all(root.join("oss-cad-suite/bin")).unwrap();
        std::fs::write(root.join("oss-cad-suite/bin/verilator"), "").unwrap();
        assert!(
            matches!(tc.tool(Tool::Verilator), Err(AuroraError::ComponentMissing(c)) if c == "verilator")
        );
    }

    #[test]
    fn sapho_library_is_optional_without_processors() {
        let (_g, root) = bundle(&[("yosys", "oss-cad-suite")], current());
        let tc = Toolchain::open(&root).unwrap();
        assert_eq!(tc.sapho_library(false).unwrap(), None);
        assert!(
            matches!(tc.sapho_library(true), Err(AuroraError::ComponentMissing(c)) if c == "yanc")
        );
        let (_g, root) = bundle(&[("yanc", "yanc")], current());
        std::fs::create_dir_all(root.join("yanc/SAPHO")).unwrap();
        let tc = Toolchain::open(&root).unwrap();
        assert_eq!(tc.sapho_library(false).unwrap(), None);
        assert_eq!(
            tc.sapho_library(true).unwrap(),
            Some(root.join("yanc/SAPHO"))
        );
    }

    #[test]
    fn verilator_requires_its_bundled_verilator_bin() {
        let platform = Platform::current().unwrap();
        let (_, rel) = Tool::Verilator.location(platform).unwrap();
        let (_guard, root) = bundle(&[("verilator", "vl")], current());
        let tc = Toolchain::open(&root).unwrap();
        let script = root.join("vl").join(&rel);
        std::fs::create_dir_all(script.parent().unwrap()).unwrap();
        std::fs::write(&script, "").unwrap();
        assert!(
            matches!(tc.tool(Tool::Verilator), Err(AuroraError::ToolchainIncomplete { what, .. }) if what.contains("verilator_bin"))
        );
        std::fs::write(
            root.join("vl").join(format!("{rel}_bin{}", platform.exe())),
            "",
        )
        .unwrap();
        assert_eq!(tc.tool(Tool::Verilator).unwrap(), script);
    }

    #[cfg(windows)]
    #[test]
    fn on_windows_verilator_bin_runs_without_the_perl_script() {
        let (_guard, root) = bundle(&[("verilator", "msys")], current());
        for file in [
            "ucrt64/bin/verilator",
            "ucrt64/bin/verilator_bin.exe",
            "ucrt64/bin/g++.exe",
            "ucrt64/bin/perl.exe",
            "usr/bin/make.exe",
        ] {
            let path = root.join("msys").join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "").unwrap();
        }
        let tc = Toolchain::open(&root).unwrap();
        let invocation = tc.invocation(Tool::Verilator, &root).unwrap();
        assert_eq!(
            invocation.program,
            root.join("msys/ucrt64/bin/verilator_bin.exe")
        );
        // Nada do script: nem o próprio script, nem o `--no-unlimited-stack`.
        assert!(invocation.args.is_empty(), "{:?}", invocation.args);
        let env = |key: &str| {
            invocation
                .env
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
        };
        let verilator_root = env("VERILATOR_ROOT").expect("VERILATOR_ROOT");
        assert!(
            verilator_root.ends_with("/msys/ucrt64/share/verilator")
                && !verilator_root.contains('\\'),
            "{verilator_root}"
        );
        assert_eq!(env("LC_ALL").as_deref(), Some("C"));
        assert_eq!(env("VERILATOR_BIN"), None);
    }

    #[test]
    fn icarus_and_verilator_come_from_msys_only_on_windows() {
        let at = |tool: Tool, platform| tool.location(platform).unwrap();
        let windows = Platform::WindowsX64;
        assert_eq!(
            at(Tool::Iverilog, windows),
            ("icarus", "ucrt64/bin/iverilog.exe".into())
        );
        assert_eq!(
            at(Tool::Vvp, windows),
            ("icarus", "ucrt64/bin/vvp.exe".into())
        );
        assert_eq!(
            at(Tool::Verilator, windows),
            ("verilator", "ucrt64/bin/verilator".into())
        );
        assert_eq!(at(Tool::Yosys, windows), ("yosys", "bin/yosys.exe".into()));
        assert_eq!(at(Tool::Dot, windows), ("graphviz", "bin/dot.exe".into()));
        assert_eq!(
            at(Tool::OpenFpgaLoader, windows),
            ("openfpgaloader", "bin/openFPGALoader.exe".into())
        );
        for unix in [Platform::LinuxX64, Platform::MacosArm64] {
            assert_eq!(at(Tool::Iverilog, unix), ("icarus", "bin/iverilog".into()));
            assert_eq!(
                at(Tool::Verilator, unix),
                ("verilator", "bin/verilator".into())
            );
            assert_eq!(at(Tool::Yosys, unix), ("yosys", "bin/yosys".into()));
            assert_eq!(
                at(Tool::OpenFpgaLoader, unix),
                ("openfpgaloader", "bin/openFPGALoader".into())
            );
        }
        assert_eq!(Tool::Perl.location(windows), None);
        assert_eq!(Tool::Quartus.location(windows), None);
        assert!(Tool::Quartus.is_system() && Tool::Quartus.component().is_none());

        let dir = Utf8Path::new("/b/msys");
        assert_eq!(
            program_dirs(dir, Tool::Verilator, windows),
            [dir.join("ucrt64/bin")]
        );
        assert_eq!(
            program_dirs(Utf8Path::new("/b/oss"), Tool::Yosys, windows),
            [Utf8Path::new("/b/oss/bin"), Utf8Path::new("/b/oss/lib")]
        );
        assert_eq!(
            program_dirs(Utf8Path::new("/b/oss"), Tool::Iverilog, Platform::LinuxX64),
            [Utf8Path::new("/b/oss/bin")]
        );
    }

    #[test]
    fn compiler_of_the_bundled_msys_and_of_a_declared_one() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_owned()).unwrap();
        for f in [
            "ucrt64/bin/g++.exe",
            "ucrt64/bin/perl.exe",
            "usr/bin/make.exe",
        ] {
            std::fs::create_dir_all(root.join(f).parent().unwrap()).unwrap();
            std::fs::write(root.join(f), "").unwrap();
        }
        // O do bundle: tudo em ucrt64/bin, menos o make.
        let bundled = SystemCompiler::in_bundle(&root).unwrap();
        assert!(bundled.bundled);
        assert_eq!(bundled.perl, root.join("ucrt64/bin/perl.exe"));
        assert_eq!(bundled.make, root.join("usr/bin/make.exe"));
        assert_eq!(bundled.cxx, root.join("ucrt64/bin/g++.exe"));
        assert_eq!(
            bundled.path,
            [root.join("ucrt64/bin"), root.join("usr/bin")]
        );
        // Um MSYS2 declarado pelo usuário prefere o Perl de usr/bin.
        assert!(!SystemCompiler::in_msys2(&root).unwrap().bundled);
        std::fs::write(root.join("usr/bin/perl.exe"), "").unwrap();
        assert_eq!(
            SystemCompiler::in_msys2(&root).unwrap().perl,
            root.join("usr/bin/perl.exe")
        );
        // Sem make, nenhum dos dois.
        std::fs::remove_file(root.join("usr/bin/make.exe")).unwrap();
        assert_eq!(SystemCompiler::in_bundle(&root), None);
        assert_eq!(SystemCompiler::in_msys2(&root), None);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_leave_the_bundle() {
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(outside.path().join("bin")).unwrap();
        std::fs::write(outside.path().join("bin/cmmcomp"), "").unwrap();

        // O diretório do componente.
        let (_g, root) = bundle(&[], current());
        std::os::unix::fs::symlink(outside.path(), root.join("yanc")).unwrap();
        write_component(
            &root,
            serde_json::json!({ "name": "yanc", "version": "1", "dir": "yanc", "source": "teste" }),
        );
        assert!(
            matches!(Toolchain::open(&root), Err(AuroraError::InvalidBundle { reason, .. }) if reason.contains("outside the bundle"))
        );

        // O executável.
        let (_g, root) = bundle(&[("yanc", "yanc")], current());
        std::fs::create_dir_all(root.join("yanc/bin")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("bin/cmmcomp"),
            root.join("yanc/bin/cmmcomp"),
        )
        .unwrap();
        let tc = Toolchain::open(&root).unwrap();
        assert!(
            matches!(tc.tool(Tool::Cmmcomp), Err(AuroraError::InvalidBundle { reason, .. }) if reason.contains("outside the bundle"))
        );
    }

    #[test]
    fn locate_finds_bundle_next_to_executable() {
        let dir = tempfile::tempdir().unwrap();
        let install = Utf8PathBuf::from_path_buf(dunce::canonicalize(dir.path()).unwrap()).unwrap();
        let toolchain_dir = install.join("toolchain");
        std::fs::create_dir_all(&toolchain_dir).unwrap();
        std::fs::create_dir_all(install.join("bin")).unwrap();
        let manifest = serde_json::json!({
            "schema": BUNDLE_SCHEMA, "bundle": "teste", "platform": current(),
        });
        std::fs::write(toolchain_dir.join(MANIFEST_FILE), manifest.to_string()).unwrap();

        let tc = Toolchain::locate(&install.join("bin").join("aurora")).unwrap();
        assert_eq!(tc.root(), toolchain_dir);

        // Por um symlink em outro diretório (`~/.local/bin/aurora`).
        #[cfg(unix)]
        {
            let exe = install.join("bin/aurora");
            std::fs::write(&exe, "").unwrap();
            let links = install.join("links");
            std::fs::create_dir_all(&links).unwrap();
            std::os::unix::fs::symlink(&exe, links.join("aurora")).unwrap();
            let tc = Toolchain::locate(&links.join("aurora")).unwrap();
            assert_eq!(tc.root(), toolchain_dir);
        }
        assert!(matches!(
            Toolchain::locate(Utf8Path::new("/nao/existe/bin/aurora")),
            Err(AuroraError::BundleNotFound { .. })
        ));
    }

    #[test]
    fn verify_reports_changed_and_missing_files() {
        let (_guard, root) = bundle(&[], current());
        std::fs::create_dir_all(root.join("yanc")).unwrap();
        std::fs::write(root.join("yanc/a"), "abc").unwrap();
        write_component(
            &root,
            serde_json::json!({ "name": "yanc", "version": "1", "dir": "yanc", "source": "teste",
                "files": { "yanc/a": sha256("abc".as_bytes()).unwrap(), "yanc/b": "00" } }),
        );
        let tc = Toolchain::open(&root).unwrap();
        assert_eq!(tc.verified_files(), 2);
        let bad = tc.verify().unwrap();
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].component, "yanc");
        assert_eq!(bad[0].path, "yanc/b");
        assert_eq!(bad[0].actual, None);
        assert_eq!(
            sha256("abc".as_bytes()).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn fontconfig_cache_goes_to_the_work_dir() {
        let template = "<fontconfig>\n\t<dir>TARGET_DIR/share/fonts</dir>\n\t<cachedir>/tmp/yosyshq/cache/fontconfig</cachedir>\n</fontconfig>\n";
        let out = with_cachedir(template, "/p/x");
        assert!(out.contains("<cachedir>/p/x</cachedir>"));
        assert!(!out.contains("/tmp/yosyshq"));
        let out = with_cachedir("<fontconfig>\n</fontconfig>\n", "/p/x");
        assert_eq!(
            out,
            "<fontconfig>\n\t<cachedir>/p/x</cachedir>\n</fontconfig>\n"
        );
        assert_eq!(xml_escaped("a&b<c>"), "a&amp;b&lt;c&gt;");
    }

    #[test]
    fn tool_names_round_trip() {
        for &tool in Tool::all() {
            assert_eq!(Tool::from_name(tool.binary_name()), Some(tool));
        }
        for p in [
            Platform::LinuxX64,
            Platform::MacosArm64,
            Platform::WindowsX64,
        ] {
            assert_eq!(Platform::from_name(p.as_str()), Some(p));
        }
    }
}
