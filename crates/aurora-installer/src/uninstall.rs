//! Remover uma instalação da AURORA, com o bundle (`aurora uninstall`).
//!
//! Quem sabe remover é o desinstalador que o instalador deixou na pasta, e
//! aqui só se chama:
//!
//! - Linux e macOS: `<instalação>/uninstall.sh`, gerado por este crate
//!   (`write_uninstaller`, em [`crate::install`]). Ele apaga `toolchain/`,
//!   `bin/aurora`, o `install.json`, o atalho e a pasta, se ficar vazia.
//! - Windows: o `unins000.exe` do Inno Setup, que também tira a pasta do
//!   `PATH`. Ele roda depois que quem chamou sai, porque o Windows não apaga
//!   um executável em uso.

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::Serialize;

// O Error só aparece no Windows, onde o desinstalador é o assistente.
#[cfg(windows)]
use crate::error::Error;
use crate::error::Result;
use crate::installation;
use crate::update::ChildOutput;

/// `aurora uninstall` (`docs/schema/uninstall.json`).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct UninstallReport {
    /// A pasta da instalação.
    #[schemars(with = "String")]
    pub prefix: Utf8PathBuf,
    /// `true`: a pasta e o atalho já saíram (Linux, macOS). `false`: o
    /// desinstalador do Windows foi aberto e termina depois que quem chamou
    /// sai.
    pub removed: bool,
}

/// Como remover.
#[derive(Debug, Clone, Copy)]
pub struct UninstallOptions {
    /// Sem perguntas no desinstalador do Windows (`/SILENT`): quem chamou já
    /// confirmou.
    pub silent: bool,
    /// Para onde vai o que o `uninstall.sh` escreve (Linux, macOS).
    pub script_output: ChildOutput,
}

/// Chama o desinstalador da instalação em `prefix`.
pub fn run(prefix: &Utf8Path, options: &UninstallOptions) -> Result<UninstallReport> {
    let removed = remove(prefix, options)?;
    Ok(UninstallReport {
        prefix: prefix.to_owned(),
        removed,
    })
}

#[cfg(not(windows))]
fn remove(prefix: &Utf8Path, options: &UninstallOptions) -> Result<bool> {
    use std::process::Stdio;
    let script = prefix.join(installation::UNINSTALL_SCRIPT);
    let stdout = match options.script_output {
        ChildOutput::Inherit => Stdio::inherit(),
        // Sem quem leia linha a linha, o stderr serve: o stdout de quem
        // chamou é do JSON.
        ChildOutput::ToStderr | ChildOutput::Capture => Stdio::from(std::io::stderr()),
    };
    let status = std::process::Command::new("/bin/sh")
        .arg(script.as_std_path())
        .stdout(stdout)
        .status()
        .with_context(|| format!("Running {script}"))?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "{script} failed; for a system installation (/opt/aurora), run: sudo aurora uninstall"
        )
        .into());
    }
    Ok(true)
}

#[cfg(windows)]
fn remove(prefix: &Utf8Path, options: &UninstallOptions) -> Result<bool> {
    let exe = installation::uninstaller_exe(prefix)
        .context("The AURORA uninstaller is not in the folder")?;
    let mut command = std::process::Command::new(exe.as_std_path());
    if options.silent {
        command.arg("/SILENT");
    }
    installation::open_wizard(&mut command).map_err(|error| Error::WizardFailed {
        reason: format!("Opening {exe}: {error}"),
    })?;
    Ok(false)
}

/// O arquivo de configuração de `aurora config`, que saiu na 0.2.0. Se ficou
/// de uma instalação antiga em `config_dir` (a pasta `aurora` da configuração
/// do usuário), sai também, com a pasta se ela ficar vazia.
pub fn remove_legacy_config(config_dir: &Utf8Path) {
    let _ = std::fs::remove_file(config_dir.join("config.json"));
    let _ = std::fs::remove_dir(config_dir);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_legacy_config_goes_with_its_empty_folder() {
        let dir = tempfile::tempdir().unwrap();
        let aurora = Utf8PathBuf::from_path_buf(dir.path().join("aurora")).unwrap();
        std::fs::create_dir_all(&aurora).unwrap();
        std::fs::write(aurora.join("config.json"), "{}").unwrap();
        remove_legacy_config(&aurora);
        assert!(!aurora.exists());
        // Uma pasta com outras coisas fica.
        std::fs::create_dir_all(&aurora).unwrap();
        std::fs::write(aurora.join("outro.json"), "{}").unwrap();
        remove_legacy_config(&aurora);
        assert!(aurora.join("outro.json").is_file());
    }

    #[test]
    fn a_folder_without_an_uninstaller_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let prefix = Utf8PathBuf::from_path_buf(dir.path().to_owned()).unwrap();
        let error = run(
            &prefix,
            &UninstallOptions {
                silent: true,
                script_output: ChildOutput::ToStderr,
            },
        )
        .unwrap_err();
        assert_eq!(error.code(), "install");
    }
}
