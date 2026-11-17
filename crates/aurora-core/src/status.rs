//! O retrato do projeto: tudo que uma interface mostra de uma vez sobre um
//! projeto aberto, sem rodar ferramenta nenhuma.
//!
//! É o `ProjectSnapshot` que a AURORA IDE montava por conta própria e o
//! `StatusReport` do `aurora status --json`, juntos: a CLI monta o relatório
//! dela a partir daqui (com o processador da pasta atual, que é dela), e o
//! Studio devolve o retrato como está.

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::Serialize;

use crate::project::{Processor, Project};
use crate::{ErrorInfo, FileRole, ProjectFile, ProjectIssue, SimulationResult};

/// O projeto inteiro, como uma interface o mostra. Só lê o disco.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProjectStatus {
    /// O nome do projeto (o do `.spf`).
    pub name: String,
    /// O `.spf`.
    #[schemars(with = "String")]
    pub spf: Utf8PathBuf,
    /// A pasta do projeto.
    #[schemars(with = "String")]
    pub root: Utf8PathBuf,
    /// Os módulos registrados (sintetizáveis), na ordem do `.spf`.
    pub synthesizable: Vec<ProjectFile>,
    /// Os testbenches registrados, na ordem do `.spf`.
    pub testbenches: Vec<ProjectFile>,
    /// O arquivo de topo, se escolhido.
    #[schemars(with = "Option<String>")]
    pub top_level: Option<Utf8PathBuf>,
    /// O módulo de topo, quando dá para saber: o `topLevelModule` do `.spf`,
    /// o único módulo do arquivo de topo ou o que tem o nome dele.
    pub top_module: Option<String>,
    /// Por que o módulo de topo não dá para saber (arquivo com vários
    /// módulos, nenhum com o nome dele).
    pub top_module_error: Option<ErrorInfo>,
    /// O testbench que a simulação do projeto roda.
    #[schemars(with = "Option<String>")]
    pub selected_testbench: Option<Utf8PathBuf>,
    /// O módulo desse testbench.
    pub testbench_module: Option<String>,
    /// A escolha de sinais da onda desse testbench (`wave/<testbench>.json`);
    /// vazia, a onda grava todos os sinais.
    pub wave_selection: Vec<String>,
    /// Os `.v` e `.sv` da pasta que não estão registrados, e os `.py` com
    /// `@cocotb.test`.
    #[schemars(with = "Vec<String>")]
    pub unregistered: Vec<Utf8PathBuf>,
    /// Os processadores SAPHO, na ordem do `.spf`.
    pub processors: Vec<ProcessorStatus>,
    /// A onda da simulação do projeto, se já existe no disco.
    #[schemars(with = "Option<String>")]
    pub waveform: Option<Utf8PathBuf>,
    /// Os Verilog conhecidos que podem ser o topo: os registrados, o gerado
    /// de cada processador e os de fora do `.spf`, menos os com nome de
    /// testbench e os testbenches cocotb (a regra de
    /// [`Project::set_top_level`]).
    #[schemars(with = "Vec<String>")]
    pub top_candidates: Vec<Utf8PathBuf>,
    /// O que está estranho no `.spf` sem impedir de abrir
    /// ([`Project::issues`]).
    pub issues: Vec<ProjectIssue>,
}

/// Um processador, com o que uma interface precisa saber dele além do
/// [`Processor`].
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProcessorStatus {
    /// Os campos do [`Processor`] (nome, linguagem, caminhos, frequência,
    /// clocks, arrays), no mesmo nível.
    #[serde(flatten)]
    pub processor: Processor,
    /// O Verilog e o testbench gerados existem ([`Processor::is_built`]).
    pub built: bool,
    /// Os arquivos de entrada (`<proc>/input_<n>.txt`, ao lado do fonte) que
    /// existem, por porta.
    pub inputs: Vec<PortFile>,
    /// Os arquivos de saída (`<proc>/<proc>/output_<n>.txt`, na pasta de
    /// saída) que existem, por porta.
    pub outputs: Vec<PortFile>,
    /// As entradas que o testbench lê e que não existem; só depois do build,
    /// porque é o testbench gerado que diz quais são.
    #[schemars(with = "Vec<String>")]
    pub missing_inputs: Vec<Utf8PathBuf>,
    /// A onda da simulação do processador, se já existe.
    #[schemars(with = "Option<String>")]
    pub waveform: Option<Utf8PathBuf>,
    /// O que o build gerou e existe no disco.
    pub generated: GeneratedFiles,
}

