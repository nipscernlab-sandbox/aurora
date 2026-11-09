//! O controle de versão da AURORA: o git instalado na máquina e a conta do
//! GitHub, para os métodos `git.*` e `github.*` do `aurora serve` e as vistas
//! Controle de versão e Perfil da AURORA IDE.
//!
//! Fica fora do `aurora-core` pela regra de lá: o core só roda ferramentas do
//! bundle, e o git e o `curl` são os do sistema, como o `curl` do
//! `aurora-installer` e o Claude Code do `aurora-agent`. A regra do controle
//! de versão mora aqui, com teste.
//!
//! # O git do sistema
//!
//! Tudo roda o executável `git`, achado nas pastas que quem chama passa
//! ([`Git::locate`]; este crate não lê o `PATH`). Assim o `.gitignore`, os
//! hooks, o SSH e o gerenciador de credenciais funcionam como no terminal.
//! Nenhuma chamada espera alguém digitar: sem credencial, o git falha, e a
//! falha volta como [`Error::Authentication`].
//!
//! # O repositório inteiro
//!
//! O projeto pode estar dentro de um repositório maior. As operações valem
//! para o repositório inteiro, como no terminal; o [`status`] marca os
//! arquivos de dentro da pasta do projeto ([`ChangedFile::in_project`]) e os
//! põe primeiro. Os caminhos são relativos à pasta de topo, com `/`.
//!
//! # O commit
//!
//! O [`commit`] leva exatamente os arquivos escolhidos, no estado em que
//! estão no disco, como o GitHub Desktop: o que foi preparado no terminal e
//! não foi escolhido fica de fora e continua preparado.
//!
//! # A conta do GitHub
//!
//! O login é pelo fluxo de dispositivo do GitHub ([`github`]); o token vai
//! para o gerenciador de credenciais do git (`git credential approve`), e a
//! AURORA não guarda segredo nenhum. O push e o pull usam o mesmo
//! gerenciador, e o `git push` do terminal também passa a funcionar.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::print_stdout, clippy::print_stderr)]

mod error;
pub mod github;
mod parse;
mod repo;
mod run;
mod types;

use camino::{Utf8Path, Utf8PathBuf};

pub use error::{Error, Result};
pub use repo::{
    add_remote, branches, commit, commit_files, diff, discard, fetch, init, log, pull, push,
    read_file, set_identity, status, switch,
};
pub use types::*;

/// A menor versão do git aceita: a do `--pathspec-from-file` do `git add` e
/// do `git commit` (2.26), que leva a lista de arquivos pelo stdin em vez da
/// linha de comando, que tem limite de tamanho no Windows.
pub const MINIMUM_VERSION: [u32; 3] = [2, 26, 0];

/// O máximo de arquivos que o [`status`] lista.
pub const MAX_FILES: usize = 5000;

/// O maior arquivo que o [`read_file`] lê, em bytes.
pub const MAX_FILE_SIZE: u64 = 4 * 1024 * 1024;

/// O git da máquina.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Git {
    program: Utf8PathBuf,
    version: String,
    numbers: [u32; 3],
}

impl Git {
    /// O `git` da primeira pasta de `dirs` que o tiver, conferido pela
    /// versão.
    ///
    /// # Erros
    ///
    /// [`Error::GitMissing`] sem git, e [`Error::GitTooOld`] com um mais
    /// velho que [`MINIMUM_VERSION`].
    pub fn locate(dirs: &[Utf8PathBuf]) -> Result<Git> {
        let program = run::find_in(dirs, "git").ok_or(Error::GitMissing)?;
        Git::at(&program)
    }

    /// O git em `program`.
    ///
    /// # Erros
    ///
    /// Os de [`Git::locate`].
    pub fn at(program: &Utf8Path) -> Result<Git> {
        if !command_line_tools_present(program) {
            return Err(Error::GitMissing);
        }
        let cwd = program.parent().unwrap_or(program);
        let output = run::exec(
            program,
            cwd,
            &["--version".to_owned()],
            false,
            &run::Options::default(),
        )
        .map_err(|_| Error::GitMissing)?;
        let (version, numbers) = parse::version(&output.text()).ok_or(Error::GitMissing)?;
        if numbers < MINIMUM_VERSION {
            return Err(Error::GitTooOld {
                version,
                minimum: MINIMUM_VERSION.map(|n| n.to_string()).join("."),
            });
        }
        Ok(Git {
            program: program.to_owned(),
            version,
            numbers,
        })
    }

    /// A versão (`2.54.0.windows.1`).
    pub fn version(&self) -> &str {
        &self.version
    }

    /// O executável.
    pub fn program(&self) -> &Utf8Path {
        &self.program
    }

    /// A versão é pelo menos `minimum`.
    pub(crate) fn at_least(&self, minimum: [u32; 3]) -> bool {
        self.numbers >= minimum
    }

    /// Roda o git em `cwd`.
    pub(crate) fn run(
        &self,
        cwd: &Utf8Path,
        args: &[&str],
        options: &run::Options<'_>,
    ) -> Result<run::Output> {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        run::exec(&self.program, cwd, &args, true, options)
    }

    /// Roda e exige sucesso; o erro leva o subcomando e o stderr.
    pub(crate) fn run_ok(
        &self,
        cwd: &Utf8Path,
        args: &[&str],
        options: &run::Options<'_>,
    ) -> Result<run::Output> {
        let output = self.run(cwd, args, options)?;
        if output.success() {
            Ok(output)
        } else {
            Err(failure(args, &output.stderr))
        }
    }
}

/// O erro de um git que falhou: as recusas conhecidas viram variantes; o
/// resto, [`Error::Failed`].
pub(crate) fn failure(args: &[&str], stderr: &str) -> Error {
    if stderr.contains("detected dubious ownership") {
        let path = stderr
            .lines()
            .find_map(|l| {
                l.trim()
                    .strip_prefix("git config --global --add safe.directory ")
            })
            .map(|p| Utf8PathBuf::from(p.trim().trim_matches('\'')))
            .unwrap_or_default();
        return Error::DubiousOwnership { path };
    }
    if stderr.contains("Please tell me who you are")
        || stderr.contains("unable to auto-detect email")
    {
        return Error::IdentityMissing;
    }
    let owned: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    Error::Failed {
        command: run::subcommand(&owned),
        stderr: stderr.trim().to_owned(),
    }
}

/// No macOS, o `/usr/bin/git` sem as Command Line Tools não é o git: rodá-lo
/// abre o instalador da Apple. Lá, ele só vale com elas instaladas.
fn command_line_tools_present(program: &Utf8Path) -> bool {
    if !cfg!(target_os = "macos") || program != "/usr/bin/git" {
        return true;
    }
    let tool = Utf8Path::new("/usr/bin/xcode-select");
    run::exec(
        tool,
        Utf8Path::new("/"),
        &["-p".to_owned()],
        false,
        &run::Options::default(),
    )
    .is_ok_and(|o| o.success())
}
