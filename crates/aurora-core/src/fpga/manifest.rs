//! O registro da compilação para a placa (`aurora-build.json`, na pasta da
//! placa): o hash do `.sof` e de tudo o que entrou nele. A gravação
//! ([`super::program()`]) só aceita um `.sof` que o registro descreve, e
//! descreve o projeto como ele está agora.
//!
//! Sem isso a placa podia receber um design diferente do que está na tela: o
//! `.sof` de antes de uma mudança no Verilog, no programa de um processador
//! (as memórias `.mif`) ou nas ligações, ou o de uma versão da AURORA com
//! outros pinos para a mesma placa. A comparação é pelo conteúdo, e não pela
//! data: um build que regrava os mesmos arquivos não invalida a compilação.
//!
//! Fora do registro ficam os arquivos que um `$readmemh` do usuário lê: o
//! AURORA não sabe quais são.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use super::board::Board;
use super::config::FpgaConfig;
use crate::error::{AuroraError, Result};
use crate::project::Project;
use crate::toolchain::Toolchain;

/// O nome do registro, na pasta da placa.
pub(crate) const MANIFEST: &str = "aurora-build.json";

/// O que a compilação gravou.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Manifest {
    /// A placa (o `id`).
    pub board: String,
    /// O modelo da FPGA.
    pub device: String,
    /// O topo do projeto.
    pub top: String,
    /// O SHA-256 do `.sof`.
    pub bitstream: String,
    /// O SHA-256 de cada entrada: os fontes, as memórias dos processadores,
    /// o `fpga.json` e a definição da placa (`board:<id>`).
    pub inputs: BTreeMap<String, String>,
}

/// As entradas da compilação para `board` e o hash de cada uma, lidas agora.
/// Um arquivo que sumiu entra com hash vazio: some do projeto, muda o
/// registro.
pub(crate) fn inputs(
    toolchain: &Toolchain,
    project: &Project,
    board: &Board,
) -> Result<BTreeMap<String, String>> {
    let mut files: Vec<Utf8PathBuf> = crate::synth::board_sources(toolchain, project)?;
    for processor in project.processors() {
        files.extend(
            processor
                .files_read_by_verilog()
                .into_iter()
                .filter(|p| p.extension() == Some("mif") && p.is_file()),
        );
    }
    files.push(FpgaConfig::path(project.root()));
    let mut out = BTreeMap::new();
    for file in files {
        out.insert(file.to_string(), hash_file(&file).unwrap_or_default());
    }
    let definition = serde_json::to_string(board).unwrap_or_default();
    out.insert(
        format!("board:{}", board.id),
        crate::toolchain::sha256(definition.as_bytes()).unwrap_or_default(),
    );
    Ok(out)
}

/// O SHA-256 de um arquivo.
pub(crate) fn hash_file(path: &Utf8Path) -> Result<String> {
    let file = std::fs::File::open(path).map_err(AuroraError::io("Reading", path))?;
    crate::toolchain::sha256(file).map_err(AuroraError::io("Reading", path))
}

/// Grava o registro na pasta da placa `dir`.
pub(crate) fn write(dir: &Utf8Path, manifest: &Manifest) -> Result<()> {
    let path = dir.join(MANIFEST);
    let text = serde_json::to_string_pretty(manifest).unwrap_or_default();
    std::fs::write(&path, text).map_err(AuroraError::io("Writing", &path))
}

/// Apaga o registro: uma compilação que começou não vale mais a anterior,
/// termine como terminar.
pub(crate) fn remove(dir: &Utf8Path) {
    let _ = std::fs::remove_file(dir.join(MANIFEST));
}

/// O registro da pasta da placa `dir`, se existir e for legível.
pub(crate) fn read(dir: &Utf8Path) -> Option<Manifest> {
    let text = std::fs::read_to_string(dir.join(MANIFEST)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Por que o `.sof` não descreve o projeto de agora, uma razão por item;
/// vazio, ele descreve. `now` são as entradas lidas agora ([`inputs`]).
pub(crate) fn differences(
    manifest: &Manifest,
    board: &Board,
    bitstream_hash: &str,
    now: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if manifest.board != board.id || manifest.device != board.device.part {
        reasons.push(format!(
            "it was built for {} ({}), and fpga.json now says {} ({})",
            manifest.board, manifest.device, board.id, board.device.part
        ));
    }
    if manifest.bitstream != bitstream_hash {
        reasons.push("the .sof is not the one the last build wrote".into());
    }
    for (input, hash) in now {
        match manifest.inputs.get(input) {
            None => reasons.push(format!(
                "{} joined the project after the build",
                short(input)
            )),
            Some(old) if old != hash && hash.is_empty() => {
                reasons.push(format!("{} no longer exists", short(input)));
            }
            Some(old) if old != hash => {
                reasons.push(format!("{} changed after the build", short(input)))
            }
            Some(_) => {}
        }
    }
    for input in manifest.inputs.keys().filter(|i| !now.contains_key(*i)) {
        reasons.push(format!("{} left the project after the build", short(input)));
    }
    reasons
}

/// O nome do arquivo de uma entrada, ou a entrada (`board:de2-115`).
fn short(input: &str) -> &str {
    if input.starts_with("board:") {
        return input;
    }
    Utf8Path::new(input).file_name().unwrap_or(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        Manifest {
            board: "de2-115".into(),
            device: "EP4CE115F29C7".into(),
            top: "proc".into(),
            bitstream: "aaa".into(),
            inputs: [
                ("/p/proc.v".to_owned(), "111".to_owned()),
                ("/p/fpga.json".to_owned(), "222".to_owned()),
                ("board:de2-115".to_owned(), "333".to_owned()),
            ]
            .into_iter()
            .collect(),
        }
    }

    #[test]
    fn the_same_project_and_sof_have_no_differences() {
        let board = super::super::board("de2-115").unwrap();
        let m = manifest();
        assert!(differences(&m, &board, "aaa", &m.inputs).is_empty());
    }

    #[test]
    fn every_kind_of_change_is_named() {
        let board = super::super::board("de10-nano").unwrap();
        let m = manifest();
        let mut now = m.inputs.clone();
        now.insert("/p/proc.v".into(), "999".into());
        now.insert("/p/novo.v".into(), "444".into());
        now.remove("/p/fpga.json");
        now.insert("board:de2-115".into(), String::new());
        let reasons = differences(&m, &board, "bbb", &now);
        let text = reasons.join("\n");
        assert!(
            text.contains("built for de2-115 (EP4CE115F29C7), and fpga.json now says de10-nano"),
            "{text}"
        );
        assert!(text.contains("the .sof is not the one"), "{text}");
        assert!(text.contains("proc.v changed after the build"), "{text}");
        assert!(text.contains("novo.v joined the project"), "{text}");
        assert!(text.contains("fpga.json left the project"), "{text}");
        assert!(text.contains("board:de2-115 no longer exists"), "{text}");
    }
}