/// Os arquivos que o build de um processador gera, pelo papel. Só entram os
/// que existem.
#[derive(Debug, Clone, Default, Serialize, JsonSchema)]
pub struct GeneratedFiles {
    /// `<nome>.v`: o processador em Verilog, um módulo do design.
    #[schemars(with = "Option<String>")]
    pub verilog: Option<Utf8PathBuf>,
    /// `<nome>_tb.v`: o testbench que o `asmcomp` gera.
    #[schemars(with = "Option<String>")]
    pub testbench: Option<Utf8PathBuf>,
    /// `<nome>.asm`: o assembly que o compilador gera.
    #[schemars(with = "Option<String>")]
    pub assembly: Option<Utf8PathBuf>,
    /// `*.mif`: as memórias de dados e de instruções.
    #[schemars(with = "Vec<String>")]
    pub memories: Vec<Utf8PathBuf>,
    /// As tabelas e os logs do YANC (`prog_log.txt`, `src_lines.txt`,
    /// `trad_opcode.txt`, `app_log.txt`, `pc_<nome>_mem.txt`); as saídas da
    /// simulação (`output_<n>.txt`, `toma_<n>.txt`) ficam de fora.
    #[schemars(with = "Vec<String>")]
    pub intermediates: Vec<Utf8PathBuf>,
}

impl GeneratedFiles {
    fn of(processor: &Processor) -> Self {
        let existing = |path: Utf8PathBuf| path.is_file().then_some(path);
        let out = &processor.output_dir;
        Self {
            verilog: existing(processor.verilog_path()),
            testbench: existing(processor.testbench_path()),
            assembly: existing(processor.assembly_path()),
            memories: files_with(out, &["mif"]),
            intermediates: files_with(out, &["txt", "log"])
                .into_iter()
                .filter(|p| {
                    p.file_name()
                        .is_some_and(|n| !n.starts_with("output_") && !n.starts_with("toma_"))
                })
                .collect(),
        }
    }
}

/// Um arquivo de porta de entrada ou de saída de um processador.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct PortFile {
    /// A porta (o `n` de `in(n)` e `out(n)`).
    pub port: u32,
    /// O arquivo.
    #[schemars(with = "String")]
    pub path: Utf8PathBuf,
}

/// Os arquivos de `dir` com uma das extensões, em ordem.
fn files_with(dir: &Utf8Path, extensions: &[&str]) -> Vec<Utf8PathBuf> {
    let Ok(entries) = dir.read_dir_utf8() else {
        return Vec::new();
    };
    let mut files: Vec<Utf8PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.into_path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| extensions.contains(&e)))
        .collect();
    files.sort();
    files
}

/// Os `<prefixo><n>.txt` de uma pasta, por porta.
fn port_files(dir: &Utf8Path, prefix: &str) -> Vec<PortFile> {
    let Ok(entries) = dir.read_dir_utf8() else {
        return Vec::new();
    };
    let mut files: Vec<PortFile> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let port = entry
                .file_name()
                .strip_prefix(prefix)?
                .strip_suffix(".txt")?
                .parse()
                .ok()?;
            Some(PortFile {
                port,
                path: entry.path().to_owned(),
            })
        })
        .collect();
    files.sort_by_key(|f| f.port);
    files
}

impl Project {
    /// O retrato do projeto: o que uma interface mostra de uma vez. Só lê o
    /// disco; nenhuma ferramenta roda. Um arquivo de topo sem módulo
    /// identificável não impede o retrato: o motivo vai em
    /// `top_module_error`.
    pub fn status(&self) -> ProjectStatus {
        let (top_module, top_module_error) = match self.top_module() {
            Ok(module) => (module, None),
            Err(error) => (None, Some(ErrorInfo::from(error))),
        };
        let processors: Vec<ProcessorStatus> = self
            .processors()
            .iter()
            .map(|p| {
                let built = p.is_built();
                ProcessorStatus {
                    processor: p.clone(),
                    built,
                    inputs: port_files(&p.dir, "input_"),
                    outputs: port_files(&p.output_dir, "output_"),
                    missing_inputs: if built {
                        crate::missing_inputs(p).unwrap_or_default()
                    } else {
                        Vec::new()
                    },
                    waveform: crate::waveform_path(self, Some(p))
                        .ok()
                        .filter(|w| w.is_file()),
                    generated: GeneratedFiles::of(p),
                }
            })
            .collect();
        ProjectStatus {
            name: self.name().to_owned(),
            spf: self.spf_path().to_owned(),
            root: self.root().to_owned(),
            synthesizable: self.files(FileRole::Synthesizable),
            testbenches: self.files(FileRole::Testbench),
            top_level: self.top_level(),
            top_module,
            top_module_error,
            selected_testbench: self.testbench(),
            testbench_module: self.testbench_module().ok().flatten(),
            wave_selection: crate::wave_testbench(self)
                .and_then(|module| crate::read_selection(self, &module))
                .unwrap_or_default(),
            top_candidates: top_candidates(self, &processors),
            unregistered: self.unregistered_hdl(),
            processors,
            waveform: crate::waveform_path(self, None)
                .ok()
                .filter(|w| w.is_file()),
            issues: self.issues(),
        }
    }
}

/// Os candidatos a topo, na ordem: registrados (módulos e testbenches), o
/// Verilog gerado dos processadores e os de fora do `.spf`. Um arquivo com
/// nome de testbench ou um testbench cocotb nunca é topo.
fn top_candidates(project: &Project, processors: &[ProcessorStatus]) -> Vec<Utf8PathBuf> {
    let mut candidates: Vec<Utf8PathBuf> = Vec::new();
    let registered = [FileRole::Synthesizable, FileRole::Testbench]
        .into_iter()
        .flat_map(|role| project.files(role))
        .map(|f| f.path);
    let generated = processors
        .iter()
        .filter_map(|p| p.generated.verilog.clone());
    for path in registered
        .chain(generated)
        .chain(project.unregistered_hdl())
    {
        let name = path.file_name().unwrap_or_default();
        if !crate::verilog::is_testbench_name(name)
            && !crate::cocotb::is_testbench(&path)
            && !candidates.contains(&path)
        {
            candidates.push(path);
        }
    }
    candidates
}

impl Processor {
    /// As portas de saída que uma simulação deste processador escreveu, em
    /// ordem e sem repetição: o `n` de cada `output_<n>.txt` de
    /// [`SimulationResult::outputs`] que está na pasta de saída. A porta sai
    /// do nome do arquivo porque o processador não guarda o `#NUIOOU`.
    pub fn output_ports(&self, result: &SimulationResult) -> Vec<u32> {
        let dir = &self.output_dir;
        let mut ports: Vec<u32> = result
            .outputs
            .iter()
            .filter(|path| path.parent() == Some(dir.as_path()))
            .filter_map(|path| {
                path.file_name()?
                    .strip_prefix("output_")?
                    .strip_suffix(".txt")?
                    .parse()
                    .ok()
            })
            .collect();
        ports.sort_unstable();
        ports.dedup();
        ports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_files_are_sorted_by_port_and_ignore_other_names() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        for name in [
            "input_2.txt",
            "input_0.txt",
            "input_x.txt",
            "output_1.txt",
            "nota.txt",
        ] {
            std::fs::write(root.join(name), "1\n").unwrap();
        }
        let inputs = port_files(root, "input_");
        assert_eq!(inputs.iter().map(|f| f.port).collect::<Vec<_>>(), [0, 2]);
        assert_eq!(inputs[0].path, root.join("input_0.txt"));
        assert_eq!(port_files(root, "output_").len(), 1);
        assert!(port_files(&root.join("nao-existe"), "input_").is_empty());
    }

    #[test]
    fn files_with_filters_by_extension_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        for name in ["b.mif", "a.mif", "c.txt", "d.v"] {
            std::fs::write(root.join(name), "").unwrap();
        }
        let mifs = files_with(root, &["mif"]);
        assert_eq!(mifs, [root.join("a.mif"), root.join("b.mif")]);
        assert_eq!(files_with(root, &["txt", "log"]), [root.join("c.txt")]);
    }
}
