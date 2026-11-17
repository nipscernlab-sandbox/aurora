//! Arquivos do projeto além dos processadores: Verilog e VHDL sintetizáveis,
//! testbenches e os arquivos de entrada e saída da simulação.
//!
//! No `.spf` (AURORA, `js/project/file_mode.js` e `spf_store.ts`):
//!
//! - `structure.synthesizableFiles` e `structure.testbenchFiles`: listas de
//!   `{ "name", "path", "isTopLevel" }`. O caminho é relativo à raiz quando o
//!   arquivo está dentro dela; fora dela, relativo com `..` quando o arquivo
//!   e o projeto estão no mesmo repositório git, e absoluto quando não. Um
//!   absoluto de outra máquina que não existe aqui é procurado pela cauda
//!   dentro da raiz, como a AURORA faz.
//! - `structure.topLevelFile`: o arquivo do módulo de topo, que a síntese
//!   usa; o módulo é o único do arquivo, o que tem o nome dele ou, se foi
//!   escolhido pelo nome, `structure.topLevelModule` (campo da AURORA).
//! - `structure.testbenchFile`: o testbench que a simulação do projeto roda.
//!
//! Um `.py` só entra como testbench: é um testbench cocotb, como na AURORA
//! ([`cocotb`](crate::cocotb)). A linguagem de cada arquivo sai da extensão
//! ([`SourceLanguage`]); o `.spf` não a guarda, e todo VHDL fica na
//! biblioteca `work`.
//!
//! As listas são lidas do documento a cada chamada: o `.spf` é a única fonte,
//! e nada fica duplicado em memória para sair de sincronia.
//!
//! # Exemplo
//!
//! ```
//! use aurora_core::{FileRole, Project};
//! # let tmp = tempfile::tempdir()?;
//! # let dir = camino::Utf8Path::from_path(tmp.path()).unwrap();
//!
//! let mut project = Project::create(dir, "p")?;
//! let top = project.add_file(FileRole::Synthesizable, "rtl/top.v", Some("module top; endmodule\n"))?;
//! let tb = project.add_file(FileRole::Testbench, "rtl/top_tb.v", Some("module top_tb; endmodule\n"))?;
//! project.set_top_level(&top)?;
//! project.set_testbench(&tb)?;
//! assert_eq!(project.top_level(), Some(top));
//! assert_eq!(project.testbench(), Some(tb));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::{AuroraError, Result};
use crate::hdl::SourceLanguage;
use crate::project::{Processor, Project};

/// Em qual lista do `.spf` um arquivo fica. Em JSON: `"synthesizable"` ou
/// `"testbench"`.
///
/// [`Project::add_verilog`] decide o papel pelo conteúdo, como a AURORA
/// ([`classify`](crate::verilog::classify)); [`Project::add_file`] recebe o
/// papel de quem chama. Um arquivo fica numa lista só.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FileRole {
    /// Entra na síntese e na simulação.
    Synthesizable,
    /// Só entra na simulação.
    Testbench,
}

impl FileRole {
    fn list_key(self) -> &'static str {
        match self {
            FileRole::Synthesizable => "synthesizableFiles",
            FileRole::Testbench => "testbenchFiles",
        }
    }

    fn other(self) -> FileRole {
        match self {
            FileRole::Synthesizable => FileRole::Testbench,
            FileRole::Testbench => FileRole::Synthesizable,
        }
    }

    fn selected_key(self) -> &'static str {
        match self {
            FileRole::Synthesizable => "topLevelFile",
            FileRole::Testbench => "testbenchFile",
        }
    }
}

/// Um arquivo registrado no projeto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct ProjectFile {
    /// Em qual lista ele está.
    pub role: FileRole,
    /// Caminho absoluto.
    #[schemars(with = "String")]
    pub path: Utf8PathBuf,
    /// Marcado como topo (sintetizável) ou como o testbench escolhido.
    pub top_level: bool,
    /// A linguagem, pela extensão; `None` para um arquivo de outra extensão
    /// que o `.spf` traga.
    pub language: Option<SourceLanguage>,
}

/// O que [`Project::add_hdl`] fez com um arquivo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct AddedFile {
    /// Caminho absoluto.
    #[schemars(with = "String")]
    pub path: Utf8PathBuf,
    /// Em qual lista ficou.
    pub role: FileRole,
    /// A linguagem, pela extensão.
    pub language: Option<SourceLanguage>,
    /// O arquivo não existia e foi criado a partir do modelo.
    pub created: bool,
    /// Virou o módulo de topo (era o primeiro sintetizável) ou o testbench
    /// escolhido.
    pub selected: bool,
}

/// Para onde [`Project::reorder_file`] leva um arquivo na lista dele. Em
/// JSON: `{"kind": "first"}`, `{"kind": "before", "path": "rtl/b.v"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "path")]
#[non_exhaustive]
pub enum ListPosition {
    /// O primeiro da lista.
    First,
    /// O último.
    Last,
    /// Logo antes deste arquivo, da mesma lista.
    Before(#[schemars(with = "String")] Utf8PathBuf),
    /// Logo depois deste arquivo, da mesma lista.
    After(#[schemars(with = "String")] Utf8PathBuf),
}

/// O que [`Project::move_path`] fez.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct MovedPath {
    /// Onde estava, absoluto.
    #[schemars(with = "String")]
    pub from: Utf8PathBuf,
    /// Onde ficou, absoluto.
    #[schemars(with = "String")]
    pub to: Utf8PathBuf,
    /// Os arquivos registrados no `.spf` que mudaram de lugar, já com o
    /// caminho novo: o próprio arquivo, ou os que estavam dentro da pasta.
    /// Continuam com o mesmo papel e a mesma marca de topo ou de testbench
    /// escolhido.
    pub files: Vec<ProjectFile>,
}

/// `project.remove`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct RemoveOutcome {
    /// Tirados do projeto (continuam no disco).
    #[schemars(with = "Vec<String>")]
    pub removed: Vec<Utf8PathBuf>,
    /// Pedidos, mas que não estavam no projeto.
    #[schemars(with = "Vec<String>")]
    pub not_registered: Vec<Utf8PathBuf>,
}

impl Project {
    /// Registra um arquivo Verilog ou VHDL, ou um testbench cocotb (`.py`),
    /// criando-o a partir do modelo se ele não existir, e grava o `.spf`.
    ///
    /// - Arquivo existente: o papel vem do conteúdo
    ///   ([`verilog::classify`](crate::verilog::classify) ou
    ///   [`vhdl::classify`](crate::vhdl::classify)), ou é testbench com
    ///   `testbench`. Se ele estava na outra lista, muda de lista.
    /// - Arquivo novo: é testbench com `testbench` ou se o nome indicar (`_tb`,
    ///   `tb_`, `test`); senão, módulo (ou entidade). O testbench-modelo
    ///   instancia o módulo do nome (`and_gate_tb.v` testa `and_gate`) ou,
    ///   sem ele, o de topo, com as portas lidas por
    ///   [`read_interfaces`](crate::verilog::read_interfaces). O de VHDL
    ///   instancia uma entidade VHDL registrada pela mesma regra
    ///   ([`vhdl::testbench_template`](crate::vhdl::testbench_template)).
    /// - Um `.py` é sempre testbench. Novo, sai do modelo cocotb
    ///   ([`cocotb::testbench_template`](crate::cocotb::testbench_template)),
    ///   com o módulo testado escolhido pela mesma regra (`test_somador.py`
    ///   testa `somador`) na diretiva `# aurora-toplevel:`.
    /// - O primeiro sintetizável vira o topo; o primeiro testbench, o
    ///   testbench escolhido.
    ///
    /// `path` absoluto ou relativo à raiz do projeto.
    pub fn add_hdl(
        &mut self,
        toolchain: Option<&crate::Toolchain>,
        path: impl AsRef<Utf8Path>,
        testbench: bool,
    ) -> Result<AddedFile> {
        let path = self.absolute(path.as_ref());
        self.check_add_hdl(&path)?;
        let file_name = path.file_name().unwrap_or_default().to_owned();
        let created = !path.exists();
        let language = SourceLanguage::of(&path);
        let python = crate::cocotb::is_testbench(&path);
        let vhdl = language == Some(SourceLanguage::Vhdl);
        let mut declares_module = true;
        let role = if created {
            let stem = path.file_stem().unwrap_or_default().to_owned();
            let role = if python || testbench || crate::verilog::name_suggests_testbench(&file_name)
            {
                FileRole::Testbench
            } else {
                FileRole::Synthesizable
            };
            let text = match role {
                FileRole::Synthesizable if vhdl => crate::vhdl::entity_template(&stem),
                FileRole::Synthesizable => crate::verilog::module_template(&stem),
                FileRole::Testbench if vhdl => {
                    let dut = self.entity_under_test(&stem);
                    crate::vhdl::testbench_template(&stem, dut.as_ref())
                }
                FileRole::Testbench => {
                    let dut = self.module_under_test(toolchain, &stem)?;
                    if python {
                        crate::cocotb::testbench_template(dut.as_ref())
                    } else {
                        crate::verilog::testbench_template(&stem, dut.as_ref())
                    }
                }
            };
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(AuroraError::io("Creating directory", parent))?;
            }
            std::fs::write(&path, text).map_err(AuroraError::io("Creating file", &path))?;
            role
        } else {
            let text = std::fs::read_to_string(&path).map_err(AuroraError::io("Reading", &path))?;
            declares_module = !units_in(&path, &text).is_empty();
            if testbench || python {
                FileRole::Testbench
            } else if vhdl {
                crate::vhdl::classify(&text, &file_name)
            } else {
                crate::verilog::classify(&text, &file_name)
            }
        };

        self.add_file(role, &path, None)?;
        let selected = match role {
            // O primeiro sintetizável vira o topo, menos com nome de
            // testbench, que não pode ser topo, e sem módulo (um arquivo só
            // de `define).
            FileRole::Synthesizable
                if self.top_level().is_none()
                    && declares_module
                    && !crate::verilog::is_testbench_name(&file_name) =>
            {
                self.set_top_level(&path)?;
                true
            }
            FileRole::Testbench if self.selected(FileRole::Testbench).is_none() => {
                self.set_testbench(&path)?;
                true
            }
            _ => self.selected(role).as_ref() == Some(&path),
        };
        Ok(AddedFile {
            path,
            role,
            language,
            created,
            selected,
        })
    }

    /// O mesmo que [`Project::add_hdl`], com o nome de antes do VHDL.
    pub fn add_verilog(
        &mut self,
        toolchain: Option<&crate::Toolchain>,
        path: impl AsRef<Utf8Path>,
        testbench: bool,
    ) -> Result<AddedFile> {
        self.add_hdl(toolchain, path, testbench)
    }

    /// Registra no `.spf` o que o build de cada processador gerou e o projeto
    /// ainda não tem: o `<nome>/<nome>/<nome>.v` entre os sintetizáveis e o
    /// `<nome>/<nome>/<nome>_tb.v` entre os testbenches, no fim de cada lista.
    /// É o que a AURORA faz ao atualizar a árvore de arquivos, e o que
    /// `aurora build`, `sim`, `synth` e `fpga build` chamam depois de compilar.
    ///
    /// - Só os builds que terminaram bem, e só os arquivos que existem.
    /// - Num projeto sem sintetizáveis, o `.v` do primeiro processador vira o
    ///   topo; num sem testbench, o testbench dele vira o escolhido.
    /// - O que já está registrado, em qualquer das listas, fica como está.
    ///   Um arquivo tirado com [`remove_file`](Self::remove_file) volta no
    ///   build seguinte.
    ///
    /// As operações já usavam o `<nome>.v` gerado sem ele estar no `.spf`
    /// (e tiram o repetido quando está); registrado, ele aparece na lista de
    /// arquivos de quem lê o `.spf`, e a AURORA, que só compila o que está
    /// nele, o encontra.
    ///
    /// ```no_run
    /// use aurora_core::{BuildOptions, Control, OnFailure, Project, Toolchain, build_processors};
    /// # let toolchain = Toolchain::open("/opt/aurora/toolchain")?;
    /// let mut project = Project::open("/p/demo")?;
    /// let builds = build_processors(
    ///     &toolchain,
    ///     project.buildable_processors(),
    ///     &BuildOptions::default(),
    ///     OnFailure::Continue,
    ///     &Control::default(),
    ///     |_| {},
    /// )?;
    /// for file in project.register_generated(&builds)? {
    ///     println!("{} entrou no projeto ({:?})", file.path, file.role);
    /// }
    /// # Ok::<(), aurora_core::AuroraError>(())
    /// ```
    ///
    /// # Erros
    ///
    /// [`AuroraError::Io`] se o `.spf` não puder ser gravado. Os registrados
    /// antes do erro ficam.
    pub fn register_generated(&mut self, builds: &[crate::BuildResult]) -> Result<Vec<AddedFile>> {
        use crate::ArtifactKind;
        let mut added = Vec::new();
        for build in builds.iter().filter(|b| b.succeeded()) {
            for (kind, role) in [
                (ArtifactKind::Verilog, FileRole::Synthesizable),
                (ArtifactKind::Testbench, FileRole::Testbench),
            ] {
                let Some(artifact) = build
                    .artifacts
                    .iter()
                    .find(|a| a.kind == kind && a.path.is_file())
                else {
                    continue;
                };
                let path = self.absolute(&artifact.path);
                let registered = [FileRole::Synthesizable, FileRole::Testbench]
                    .into_iter()
                    .any(|r| self.files(r).iter().any(|f| f.path == path));
                if registered {
                    continue;
                }
                let first = self.files(role).is_empty();
                self.add_file(role, &path, None)?;
                let selected = match role {
                    FileRole::Synthesizable if first && self.top_level().is_none() => {
                        self.set_top_level(&path)?;
                        true
                    }
                    FileRole::Testbench if self.selected(FileRole::Testbench).is_none() => {
                        self.set_testbench(&path)?;
                        true
                    }
                    _ => false,
                };
                tracing::info!(%path, ?role, selected, "Generated file registered");
                added.push(AddedFile {
                    language: SourceLanguage::of(&path),
                    path,
                    role,
                    created: false,
                    selected,
                });
            }
        }
        Ok(added)
    }

    /// Registra vários arquivos, na ordem ([`Project::add_hdl`] para cada
    /// um), conferindo todos antes de registrar o primeiro
    /// ([`Project::check_add_hdl`]): um nome ruim no meio da lista não deixa
    /// metade registrada. É o `aurora add` e o `project.add`.
    pub fn add_hdl_files(
        &mut self,
        toolchain: Option<&crate::Toolchain>,
        paths: &[Utf8PathBuf],
        testbench: bool,
    ) -> Result<Vec<AddedFile>> {
        for path in paths {
            self.check_add_hdl(path)?;
        }
        paths
            .iter()
            .map(|path| self.add_hdl(toolchain, path, testbench))
            .collect()
    }

    /// O mesmo que [`Project::add_hdl_files`], com o nome de antes do VHDL.
    pub fn add_verilog_files(
        &mut self,
        toolchain: Option<&crate::Toolchain>,
        paths: &[Utf8PathBuf],
        testbench: bool,
    ) -> Result<Vec<AddedFile>> {
        self.add_hdl_files(toolchain, paths, testbench)
    }

    /// Confere, sem mudar nada, se [`Project::add_hdl`] aceitaria `path`:
    /// termina em `.v`, `.sv`, `.vhd`, `.vhdl` ou `.py` e, se ainda não
    /// existe, o nome do arquivo serve de nome de módulo do Verilog, de
    /// entidade do VHDL ou de módulo do Python. Quem registra vários
    /// arquivos confere todos antes, para não registrar metade.
    ///
    /// # Erros
    ///
    /// [`AuroraError::InvalidName`], com o motivo.
    pub fn check_add_hdl(&self, path: impl AsRef<Utf8Path>) -> Result<()> {
        let path = self.absolute(path.as_ref());
        check_extension(&path, None)?;
        if !path.exists() {
            let stem = path.file_stem().unwrap_or_default();
            match SourceLanguage::of(&path) {
                Some(SourceLanguage::Python) => crate::cocotb::test_module(&path).map(|_| ())?,
                Some(SourceLanguage::Vhdl) => {
                    crate::vhdl::validate_identifier(stem, "The file name becomes the entity name")?
                }
                _ => crate::project::validate_identifier(
                    stem,
                    "The file name becomes the module name",
                )?,
            }
        }
        Ok(())
    }

    /// O mesmo que [`Project::check_add_hdl`], com o nome de antes do VHDL.
    pub fn check_add_verilog(&self, path: impl AsRef<Utf8Path>) -> Result<()> {
        self.check_add_hdl(path)
    }

    /// A entidade VHDL que o testbench `stem` testa, com os genéricos e as
    /// portas: a do nome (`contador_tb` testa `contador`) entre os
    /// sintetizáveis VHDL registrados, ou a de topo, se for VHDL.
    fn entity_under_test(&self, stem: &str) -> Option<crate::vhdl::Entity> {
        let mut wanted: Vec<String> = tested_name(stem).map(str::to_owned).into_iter().collect();
        if let Ok(Some(top)) = self.top_module() {
            wanted.push(top);
        }
        let entities: Vec<crate::vhdl::Entity> = self
            .files(FileRole::Synthesizable)
            .into_iter()
            .filter(|f| crate::hdl::is_vhdl(&f.path))
            .filter_map(|f| std::fs::read_to_string(&f.path).ok())
            .flat_map(|text| crate::vhdl::entities(&text))
            .collect();
        wanted.iter().find_map(|name| {
            entities
                .iter()
                .find(|e| e.name.eq_ignore_ascii_case(name))
                .cloned()
        })
    }

    /// O módulo que o testbench `stem` testa: o do nome (`and_tb` e `tb_and`
    /// testam `and`) entre os sintetizáveis registrados, ou o de topo.
    fn module_under_test(
        &self,
        toolchain: Option<&crate::Toolchain>,
        stem: &str,
    ) -> Result<Option<crate::verilog::ModuleInterface>> {
        let mut wanted: Vec<String> = tested_name(stem).map(str::to_owned).into_iter().collect();
        if let Ok(Some(top)) = self.top_module() {
            wanted.push(top);
        }
        let files: Vec<Utf8PathBuf> = self
            .files(FileRole::Synthesizable)
            .into_iter()
            .map(|f| f.path)
            .filter(|p| p.is_file() && crate::hdl::is_verilog(p))
            .collect();
        for module in &wanted {
            let declaring: Vec<Utf8PathBuf> = files
                .iter()
                .filter(|f| {
                    std::fs::read_to_string(f)
                        .is_ok_and(|t| crate::verilog::modules_in(&t).contains(module))
                })
                .cloned()
                .collect();
            if declaring.is_empty() {
                continue;
            }
            let interfaces =
                crate::verilog::read_interfaces_in(toolchain, &declaring, Some(self.root()))?;
            if let Some(found) = interfaces.into_iter().find(|i| &i.name == module) {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    /// Tira o arquivo das duas listas e das escolhas de topo e testbench (não
    /// apaga do disco). Diz se ele estava registrado.
    pub fn remove_hdl(&mut self, path: impl AsRef<Utf8Path>) -> Result<bool> {
        let path = self.absolute(path.as_ref());
        let synthesizable = self.remove_file(FileRole::Synthesizable, &path)?;
        let testbench = self.remove_file(FileRole::Testbench, &path)?;
        Ok(synthesizable || testbench)
    }

    /// O mesmo que [`Project::remove_hdl`], com o nome de antes do VHDL.
    pub fn remove_verilog(&mut self, path: impl AsRef<Utf8Path>) -> Result<bool> {
        self.remove_hdl(path)
    }

    /// Tira vários arquivos das listas, na ordem ([`Project::remove_hdl`]
    /// para cada um), e diz quais estavam registrados e quais não: o mesmo
    /// arquivo duas vezes sai na primeira e não está mais na segunda. É o
    /// `aurora remove` e o `project.remove`.
    pub fn remove_hdl_files(&mut self, paths: &[Utf8PathBuf]) -> Result<RemoveOutcome> {
        let mut removed = Vec::new();
        let mut not_registered = Vec::new();
        for path in paths {
            if self.remove_hdl(path)? {
                removed.push(path.clone());
            } else {
                not_registered.push(path.clone());
            }
        }
        Ok(RemoveOutcome {
            removed,
            not_registered,
        })
    }

    /// O mesmo que [`Project::remove_hdl_files`], com o nome de antes do VHDL.
    pub fn remove_verilog_files(&mut self, paths: &[Utf8PathBuf]) -> Result<RemoveOutcome> {
        self.remove_hdl_files(paths)
    }

    /// Escolhe o topo por arquivo (caminho absoluto ou relativo à raiz, que
    /// é registrado se ainda não estiver) ou por nome de módulo (ou de
    /// entidade, sem diferenciar maiúsculas no VHDL) entre os sintetizáveis
    /// registrados. Devolve o arquivo.
    ///
    /// # Erros
    ///
    /// - [`AuroraError::ModuleNotFound`] se não é um arquivo e nenhum
    ///   sintetizável declara o módulo;
    /// - [`AuroraError::AmbiguousModule`] se mais de um declara;
    /// - os de [`Project::set_top_level`].
    pub fn set_top(&mut self, target: &str) -> Result<Utf8PathBuf> {
        self.begin_write()?;
        let as_path = self.absolute(Utf8Path::new(target));
        if as_path.is_file() {
            self.set_top_level(&as_path)?;
            return Ok(as_path);
        }
        let mut available: Vec<String> = Vec::new();
        let mut declaring = Vec::new();
        for file in self.files(FileRole::Synthesizable) {
            let Ok(text) = std::fs::read_to_string(&file.path) else {
                continue;
            };
            let modules = units_in(&file.path, &text);
            if modules.iter().any(|m| same_unit(&file.path, m, target))
                && !declaring.contains(&file.path)
            {
                declaring.push(file.path);
            }
            for module in modules {
                if !available.contains(&module) {
                    available.push(module);
                }
            }
        }
        match declaring.as_slice() {
            [] => Err(AuroraError::ModuleNotFound {
                name: target.to_owned(),
                available,
            }),
            [file] => {
                let file = file.clone();
                self.set_top_level(&file)?;
                // O arquivo pode ter vários módulos: o escolhido fica gravado
                // (a AURORA ignora o campo e usa o nome do arquivo).
                self.begin_write()?;
                self.document["structure"][TOP_MODULE_KEY] = target.into();
                self.save()?;
                Ok(file)
            }
            _ => Err(AuroraError::AmbiguousModule {
                name: target.to_owned(),
                files: declaring,
            }),
        }
    }

    /// O nome do módulo de topo: o único módulo do arquivo de topo, ou o que
    /// tem o nome do arquivo. `None` sem topo.
    ///
    /// # Erros
    ///
    /// [`AuroraError::ModuleNotFound`] se o arquivo não declara módulo, ou
    /// declara vários e nenhum tem o nome dele.
    pub fn top_module(&self) -> Result<Option<String>> {
        let Some(file) = self.top_level() else {
            return Ok(None);
        };
        if let Some(chosen) = self.document["structure"][TOP_MODULE_KEY]
            .as_str()
            .filter(|m| !m.is_empty())
        {
            let text = std::fs::read_to_string(&file).map_err(AuroraError::io("Reading", &file))?;
            if let Some(found) = units_in(&file, &text)
                .into_iter()
                .find(|m| same_unit(&file, m, chosen))
            {
                return Ok(Some(found));
            }
        }
        module_of(&file).map(Some)
    }

    /// O nome do módulo do testbench escolhido: o único do arquivo, o que
    /// tem o nome dele, ou, com vários e nenhum com o nome, o único que
    /// nenhum outro do arquivo instancia (`bancada_tb.v` com `gerador` e
    /// `principal`, que instancia `gerador`, simula `principal`). Num
    /// testbench cocotb (`.py`), o módulo Python dos testes: o nome do
    /// arquivo.
    pub fn testbench_module(&self) -> Result<Option<String>> {
        self.testbench()
            .map(|f| {
                if crate::cocotb::is_testbench(&f) {
                    Ok(f.file_stem().unwrap_or_default().to_owned())
                } else {
                    testbench_module_of(&f)
                }
            })
            .transpose()
    }

    /// Os `.v`, `.sv`, `.vhd` e `.vhdl` da pasta do projeto e das subpastas
    /// (até 4 níveis; fora `.aurora8/`, as ocultas, as dos processadores, a
    /// `TopLevel/` legada, que entra sozinha, e as de outro projeto, com
    /// `.spf` próprio) que não estão registrados, e os `.py` com testes
    /// cocotb (`@cocotb.test`).
    pub fn unregistered_hdl(&self) -> Vec<Utf8PathBuf> {
        let registered: Vec<Utf8PathBuf> = self
            .files(FileRole::Synthesizable)
            .into_iter()
            .chain(self.files(FileRole::Testbench))
            .map(|f| f.path)
            .collect();
        let skip: Vec<Utf8PathBuf> = self
            .processors()
            .iter()
            .map(|p| p.dir.clone())
            .chain([self.root.join("TopLevel")])
            .collect();
        let mut found = Vec::new();
        let mut pending = vec![(self.root.clone(), 0)];
        while let Some((dir, depth)) = pending.pop() {
            let Ok(entries) = dir.read_dir_utf8() else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path().to_owned();
                let name = entry.file_name();
                if name.starts_with('.') {
                    continue;
                }
                if path.is_dir() {
                    // Uma subpasta com `.spf` é outro projeto: os arquivos
                    // dela são dele.
                    let other_project = !matches!(crate::project::spf_in(&path), Ok(None));
                    if depth < 4 && !skip.contains(&path) && !other_project {
                        pending.push((path, depth + 1));
                    }
                } else if (crate::hdl::is_verilog(&path)
                    || crate::hdl::is_vhdl(&path)
                    || (crate::cocotb::is_testbench(&path)
                        && std::fs::read_to_string(&path)
                            .is_ok_and(|t| t.contains("@cocotb.test"))))
                    && !registered.contains(&path)
                {
                    found.push(path);
                }
            }
        }
        found.sort();
        found
    }

    /// O mesmo que [`Project::unregistered_hdl`], com o nome de antes do
    /// VHDL.
    pub fn unregistered_verilog(&self) -> Vec<Utf8PathBuf> {
        self.unregistered_hdl()
    }

    /// Os arquivos de um papel, na ordem do `.spf`.
    ///
    /// Um arquivo fica numa lista só, também na leitura de um `.spf` editado
    /// à mão: repetido na mesma lista (inclusive por outro caminho, como
    /// `rtl/../rtl/a.v` ou `rtl\a.v`), conta uma vez, e marcado se alguma das
    /// entradas está; nas duas listas, conta como testbench se o nome indica
    /// (`_tb`, `tb_`, `test`), e como sintetizável senão. É um aviso de
    /// [`Project::issues`] (`duplicate_file`), e a próxima gravação tira as
    /// entradas que sobram.
    ///
    /// A marca de topo é `isTopLevel`; nos testbenches, também o
    /// `isMarkedTestbench` legado da AURORA, que ela trata como igual.
    pub fn files(&self, role: FileRole) -> Vec<ProjectFile> {
        let theirs: Vec<Utf8PathBuf> = self
            .listed(role.other())
            .into_iter()
            .map(|f| f.path)
            .collect();
        let mut files: Vec<ProjectFile> = Vec::new();
        for file in self.listed(role) {
            if let Some(seen) = files.iter_mut().find(|f| f.path == file.path) {
                seen.top_level |= file.top_level;
                continue;
            }
            if theirs.contains(&file.path) && owner(&file.path) != role {
                continue;
            }
            files.push(file);
        }
        files
    }

    /// As entradas de uma lista como estão, com repetidos.
    fn listed(&self, role: FileRole) -> Vec<ProjectFile> {
        let Some(list) = self.document["structure"][role.list_key()].as_array() else {
            return Vec::new();
        };
        list.iter()
            .filter_map(|entry| {
                let path = self.resolve_path(entry_path(entry)?);
                Some(ProjectFile {
                    role,
                    language: SourceLanguage::of(&path),
                    path,
                    top_level: is_marked(entry),
                })
            })
            .collect()
    }

    /// O arquivo de topo: `topLevelFile`, senão o sintetizável marcado.
    pub fn top_level(&self) -> Option<Utf8PathBuf> {
        self.selected(FileRole::Synthesizable)
    }

    /// O testbench da simulação do projeto, na ordem da AURORA
    /// (`escolherTestbench`): `testbenchFile`; senão o testbench marcado; senão
    /// o primeiro da lista.
    pub fn testbench(&self) -> Option<Utf8PathBuf> {
        self.selected(FileRole::Testbench).or_else(|| {
            self.files(FileRole::Testbench)
                .into_iter()
                .next()
                .map(|f| f.path)
        })
    }

    /// O escolhido de um papel: `topLevelFile` (ou `testbenchFile`) se está
    /// registrado na lista do papel, senão o marcado com `isTopLevel`. Um
    /// escolhido que não está na lista, ou um topo com nome de testbench, não
    /// vale para nenhuma operação (é um aviso de [`Project::issues`]): assim
    /// `status`, `top`, `synth` e `check` veem o mesmo topo.
    fn selected(&self, role: FileRole) -> Option<Utf8PathBuf> {
        let files = self.files(role);
        let usable = |path: &Utf8Path| {
            role == FileRole::Testbench
                || !crate::verilog::is_testbench_name(path.file_name().unwrap_or_default())
        };
        let declared = self
            .declared_selection(role)
            .filter(|p| usable(p) && files.iter().any(|f| &f.path == p));
        declared.or_else(|| {
            files
                .into_iter()
                .find(|f| f.top_level && usable(&f.path))
                .map(|f| f.path)
        })
    }

    /// O que `topLevelFile` (ou `testbenchFile`) diz, registrado ou não.
    fn declared_selection(&self, role: FileRole) -> Option<Utf8PathBuf> {
        self.document["structure"][role.selected_key()]
            .as_str()
            .filter(|p| !p.trim().is_empty())
            .map(|p| self.resolve_path(p))
    }

    /// Os avisos sobre os arquivos do `.spf` ([`Project::issues`]).
    pub(crate) fn file_issues(&self) -> Vec<crate::ProjectIssue> {
        use crate::project::{IssueKind, ProjectIssue};
        let mut issues = Vec::new();
        let synthesizable = self.listed(FileRole::Synthesizable);
        let testbenches = self.listed(FileRole::Testbench);
        let mut reported: Vec<Utf8PathBuf> = Vec::new();
        for file in synthesizable.iter().chain(&testbenches) {
            if reported.contains(&file.path) {
                continue;
            }
            let count = |list: &[ProjectFile]| list.iter().filter(|f| f.path == file.path).count();
            let (in_synth, in_tb) = (count(&synthesizable), count(&testbenches));
            let message = if in_synth > 0 && in_tb > 0 {
                let role = match owner(&file.path) {
                    FileRole::Synthesizable => "a module",
                    FileRole::Testbench => "a testbench",
                };
                format!(
                    "{} is in synthesizableFiles and in testbenchFiles; it counts once, as {role}",
                    file.path
                )
            } else if in_synth + in_tb > 1 {
                format!(
                    "{} is listed {} times in {}; it counts once",
                    file.path,
                    in_synth + in_tb,
                    file.role.list_key()
                )
            } else {
                continue;
            };
            reported.push(file.path.clone());
            issues.push(ProjectIssue::new(
                IssueKind::DuplicateFile,
                Some(file.path.clone()),
                Some(file.role.list_key().to_owned()),
                message,
            ));
        }
        for role in [FileRole::Synthesizable, FileRole::Testbench] {
            let list = self.document["structure"][role.list_key()].as_array();
            for stored in list.into_iter().flatten().filter_map(entry_path) {
                if let Some(found) = rescued(&self.root, stored) {
                    issues.push(ProjectIssue::new(
                        IssueKind::RescuedPath,
                        Some(found.clone()),
                        Some(stored.to_owned()),
                        format!("{stored} does not exist here; using {found}, found inside the project folder"),
                    ));
                }
            }
            let Some(declared) = self.declared_selection(role) else {
                continue;
            };
            let field = role.selected_key();
            if !self.files(role).iter().any(|f| f.path == declared) {
                issues.push(ProjectIssue::new(
                    IssueKind::SelectionNotRegistered,
                    Some(declared.clone()),
                    Some(field.to_owned()),
                    format!(
                        "{field} is {declared}, which is not in {}; it is ignored",
                        role.list_key()
                    ),
                ));
            } else if role == FileRole::Synthesizable
                && crate::verilog::is_testbench_name(declared.file_name().unwrap_or_default())
            {
                issues.push(ProjectIssue::new(
                    IssueKind::TestbenchAsTop,
                    Some(declared.clone()),
                    Some(field.to_owned()),
                    format!("{field} is {declared}, a testbench name, which cannot be the top level; it is ignored"),
                ));
            }
        }
        issues
    }

    /// Tira do documento as entradas que [`Project::files`] não conta: as
    /// repetidas numa lista (a marca de topo passa para a que fica) e as que
    /// estão nas duas listas, da lista que não é a do arquivo. Chamado antes
    /// de cada gravação, como [`Project::repair_rescued`].
    pub(crate) fn repair_duplicates(&mut self) {
        let root = self.root.clone();
        let resolved = |role: FileRole| -> Vec<Utf8PathBuf> {
            self.listed(role).into_iter().map(|f| f.path).collect()
        };
        let lists = [
            resolved(FileRole::Synthesizable),
            resolved(FileRole::Testbench),
        ];
        for (i, role) in [FileRole::Synthesizable, FileRole::Testbench]
            .into_iter()
            .enumerate()
        {
            let theirs = &lists[1 - i];
            let Some(list) = self
                .document
                .get_mut("structure")
                .and_then(|s| s.get_mut(role.list_key()))
                .and_then(Value::as_array_mut)
            else {
                continue;
            };
            let mut seen: Vec<(Utf8PathBuf, usize)> = Vec::new();
            let mut kept: Vec<Value> = Vec::with_capacity(list.len());
            for entry in list.drain(..) {
                let Some(path) = entry_path(&entry).map(|p| resolve(&root, p)) else {
                    kept.push(entry);
                    continue;
                };
                if theirs.contains(&path) && owner(&path) != role {
                    continue;
                }
                if let Some((_, at)) = seen.iter().find(|(p, _)| *p == path) {
                    if is_marked(&entry) && kept[*at].is_object() {
                        kept[*at]["isTopLevel"] = true.into();
                    }
                    continue;
                }
                seen.push((path, kept.len()));
                kept.push(entry);
            }
            *list = kept;
        }
    }

    /// Troca no documento os absolutos de outra máquina achados pela cauda
    /// (ver [`Project::resolve_path`]) pelo caminho de hoje, na forma em que
    /// a AURORA grava. Chamado antes de cada gravação: o `.spf` sai consertado
    /// na primeira mudança, e abrir continua só lendo.
    pub(crate) fn repair_rescued(&mut self) {
        let root = self.root.clone();
        let Some(structure) = self.document.get_mut("structure") else {
            return;
        };
        for role in [FileRole::Synthesizable, FileRole::Testbench] {
            if let Some(list) = structure
                .get_mut(role.list_key())
                .and_then(Value::as_array_mut)
            {
                for entry in list.iter_mut().filter(|e| e.is_object()) {
                    let Some(found) = entry_path(entry).and_then(|p| rescued(&root, p)) else {
                        continue;
                    };
                    entry["path"] = store(&root, &found).into();
                    entry["name"] = found.file_name().unwrap_or_default().into();
                }
            }
            let key = role.selected_key();
            if let Some(found) = structure
                .get(key)
                .and_then(Value::as_str)
                .and_then(|p| rescued(&root, p))
            {
                structure[key] = store(&root, &found).into();
            }
        }
    }

    /// Registra um arquivo no projeto e grava o `.spf`. Devolve o caminho
    /// absoluto.
    ///
    /// `path` pode ser relativo à raiz ou absoluto (inclusive fora do
    /// projeto). Com `contents`, cria o arquivo e os diretórios, recusando se
    /// ele já existir; com `None`, o arquivo precisa existir. Registrar de
    /// novo o mesmo caminho no mesmo papel não duplica a entrada. O `.spf`
    /// guarda o caminho relativo à raiz quando o arquivo está dentro dela, ou
    /// no mesmo repositório git que ela (com `..`), e absoluto quando não.
    ///
    /// # Erros
    ///
    /// - [`AuroraError::InvalidName`] se o arquivo não termina em `.v` ou `.sv`;
    /// - [`AuroraError::InvalidProject`] se `contents` foi dado e o arquivo já
    ///   existe, ou se não foi dado e o arquivo não existe.
    ///
    /// Em todos, nada foi gravado.
    pub fn add_file(
        &mut self,
        role: FileRole,
        path: impl AsRef<Utf8Path>,
        contents: Option<&str>,
    ) -> Result<Utf8PathBuf> {
        self.begin_write()?;
        let path = self.absolute(path.as_ref());
        check_extension(&path, Some(role))?;
        match contents {
            Some(text) => {
                if path.exists() {
                    return Err(AuroraError::InvalidProject {
                        path,
                        reason: "The file already exists; AURORA does not overwrite it".into(),
                    });
                }
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(AuroraError::io("Creating directory", parent))?;
                }
                std::fs::write(&path, text).map_err(AuroraError::io("Creating file", &path))?;
            }
            None if !path.is_file() => {
                return Err(AuroraError::InvalidProject {
                    path,
                    reason: "The file does not exist".into(),
                });
            }
            None => {}
        }

        // Um arquivo fica numa lista só: registrar como testbench tira dos
        // sintetizáveis, e vice-versa (como o "Set as Top Level" da AURORA).
        let other = match role {
            FileRole::Synthesizable => FileRole::Testbench,
            FileRole::Testbench => FileRole::Synthesizable,
        };
        if self.files(other).iter().any(|f| f.path == path) {
            self.remove_file(other, &path)?;
        }
        if !self.files(role).iter().any(|f| f.path == path) {
            let entry = json!({
                "name": path.file_name().unwrap_or_default(),
                "path": self.stored_path(&path),
                "isTopLevel": false,
            });
            self.list_mut(role).push(entry);
            self.save()?;
        }
        tracing::info!(%path, ?role, "File added");
        Ok(path)
    }

    /// Muda a posição de um arquivo registrado na lista dele e grava o
    /// `.spf`. A ordem da lista é a ordem em que os compiladores leem os
    /// arquivos: um `` `define `` só vale para os que vêm depois dele.
    /// Devolve a lista na ordem nova.
    ///
    /// ```
    /// use aurora_core::{FileRole, ListPosition, Project};
    /// # let tmp = tempfile::tempdir()?;
    /// # let dir = camino::Utf8Path::from_path(tmp.path()).unwrap();
    /// let mut project = Project::create(dir, "p")?;
    /// project.add_file(FileRole::Synthesizable, "alu.v", Some("module alu; endmodule\n"))?;
    /// project.add_file(FileRole::Synthesizable, "defs.v", Some("`define N 8\n"))?;
    /// let files = project.reorder_file("defs.v", &ListPosition::First)?;
    /// assert!(files[0].path.ends_with("defs.v"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Erros
    ///
    /// [`AuroraError::InvalidProject`] se o arquivo não está registrado, ou se
    /// o de referência (`Before`, `After`) não está na mesma lista. Nada é
    /// gravado.
    pub fn reorder_file(
        &mut self,
        path: impl AsRef<Utf8Path>,
        position: &ListPosition,
    ) -> Result<Vec<ProjectFile>> {
        self.begin_write()?;
        let path = self.absolute(path.as_ref());
        let root = self.root.clone();
        let index_in = |list: &[Value], wanted: &Utf8Path| {
            list.iter()
                .position(|e| entry_path(e).is_some_and(|p| resolve(&root, p) == wanted))
        };
        let role = [FileRole::Synthesizable, FileRole::Testbench]
            .into_iter()
            .find(|role| self.files(*role).iter().any(|f| f.path == path))
            .ok_or_else(|| AuroraError::InvalidProject {
                path: path.clone(),
                reason: "The file is not in the project".into(),
            })?;
        let anchor = match position {
            ListPosition::Before(other) | ListPosition::After(other) => Some(self.absolute(other)),
            _ => None,
        };
        let list = self.list_mut(role);
        let from = index_in(list, &path).expect("o arquivo está nesta lista");
        let entry = list.remove(from);
        let to = match (position, &anchor) {
            (ListPosition::First, _) => 0,
            (ListPosition::Last, _) => list.len(),
            (ListPosition::Before(_), Some(anchor)) | (ListPosition::After(_), Some(anchor)) => {
                let Some(at) = index_in(list, anchor) else {
                    list.insert(from, entry);
                    return Err(AuroraError::InvalidProject {
                        path: anchor.clone(),
                        reason: format!("Not in {} with {}", role.list_key(), path),
                    });
                };
                if matches!(position, ListPosition::After(_)) {
                    at + 1
                } else {
                    at
                }
            }
            _ => list.len(),
        };
        list.insert(to, entry);
        self.save()?;
        Ok(self.files(role))
    }

    /// Tira o arquivo do projeto (não apaga do disco). Diz se ele estava lá.
    pub fn remove_file(&mut self, role: FileRole, path: impl AsRef<Utf8Path>) -> Result<bool> {
        self.begin_write()?;
        let path = self.absolute(path.as_ref());
        let removed = self.unlist(role, &path);
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// Tira `path` (absoluto) da lista do papel e desfaz a escolha de topo ou
    /// de testbench que apontava para ele, sem gravar. Diz se ele estava na
    /// lista. Quem chama já tem a trava de [`Project::begin_write`].
    pub(crate) fn unlist(&mut self, role: FileRole, path: &Utf8Path) -> bool {
        let root = self.root.clone();
        let list = self.list_mut(role);
        let before = list.len();
        list.retain(|entry| {
            entry_path(entry)
                .map(|p| resolve(&root, p))
                .is_none_or(|p| p.as_path() != path)
        });
        let removed = list.len() != before;
        let selected = self.document["structure"][role.selected_key()]
            .as_str()
            .map(|p| self.resolve_path(p));
        if selected.as_deref() == Some(path) {
            self.document["structure"][role.selected_key()] = "".into();
            if role == FileRole::Synthesizable
                && let Some(structure) = self.document["structure"].as_object_mut()
            {
                structure.remove(TOP_MODULE_KEY);
            }
        }
        removed
    }

    /// Marca o módulo de topo do projeto: grava `topLevelFile` e deixa só este
    /// arquivo com `isTopLevel` entre os sintetizáveis. O arquivo precisa
    /// existir; é registrado como sintetizável se ainda não estiver (um
    /// testbench registrado muda de lista). O nome do módulo de topo é o nome
    /// do arquivo sem extensão.
    ///
    /// Qualquer Verilog pode ser o topo, inclusive o `<nome>.v` que
    /// o build de um processador gerou (registrá-lo não o duplica no design:
    /// as operações tiram os repetidos). A exceção é o nome de testbench
    /// ([`is_testbench_name`](crate::verilog::is_testbench_name)):
    /// `tb_<nome>.v`, `<nome>_tb.v`, `tb.v`.
    ///
    /// # Erros
    ///
    /// [`AuroraError::InvalidName`] com nome de testbench (nada muda);
    /// [`AuroraError::InvalidProject`] se o arquivo não existe.
    pub fn set_top_level(&mut self, path: impl AsRef<Utf8Path>) -> Result<()> {
        self.begin_write()?;
        let path = self.absolute(path.as_ref());
        let file_name = path.file_name().unwrap_or_default();
        if crate::verilog::is_testbench_name(file_name) {
            return Err(AuroraError::InvalidName {
                name: file_name.to_owned(),
                reason: "A testbench (tb_<name>.v, <name>_tb.v) cannot be the top level".into(),
            });
        }
        // Um módulo escolhido pelo nome (`topLevelModule`) valia para o
        // arquivo anterior.
        if let Some(structure) = self.document["structure"].as_object_mut() {
            structure.remove(TOP_MODULE_KEY);
        }
        self.select(FileRole::Synthesizable, &path)
    }

    /// Escolhe o testbench de [`simulate_project`](crate::simulate_project):
    /// grava `testbenchFile` e deixa só este com `isTopLevel` entre os
    /// testbenches. O arquivo precisa existir; é registrado se ainda não
    /// estiver.
    pub fn set_testbench(&mut self, path: impl AsRef<Utf8Path>) -> Result<()> {
        self.begin_write()?;
        self.select(FileRole::Testbench, path.as_ref())
    }

    fn select(&mut self, role: FileRole, path: &Utf8Path) -> Result<()> {
        let path = self.add_file(role, path, None)?;
        let stored = self.stored_path(&path);
        let root = self.root.clone();
        for entry in self.list_mut(role).iter_mut().filter(|e| e.is_object()) {
            let is_it = entry_path(entry).map(|p| resolve(&root, p)) == Some(path.clone());
            entry["isTopLevel"] = is_it.into();
            // A marca legada da AURORA vira `isTopLevel`, como no próximo
            // save dela; sem isso, a marca velha escolheria o testbench antigo.
            if let Some(object) = entry.as_object_mut() {
                object.remove("isMarkedTestbench");
            }
        }
        self.document["structure"][role.selected_key()] = stored.into();
        self.save()
    }

    fn list_mut(&mut self, role: FileRole) -> &mut Vec<Value> {
        let structure = &mut self.document["structure"];
        if !structure[role.list_key()].is_array() {
            structure[role.list_key()] = json!([]);
        }
        structure[role.list_key()]
            .as_array_mut()
            .expect("acabou de virar array")
    }

    /// Move ou renomeia um arquivo ou uma pasta dentro do projeto e atualiza
    /// o `.spf`, como arrastar na árvore de arquivos de uma IDE.
    ///
    /// `to` é o caminho final (não a pasta de destino): `rtl/a.v` para
    /// `src/a.v` move, `a.v` para `b.v` renomeia. Os dois são absolutos ou
    /// relativos à raiz. As pastas que faltam no caminho de `to` são criadas.
    ///
    /// Os arquivos Verilog registrados que mudam de lugar (o próprio `from`,
    /// ou os que estão dentro dele) continuam registrados no lugar novo: o
    /// `.spf` troca o caminho e o nome de cada entrada, na mesma posição da
    /// lista, e `topLevelFile`, `testbenchFile` e o módulo de topo escolhido
    /// pelo nome acompanham. Se o `.spf` não puder ser gravado, o movimento é
    /// desfeito antes do erro.
    ///
    /// Ficam onde estão, porque o Core os acha pelo lugar: o `.spf`, a pasta
    /// `.aurora8`, e de cada processador a pasta, a de saída e o fonte. Os
    /// outros arquivos de dentro delas (entradas, saídas, gerados) podem ser
    /// movidos.
    ///
    /// ```
    /// use aurora_core::{FileRole, Project};
    /// # let tmp = tempfile::tempdir()?;
    /// # let dir = camino::Utf8Path::from_path(tmp.path()).unwrap();
    ///
    /// let mut project = Project::create(dir, "p")?;
    /// project.add_file(FileRole::Synthesizable, "top.v", Some("module top; endmodule\n"))?;
    /// project.set_top_level("top.v")?;
    ///
    /// let moved = project.move_path("top.v", "rtl/top.v")?;
    /// assert_eq!(project.top_level(), Some(moved.to.clone()));
    /// assert_eq!(moved.files.len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Erros
    ///
    /// - [`AuroraError::OutsideProject`] se `from` ou `to` estiver fora da
    ///   pasta do projeto;
    /// - [`AuroraError::CannotMove`] para o que fica no lugar (acima), para uma
    ///   pasta que iria para dentro dela mesma e para um destino dentro de
    ///   `.aurora8`;
    /// - [`AuroraError::PathExists`] se `to` já existe, inclusive como link
    ///   quebrado (renomear só a caixa, `a.v` para `A.v`, vale também num
    ///   sistema que não distingue maiúsculas);
    /// - [`AuroraError::InvalidProject`] se `from` não existe;
    /// - [`AuroraError::Io`] se o sistema recusar.
    ///
    /// Em todos, nada foi alterado.
    pub fn move_path(
        &mut self,
        from: impl AsRef<Utf8Path>,
        to: impl AsRef<Utf8Path>,
    ) -> Result<MovedPath> {
        self.begin_write()?;
        let from = self.absolute(from.as_ref());
        // O destino com o nome que foi pedido: resolvido inteiro, `A.v` num
        // sistema que ignora a caixa viraria o `a.v` que já existe.
        let to = self.absolute_parent(to.as_ref());
        for path in [&from, &to] {
            if path.strip_prefix(&self.root).is_err() {
                return Err(AuroraError::OutsideProject(path.clone()));
            }
        }
        let refuse = |reason: &str| AuroraError::CannotMove {
            path: from.clone(),
            reason: reason.into(),
        };
        if from == self.root {
            return Err(refuse("It is the project folder"));
        }
        match std::fs::symlink_metadata(&from) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(AuroraError::InvalidProject {
                    path: from,
                    reason: "Does not exist".into(),
                });
            }
            Err(e) => return Err(AuroraError::io("Moving", &from)(e)),
        }
        if let Some(reason) = self.fixed_place(&from) {
            return Err(refuse(reason));
        }
        if to == from {
            return Ok(MovedPath {
                from,
                to,
                files: Vec::new(),
            });
        }
        if to.starts_with(&from) {
            return Err(refuse("A folder cannot go inside itself"));
        }
        if to.starts_with(self.root.join(".aurora8")) {
            return Err(refuse("Nothing goes into the .aurora8 folder"));
        }
        // `symlink_metadata`, e não `exists`: um link quebrado no destino
        // também ocupa o nome. A exceção é o próprio arquivo com outra caixa
        // (`a.v` para `A.v`) num sistema que não distingue maiúsculas.
        let case_only = from.as_str().eq_ignore_ascii_case(to.as_str())
            && crate::paths::canonicalize(&to).is_ok_and(|real| real == from);
        if std::fs::symlink_metadata(&to).is_ok() && !case_only {
            return Err(AuroraError::PathExists(to));
        }

        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)
                .map_err(AuroraError::io("Creating directory", parent))?;
        }
        rename_or_copy(&from, &to)?;
        let changed = self.rewrite_moved(&from, &to);
        if changed && let Err(error) = self.save() {
            // O disco volta a bater com o .spf, que não mudou.
            if rename_or_copy(&to, &from).is_ok() {
                self.rewrite_moved(&to, &from);
            }
            return Err(error);
        }
        let files = [FileRole::Synthesizable, FileRole::Testbench]
            .into_iter()
            .flat_map(|role| self.files(role))
            .filter(|f| f.path.starts_with(&to))
            .collect();
        tracing::info!(%from, %to, "Moved");
        Ok(MovedPath { from, to, files })
    }

    /// Registra no `.spf` um movimento que já aconteceu fora da AURORA: o
    /// arquivo ou a pasta foi renomeado no explorador de uma IDE, ou com o
    /// `mv`, e os registrados que estavam em `from` passam a estar em `to`.
    /// É o [`move_path`](Self::move_path) sem mexer no disco, como o
    /// `hg mv --after`.
    ///
    /// Valem as mesmas regras do `move_path` (dentro do projeto, nada do que
    /// fica no lugar, nada para dentro de `.aurora8`), e o disco tem que bater
    /// com o movimento: `from` não existe mais e `to` existe. Se o `.spf`
    /// não puder ser gravado, ele fica como estava.
    ///
    /// ```
    /// use aurora_core::{FileRole, Project};
    /// # let tmp = tempfile::tempdir()?;
    /// # let dir = camino::Utf8Path::from_path(tmp.path()).unwrap();
    ///
    /// let mut project = Project::create(dir, "p")?;
    /// project.add_file(FileRole::Synthesizable, "top.v", Some("module top; endmodule\n"))?;
    /// let root = project.root().to_owned();
    /// std::fs::rename(root.join("top.v"), root.join("cpu.v"))?;
    ///
    /// let moved = project.record_move("top.v", "cpu.v")?;
    /// assert_eq!(moved.files[0].path, root.join("cpu.v"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Erros
    ///
    /// - [`AuroraError::OutsideProject`] se `from` ou `to` estiver fora da
    ///   pasta do projeto;
    /// - [`AuroraError::CannotMove`] para o que fica no lugar, para uma pasta
    ///   que teria ido para dentro dela mesma, para um destino dentro de
    ///   `.aurora8` e quando `from` ainda existe (o movimento não aconteceu);
    /// - [`AuroraError::InvalidProject`] se `to` não existe;
    /// - [`AuroraError::Io`] se o `.spf` não puder ser gravado.
    ///
    /// Em todos, o `.spf` não mudou.
    pub fn record_move(
        &mut self,
        from: impl AsRef<Utf8Path>,
        to: impl AsRef<Utf8Path>,
    ) -> Result<MovedPath> {
        self.begin_write()?;
        // Os dois com o nome que foi pedido: `a.v` para `A.v`, num sistema
        // que ignora a caixa, resolveria os dois para o mesmo arquivo.
        let from = self.absolute_parent(from.as_ref());
        let to = self.absolute_parent(to.as_ref());
        for path in [&from, &to] {
            if path.strip_prefix(&self.root).is_err() {
                return Err(AuroraError::OutsideProject(path.clone()));
            }
        }
        let refuse = |reason: &str| AuroraError::CannotMove {
            path: from.clone(),
            reason: reason.into(),
        };
        if from == self.root {
            return Err(refuse("It is the project folder"));
        }
        if let Some(reason) = self.fixed_place(&from) {
            return Err(refuse(reason));
        }
        if to == from {
            return Ok(MovedPath {
                from,
                to,
                files: Vec::new(),
            });
        }
        if to.starts_with(&from) {
            return Err(refuse("A folder cannot go inside itself"));
        }
        if to.starts_with(self.root.join(".aurora8")) {
            return Err(refuse("Nothing goes into the .aurora8 folder"));
        }
        if std::fs::symlink_metadata(&to).is_err() {
            return Err(AuroraError::InvalidProject {
                path: to,
                reason: "Does not exist".into(),
            });
        }
        // A exceção é a mesma do `move_path`: só a caixa mudou, e `from` ainda
        // "existe" porque o sistema não distingue maiúsculas.
        let case_only = from.as_str().eq_ignore_ascii_case(to.as_str())
            && crate::paths::canonicalize(&from).is_ok_and(|real| real == to);
        if std::fs::symlink_metadata(&from).is_ok() && !case_only {
            return Err(refuse(
                "It is still there: move it first, or move it with AURORA",
            ));
        }

        if self.rewrite_moved(&from, &to)
            && let Err(error) = self.save()
        {
            self.rewrite_moved(&to, &from);
            return Err(error);
        }
        let files = [FileRole::Synthesizable, FileRole::Testbench]
            .into_iter()
            .flat_map(|role| self.files(role))
            .filter(|f| f.path.starts_with(&to))
            .collect();
        tracing::info!(%from, %to, "Recorded move");
        Ok(MovedPath { from, to, files })
    }

    /// Por que o que está em `target` não pode sair para dar lugar a `from`
    /// num [`move_path`](Self::move_path) que encontrou o destino ocupado
    /// ([`AuroraError::PathExists`]), ou `None` se pode. É a pergunta de quem
    /// oferece substituir o destino (o Studio, que o manda para a lixeira):
    /// o Core nunca sobrescreve, e protege o que fica no lugar só como origem.
    ///
    /// Não sai a pasta do projeto, o que fica no lugar (o `.spf`, a pasta
    /// `.aurora8`, a pasta, a pasta de saída e o fonte de um processador), nem
    /// uma pasta que contém algum deles ou a própria origem.
    ///
    /// ```
    /// use aurora_core::Project;
    /// # let tmp = tempfile::tempdir()?;
    /// # let dir = camino::Utf8Path::from_path(tmp.path()).unwrap();
    ///
    /// let project = Project::create(dir, "p")?;
    /// std::fs::write(dir.join("p/a.v"), "")?;
    /// assert!(project.replace_refusal("a.v", "p.spf").is_some());
    /// std::fs::write(dir.join("p/b.v"), "")?;
    /// assert_eq!(project.replace_refusal("a.v", "b.v"), None);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn replace_refusal(
        &self,
        from: impl AsRef<Utf8Path>,
        target: impl AsRef<Utf8Path>,
    ) -> Option<&'static str> {
        let from = self.absolute(from.as_ref());
        let target = self.absolute_parent(target.as_ref());
        if target == self.root || self.root.starts_with(&target) {
            return Some("It is the project folder");
        }
        if let Some(reason) = self.fixed_place(&target) {
            return Some(reason);
        }
        if from.starts_with(&target) {
            return Some("It holds what is being moved");
        }
        let held = [self.spf_path().to_owned(), self.root.join(".aurora8")]
            .into_iter()
            .chain(
                self.processors()
                    .iter()
                    .flat_map(|p| [p.dir.clone(), p.output_dir.clone(), p.source.clone()]),
            )
            .any(|kept| kept.starts_with(&target));
        held.then_some("It holds what stays in place: the project file, .aurora8 or a processor")
    }

    /// Por que `path` não sai do lugar, ou `None` se pode sair.
    fn fixed_place(&self, path: &Utf8Path) -> Option<&'static str> {
        if path == self.spf_path() {
            return Some("The project file stays in the project folder");
        }
        if path.starts_with(self.root.join(".aurora8")) {
            return Some("The .aurora8 folder belongs to AURORA");
        }
        for processor in self.processors() {
            if path == processor.dir {
                return Some("A processor folder is found by the processor name");
            }
            if path == processor.output_dir {
                return Some("The output folder of a processor is where YANC writes it");
            }
            if path == processor.source {
                return Some("The processor source is found by the processor name");
            }
        }
        None
    }

    /// Troca no documento os caminhos que estavam em `from` (ou dentro dele)
    /// pelos de `to`. Diz se algo mudou; não grava.
    pub(crate) fn rewrite_moved(&mut self, from: &Utf8Path, to: &Utf8Path) -> bool {
        let root = self.root.clone();
        let moved = |stored: &str| -> Option<Utf8PathBuf> {
            let path = resolve(&root, stored);
            let rel = path.strip_prefix(from).ok()?;
            Some(if rel.as_str().is_empty() {
                to.to_owned()
            } else {
                to.join(rel)
            })
        };
        // `get_mut`, e não `[]`: indexar cria no documento a chave que falta.
        let Some(structure) = self.document.get_mut("structure") else {
            return false;
        };
        let mut changed = false;
        for role in [FileRole::Synthesizable, FileRole::Testbench] {
            if let Some(list) = structure
                .get_mut(role.list_key())
                .and_then(Value::as_array_mut)
            {
                for entry in list {
                    let Some(new) = entry.get("path").and_then(Value::as_str).and_then(moved)
                    else {
                        continue;
                    };
                    entry["path"] = store(&root, &new).into();
                    entry["name"] = new.file_name().unwrap_or_default().into();
                    changed = true;
                }
            }
            let key = role.selected_key();
            if let Some(new) = structure.get(key).and_then(Value::as_str).and_then(moved) {
                structure[key] = store(&root, &new).into();
                changed = true;
            }
        }
        changed
    }

    /// Caminho como está gravado num `.spf` (relativo à raiz, com ou sem
    /// `..`, ou absoluto, com `\` ou `/`) para absoluto. Um absoluto que não
    /// existe aqui (de outra máquina, `C:\...` num `.spf` aberto no Linux) é
    /// procurado pela cauda dentro da raiz, como a AURORA faz
    /// (`resgatarPelaCauda`): `C:\velho\proj\rtl\x.v` acha `<raiz>/rtl/x.v`.
    /// Sem achar, fica como está.
    pub fn resolve_path(&self, stored: &str) -> Utf8PathBuf {
        resolve(&self.root, stored)
    }

    fn absolute(&self, path: &Utf8Path) -> Utf8PathBuf {
        let joined = if path.is_absolute() {
            path.to_owned()
        } else {
            self.root.join(path)
        };
        let normalized = crate::paths::normalize(&joined);
        // Com o caminho real (symlinks resolvidos), o mesmo arquivo dado por
        // dois caminhos fica com um registro só.
        if normalized.exists() {
            return crate::paths::canonicalize(&normalized).unwrap_or(normalized);
        }
        match (normalized.parent(), normalized.file_name()) {
            (Some(parent), Some(name)) if parent.is_dir() => crate::paths::canonicalize(parent)
                .map(|p| p.join(name))
                .unwrap_or(normalized),
            _ => normalized,
        }
    }

    /// Como [`Project::absolute`], mas só a pasta é resolvida: o nome fica
    /// como foi dado (um link continua sendo o link, e a caixa pedida fica).
    fn absolute_parent(&self, path: &Utf8Path) -> Utf8PathBuf {
        let joined = if path.is_absolute() {
            path.to_owned()
        } else {
            self.root.join(path)
        };
        let normalized = crate::paths::normalize(&joined);
        match (normalized.parent(), normalized.file_name()) {
            (Some(parent), Some(name)) if parent.is_dir() => crate::paths::canonicalize(parent)
                .map(|p| p.join(name))
                .unwrap_or(normalized),
            _ => normalized,
        }
    }

    fn stored_path(&self, path: &Utf8Path) -> String {
        store(&self.root, path)
    }
}

/// Campo do `.spf` (da AURORA, fora do formato da AURORA) com o módulo de
/// topo escolhido pelo nome, para arquivos com vários módulos.
const TOP_MODULE_KEY: &str = "topLevelModule";

/// O módulo de um arquivo: o único que ele declara, ou o que tem o nome dele.
/// Num VHDL, a entidade, pela mesma regra, sem diferenciar maiúsculas.
pub(crate) fn module_of(file: &Utf8Path) -> Result<String> {
    let text = std::fs::read_to_string(file).map_err(AuroraError::io("Reading", file))?;
    let modules = units_in(file, &text);
    let stem = file.file_stem().unwrap_or_default();
    if let [only] = modules.as_slice() {
        return Ok(only.clone());
    }
    match modules.iter().find(|m| same_unit(file, m, stem)) {
        Some(named) => Ok(named.clone()),
        None => Err(AuroraError::ModuleNotFound {
            name: stem.to_owned(),
            available: modules,
        }),
    }
}

/// Os módulos (Verilog) ou as entidades (VHDL) que `file` declara, em
/// `text`, o conteúdo dele.
pub(crate) fn units_in(file: &Utf8Path, text: &str) -> Vec<String> {
    if crate::hdl::is_vhdl(file) {
        crate::vhdl::entities_in(text)
    } else {
        crate::verilog::modules_in(text)
    }
}

/// `a` e `b` são o mesmo nome de unidade na linguagem de `file`: iguais no
/// Verilog, iguais sem diferenciar maiúsculas no VHDL.
pub(crate) fn same_unit(file: &Utf8Path, a: &str, b: &str) -> bool {
    if crate::hdl::is_vhdl(file) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

/// O nome que um testbench `stem` testa pelo nome dele: `and` em `and_tb`,
/// `and_test`, `and_testbench`, `tb_and` e `test_and`.
fn tested_name(stem: &str) -> Option<&str> {
    let lower = stem.to_ascii_lowercase();
    ["_testbench", "_tb", "_test"]
        .iter()
        .find_map(|suffix| {
            lower
                .strip_suffix(suffix)
                .map(|_| &stem[..stem.len() - suffix.len()])
        })
        .or_else(|| {
            ["tb_", "test_"]
                .iter()
                .find_map(|prefix| lower.strip_prefix(prefix).map(|_| &stem[prefix.len()..]))
        })
}

/// Como o `.spf` guarda `path` (absoluto e canônico): relativo à raiz
/// quando dentro dela, como a AURORA; relativo com `..` quando fora dela mas
/// no mesmo repositório git; absoluto no resto. Sempre com `/`.
fn store(root: &Utf8Path, path: &Utf8Path) -> String {
    match path.strip_prefix(root) {
        Ok(rel) => rel.as_str().replace('\\', "/"),
        Err(_) => crate::paths::relative_in_repository(root, path)
            .unwrap_or_else(|| path.as_str().to_owned()),
    }
}

/// O que entra nas listas do `.spf`: `.v`, `.sv`, `.vhd` e `.vhdl`, e `.py`
/// (testbench cocotb) fora dos sintetizáveis. `role` `None`: o papel ainda
/// não foi decidido.
fn check_extension(path: &Utf8Path, role: Option<FileRole>) -> Result<()> {
    let refuse = |reason: &str| {
        Err(AuroraError::InvalidName {
            name: path.file_name().unwrap_or_default().to_owned(),
            reason: reason.into(),
        })
    };
    if crate::hdl::is_verilog(path) || crate::hdl::is_vhdl(path) {
        Ok(())
    } else if crate::cocotb::is_testbench(path) {
        if role == Some(FileRole::Synthesizable) {
            refuse(
                "A Python file is a cocotb testbench; it cannot be synthesizable or the top level",
            )
        } else {
            Ok(())
        }
    } else {
        refuse(
            "A design file must end in .v or .sv (Verilog) or .vhd or .vhdl (VHDL); a cocotb testbench, in .py",
        )
    }
}

/// A lista de um arquivo que está nas duas: testbench se é `.py` ou se o
/// nome indica (`_tb`, `tb_`, `test`), sintetizável senão.
fn owner(path: &Utf8Path) -> FileRole {
    if crate::cocotb::is_testbench(path)
        || crate::verilog::name_suggests_testbench(path.file_name().unwrap_or_default())
    {
        FileRole::Testbench
    } else {
        FileRole::Synthesizable
    }
}

/// A marca de topo (ou de testbench escolhido) de uma entrada: `isTopLevel`
/// ou o `isMarkedTestbench` legado, que a AURORA lê como igual
/// (`js/project/file_mode.js`).
fn is_marked(entry: &Value) -> bool {
    ["isTopLevel", "isMarkedTestbench"]
        .iter()
        .any(|key| crate::spf::boolean(entry.get(*key)).unwrap_or(false))
}

/// O caminho gravado numa entrada das listas, se houver.
fn entry_path(entry: &Value) -> Option<&str> {
    entry
        .get("path")
        .and_then(Value::as_str)
        .filter(|p| !p.trim().is_empty())
}

/// O caminho de hoje de um absoluto gravado que não existe aqui, achado
/// pela cauda dentro da raiz; `None` se ele existe, é relativo, ou não foi
/// achado.
fn rescued(root: &Utf8Path, stored: &str) -> Option<Utf8PathBuf> {
    let rooted = crate::paths::is_windows_absolute(stored) || Utf8Path::new(stored).has_root();
    if !rooted || resolve_plain(root, stored).exists() {
        return None;
    }
    crate::paths::rescue_by_tail(root, stored)
}

/// O módulo de um testbench: o de [`module_of`] ou, com vários e nenhum
/// com o nome do arquivo, a única raiz do arquivo (o módulo que nenhum
/// outro dele instancia).
pub(crate) fn testbench_module_of(file: &Utf8Path) -> Result<String> {
    module_of(file).or_else(|error| {
        let text = std::fs::read_to_string(file).map_err(AuroraError::io("Reading", file))?;
        if crate::hdl::is_vhdl(file) {
            return match crate::vhdl::design_roots(&[&text]).as_slice() {
                [one] => Ok(one.clone()),
                _ => Err(error),
            };
        }
        match crate::verilog::design_roots(&[text], None) {
            roots if !roots.explicit && roots.names.len() == 1 => {
                Ok(roots.names.into_iter().next().expect("um"))
            }
            _ => Err(error),
        }
    })
}

fn resolve(root: &Utf8Path, stored: &str) -> Utf8PathBuf {
    rescued(root, stored).unwrap_or_else(|| resolve_plain(root, stored))
}

/// [`resolve`] sem o resgate pela cauda.
fn resolve_plain(root: &Utf8Path, stored: &str) -> Utf8PathBuf {
    // `C:\x` num .spf aberto fora do Windows não é absoluto aqui, mas também
    // não faz sentido juntar à raiz; fica como está.
    if crate::paths::is_windows_absolute(stored) {
        return Utf8PathBuf::from(stored);
    }
    let normalized = if cfg!(windows) {
        stored.to_owned()
    } else {
        stored.replace('\\', "/")
    };
    let path = Utf8PathBuf::from(normalized);
    if path.is_absolute() {
        crate::paths::normalize(&path)
    } else {
        crate::paths::normalize(&root.join(path))
    }
}

/// `std::fs::rename`, e, entre discos diferentes (onde ele não funciona),
/// copiar e apagar o original.
fn rename_or_copy(from: &Utf8Path, to: &Utf8Path) -> Result<()> {
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {
            copy_tree(from, to)?;
            let removed = if from.is_dir() {
                std::fs::remove_dir_all(from)
            } else {
                std::fs::remove_file(from)
            };
            removed.map_err(AuroraError::io("Removing after copy", from))
        }
        Err(e) => Err(AuroraError::io("Moving", from)(e)),
    }
}

fn copy_tree(from: &Utf8Path, to: &Utf8Path) -> Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to).map_err(AuroraError::io("Creating directory", to))?;
        for entry in from
            .read_dir_utf8()
            .map_err(AuroraError::io("Listing directory", from))?
        {
            let entry = entry.map_err(AuroraError::io("Listing directory", from))?;
            copy_tree(entry.path(), &to.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(from, to)
            .map(|_| ())
            .map_err(AuroraError::io("Copying", from))
    }
}

impl Processor {
    /// `<proc>/input_<porta>.txt`, ao lado do fonte: um valor decimal por
    /// linha, lido pelo testbench com `$fscanf(..., "%d", ...)` quando o
    /// processador lê a porta.
    pub fn input_path(&self, port: u32) -> Utf8PathBuf {
        self.dir.join(format!("input_{port}.txt"))
    }

    /// `<proc>/<proc>/output_<porta>.txt`, na pasta de saída: um valor
    /// decimal por linha, escrito pelo testbench a cada `out()` do programa.
    pub fn output_path(&self, port: u32) -> Utf8PathBuf {
        self.output_dir.join(format!("output_{port}.txt"))
    }

    /// Grava o arquivo de entrada de uma porta (substitui o anterior: é dado
    /// de estímulo, não código).
    pub fn write_input(&self, port: u32, text: &str) -> Result<Utf8PathBuf> {
        let path = self.input_path(port);
        std::fs::create_dir_all(&self.dir)
            .map_err(AuroraError::io("Creating directory", &self.dir))?;
        std::fs::write(&path, text).map_err(AuroraError::io("Writing input", &path))?;
        Ok(path)
    }

    /// Grava a entrada de uma porta a partir de valores, um por linha, no
    /// formato que o testbench lê (`%d`).
    ///
    /// ```
    /// # use aurora_core::{Language, NewProcessor, Project, read_data_file};
    /// # let tmp = tempfile::tempdir()?;
    /// # let dir = camino::Utf8Path::from_path(tmp.path()).unwrap();
    /// # let mut project = Project::create(dir, "p")?;
    /// let soma = project.add_processor(&NewProcessor::new("soma", Language::Cmm))?;
    /// let path = soma.write_input_values(0, &[5, -7, 21])?;
    /// assert_eq!(std::fs::read_to_string(&path)?, "5\n-7\n21\n");
    /// assert_eq!(read_data_file(&path)?, [5, -7, 21]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn write_input_values(&self, port: u32, values: &[i64]) -> Result<Utf8PathBuf> {
        let text: String = values.iter().map(|v| format!("{v}\n")).collect();
        self.write_input(port, &text)
    }

    /// Lê o que a simulação escreveu numa porta de saída, como texto.
    pub fn read_output(&self, port: u32) -> Result<String> {
        let path = self.output_path(port);
        std::fs::read_to_string(&path).map_err(AuroraError::io("Reading output", &path))
    }

    /// Lê a saída de uma porta como valores ([`read_data_file`]).
    pub fn read_output_values(&self, port: u32) -> Result<Vec<i64>> {
        read_data_file(&self.output_path(port))
    }
}

/// Lê um arquivo de dados da simulação (`input_<n>.txt`, `output_<n>.txt`):
/// um inteiro decimal com sinal por linha, que é o que o testbench gerado lê
/// com `%d` e escreve com `%0d`. Linhas vazias são ignoradas.
///
/// # Erros
///
/// - [`AuroraError::InvalidDataFile`] na primeira linha que não for um inteiro
///   (a linha nunca é descartada em silêncio: um valor perdido muda o
///   resultado da simulação);
/// - [`AuroraError::Io`] se o arquivo não puder ser lido.
pub fn read_data_file(path: &Utf8Path) -> Result<Vec<i64>> {
    let text = std::fs::read_to_string(path).map_err(AuroraError::io("Reading data", path))?;
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(i, line)| {
            line.trim()
                .parse::<i64>()
                .map_err(|_| AuroraError::InvalidDataFile {
                    path: path.to_owned(),
                    line: u32::try_from(i + 1).unwrap_or(u32::MAX),
                    reason: format!("Not a decimal integer: '{}'", line.trim()),
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Language, NewProcessor};

    fn project() -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        let parent = Utf8PathBuf::from_path_buf(dunce::canonicalize(dir.path()).unwrap()).unwrap();
        let project = Project::create(&parent, "p").unwrap();
        (dir, project)
    }

    #[test]
    fn add_select_and_reopen() {
        let (_guard, mut project) = project();
        let top = project
            .add_file(
                FileRole::Synthesizable,
                "top/top.v",
                Some("module top; endmodule\n"),
            )
            .unwrap();
        let tb = project
            .add_file(
                FileRole::Testbench,
                "top/top_tb.v",
                Some("module top_tb; endmodule\n"),
            )
            .unwrap();
        assert!(top.is_file());
        assert_eq!(project.testbench(), Some(tb.clone()), "primeiro da lista");

        project.set_top_level(&top).unwrap();
        project.set_testbench(&tb).unwrap();
        // registrar de novo não duplica
        project
            .add_file(FileRole::Synthesizable, &top, None)
            .unwrap();

        let reopened = Project::open(project.spf_path()).unwrap();
        assert_eq!(reopened.files(FileRole::Synthesizable).len(), 1);
        assert_eq!(reopened.top_level(), Some(top.clone()));
        assert_eq!(reopened.testbench(), Some(tb));
        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(reopened.spf_path()).unwrap()).unwrap();
        assert_eq!(doc["structure"]["topLevelFile"], "top/top.v");
        assert_eq!(
            doc["structure"]["synthesizableFiles"][0]["isTopLevel"],
            true
        );
    }

    #[test]
    fn never_overwrites_and_requires_existing() {
        let (_guard, mut project) = project();
        project
            .add_file(FileRole::Synthesizable, "a.v", Some("x"))
            .unwrap();
        assert!(
            project
                .add_file(FileRole::Synthesizable, "a.v", Some("y"))
                .is_err()
        );
        assert!(
            project
                .add_file(FileRole::Testbench, "nao_existe.v", None)
                .is_err()
        );
        assert!(project.remove_file(FileRole::Synthesizable, "a.v").unwrap());
        assert!(project.files(FileRole::Synthesizable).is_empty());
        assert!(
            project.root().join("a.v").is_file(),
            "remover do projeto não apaga do disco"
        );
    }

    #[test]
    fn windows_style_relative_paths_resolve() {
        let root = Utf8Path::new("/r");
        assert_eq!(
            resolve(root, "proc\\Hardware\\proc.v"),
            Utf8PathBuf::from("/r/proc/Hardware/proc.v")
        );
        assert_eq!(resolve(root, "C:\\x\\a.v"), Utf8PathBuf::from("C:\\x\\a.v"));
    }

    #[test]
    fn outside_the_root_is_relative_only_in_the_same_repository() {
        let dir = tempfile::tempdir().unwrap();
        let base = Utf8PathBuf::from_path_buf(dunce::canonicalize(dir.path()).unwrap()).unwrap();
        let repo = base.join("hits");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(repo.join("rtl")).unwrap();
        std::fs::create_dir_all(repo.join("projects")).unwrap();
        std::fs::write(repo.join("rtl/n.v"), "module n; endmodule\n").unwrap();
        std::fs::write(base.join("fora.v"), "module fora; endmodule\n").unwrap();
        let mut project = Project::create(repo.join("projects"), "sim").unwrap();
        project
            .add_file(FileRole::Synthesizable, repo.join("rtl/n.v"), None)
            .unwrap();
        project
            .add_file(FileRole::Synthesizable, base.join("fora.v"), None)
            .unwrap();
        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(project.spf_path()).unwrap()).unwrap();
        let list = &doc["structure"]["synthesizableFiles"];
        assert_eq!(list[0]["path"], "../../rtl/n.v");
        assert_eq!(list[1]["path"], base.join("fora.v").as_str());
        let reopened = Project::open(project.spf_path()).unwrap();
        assert_eq!(
            reopened.files(FileRole::Synthesizable)[0].path,
            repo.join("rtl/n.v")
        );
    }

    #[test]
    fn path_from_another_machine_is_rescued_by_its_tail() {
        let (_guard, project) = project();
        let root = project.root().to_owned();
        std::fs::create_dir_all(root.join("rtl")).unwrap();
        std::fs::write(root.join("rtl/alu.v"), "module alu; endmodule\n").unwrap();
        std::fs::write(root.join("novo.v"), "module novo; endmodule\n").unwrap();
        let spf = project.spf_path().to_owned();
        let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&spf).unwrap()).unwrap();
        doc["structure"]["synthesizableFiles"] = json!([
            {"name": "alu.v", "path": "C:\\Users\\aluno\\p\\rtl\\alu.v", "isTopLevel": true}
        ]);
        doc["structure"]["topLevelFile"] = "C:\\Users\\aluno\\p\\rtl\\alu.v".into();
        std::fs::write(&spf, serde_json::to_string(&doc).unwrap()).unwrap();

        let mut project = Project::open(&spf).unwrap();
        assert_eq!(project.top_level(), Some(root.join("rtl/alu.v")));
        let issues = project.issues();
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(issues[0].kind, crate::IssueKind::RescuedPath);
        // Abrir não regrava; a primeira mudança grava o caminho de hoje.
        assert!(std::fs::read_to_string(&spf).unwrap().contains("aluno"));
        project
            .add_file(FileRole::Synthesizable, "novo.v", None)
            .unwrap();
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&spf).unwrap()).unwrap();
        assert_eq!(
            doc["structure"]["synthesizableFiles"][0]["path"],
            "rtl/alu.v"
        );
        assert_eq!(doc["structure"]["topLevelFile"], "rtl/alu.v");
        assert!(Project::open(&spf).unwrap().issues().is_empty());
    }

    #[test]
    fn unregistered_or_testbench_top_is_ignored_with_a_warning() {
        let (_guard, project) = project();
        let root = project.root().to_owned();
        std::fs::write(root.join("a.v"), "module a; endmodule\n").unwrap();
        std::fs::write(root.join("a_tb.v"), "module a_tb; endmodule\n").unwrap();
        let spf = project.spf_path().to_owned();
        let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&spf).unwrap()).unwrap();
        doc["structure"]["topLevelFile"] = "naolistado.v".into();
        std::fs::write(&spf, serde_json::to_string(&doc).unwrap()).unwrap();
        let project = Project::open(&spf).unwrap();
        assert_eq!(project.top_level(), None);
        assert_eq!(
            project.issues()[0].kind,
            crate::IssueKind::SelectionNotRegistered
        );

        doc["structure"]["synthesizableFiles"] =
            json!([{"name": "a_tb.v", "path": "a_tb.v", "isTopLevel": true}]);
        doc["structure"]["topLevelFile"] = "a_tb.v".into();
        std::fs::write(&spf, serde_json::to_string(&doc).unwrap()).unwrap();
        let project = Project::open(&spf).unwrap();
        assert_eq!(project.top_level(), None);
        assert_eq!(project.issues()[0].kind, crate::IssueKind::TestbenchAsTop);
    }

    #[test]
    fn a_bad_name_in_a_list_registers_none_of_it() {
        let (_guard, mut project) = project();
        let root = project.root().to_owned();
        let files = [root.join("bom.v"), root.join("1ruim.v")];
        assert!(matches!(
            project.add_verilog_files(None, &files, false),
            Err(AuroraError::InvalidName { .. })
        ));
        assert!(project.files(FileRole::Synthesizable).is_empty());
        assert!(!files[0].exists(), "nada foi criado");
        let added = project.add_verilog_files(None, &files[..1], false).unwrap();
        assert_eq!(added.len(), 1);
        assert!(added[0].created);
    }

    #[test]
    fn removing_a_list_says_what_was_registered() {
        let (_guard, mut project) = project();
        let root = project.root().to_owned();
        let a = root.join("a.v");
        project
            .add_verilog_files(None, std::slice::from_ref(&a), false)
            .unwrap();
        let outcome = project
            .remove_verilog_files(&[a.clone(), a.clone(), root.join("b.v")])
            .unwrap();
        assert_eq!(outcome.removed, vec![a.clone()]);
        assert_eq!(outcome.not_registered, vec![a.clone(), root.join("b.v")]);
        assert!(a.is_file(), "tirar do projeto não apaga do disco");
    }

    #[test]
    fn only_verilog_enters_the_lists_and_nothing_is_saved_on_refusal() {
        let (_guard, mut project) = project();
        let root = project.root().to_owned();
        std::fs::write(root.join("README.md"), "x").unwrap();
        std::fs::write(root.join("notas.txt"), "x").unwrap();
        let before = std::fs::read_to_string(project.spf_path()).unwrap();
        assert!(matches!(
            project.set_top("README.md"),
            Err(AuroraError::InvalidName { .. })
        ));
        assert!(matches!(
            project.set_testbench("notas.txt"),
            Err(AuroraError::InvalidName { .. })
        ));
        assert_eq!(std::fs::read_to_string(project.spf_path()).unwrap(), before);
    }

    #[test]
    fn module_in_two_files_is_ambiguous() {
        let (_guard, mut project) = project();
        for (file, text) in [
            ("core.v", "module core; alu u(); endmodule\n"),
            ("v1/alu.v", "module alu; endmodule\n"),
            ("v2/alu.v", "module alu; endmodule\n"),
        ] {
            project
                .add_file(FileRole::Synthesizable, file, Some(text))
                .unwrap();
        }
        match project.set_top("alu") {
            Err(AuroraError::AmbiguousModule { files, .. }) => assert_eq!(files.len(), 2),
            other => panic!("{other:?}"),
        }
        match project.set_top("nada") {
            Err(AuroraError::ModuleNotFound { available, .. }) => {
                assert_eq!(available, ["core", "alu"])
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            project.set_top("core").unwrap(),
            project.root().join("core.v")
        );
    }

    #[test]
    fn a_file_of_defines_does_not_become_the_top() {
        let (_guard, mut project) = project();
        let root = project.root().to_owned();
        std::fs::write(root.join("defs.v"), "`define NBITS 8\n").unwrap();
        std::fs::write(
            root.join("alu.v"),
            "module alu(input a, output y); endmodule\n",
        )
        .unwrap();
        let defs = project.add_verilog(None, "defs.v", false).unwrap();
        assert!(!defs.selected);
        let alu = project.add_verilog(None, "alu.v", false).unwrap();
        assert!(alu.selected);
        assert_eq!(project.top_level(), Some(root.join("alu.v")));
        assert!(project.check_add_verilog("bad name.v").is_err());
        assert!(project.check_add_verilog("notes.txt").is_err());
        assert!(project.check_add_verilog("ok_tb.v").is_ok());
    }

    #[test]
    fn data_file_rejects_invalid_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().join("d.txt")).unwrap();
        std::fs::write(&path, "1\n\n-2\n3.5\n").unwrap();
        match read_data_file(&path) {
            Err(AuroraError::InvalidDataFile { line, .. }) => assert_eq!(line, 4),
            other => panic!("{other:?}"),
        }
        std::fs::write(&path, " 1\n\n-2 \n").unwrap();
        assert_eq!(read_data_file(&path).unwrap(), [1, -2]);
    }

    #[test]
    fn input_and_output_files() {
        let (_guard, mut project) = project();
        let processor = project
            .add_processor(&NewProcessor::new("q", Language::Cmm))
            .unwrap()
            .clone();
        let path = processor.write_input(0, "1\n2\n").unwrap();
        assert_eq!(path, processor.dir.join("input_0.txt"));
        assert!(processor.read_output(0).is_err());
    }

    #[test]
    fn a_file_counts_once_and_the_spf_is_cleaned_on_the_next_save() {
        let (_guard, project) = project();
        let root = project.root().to_owned();
        std::fs::create_dir_all(root.join("rtl")).unwrap();
        std::fs::write(
            root.join("rtl/contador.v"),
            "module contador(input a); endmodule\n",
        )
        .unwrap();
        std::fs::write(
            root.join("rtl/contador_tb.v"),
            "module contador_tb; endmodule\n",
        )
        .unwrap();
        std::fs::write(root.join("novo.v"), "module novo(input a); endmodule\n").unwrap();
        let spf = project.spf_path().to_owned();
        let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&spf).unwrap()).unwrap();
        doc["structure"]["synthesizableFiles"] = json!([
            {"name": "contador.v", "path": "rtl/contador.v", "isTopLevel": false},
            {"name": "contador.v", "path": "rtl/../rtl/contador.v", "isTopLevel": true},
            {"name": "contador.v", "path": "rtl\\contador.v"},
            {"name": "contador_tb.v", "path": "rtl/contador_tb.v"}
        ]);
        doc["structure"]["testbenchFiles"] = json!([
            {"name": "contador.v", "path": "rtl/contador.v"},
            {"name": "contador_tb.v", "path": "rtl/contador_tb.v", "isMarkedTestbench": true}
        ]);
        std::fs::write(&spf, serde_json::to_string(&doc).unwrap()).unwrap();

        let mut project = Project::open(&spf).unwrap();
        let synth = project.files(FileRole::Synthesizable);
        assert_eq!(synth.len(), 1, "{synth:?}");
        assert!(synth[0].top_level, "a marca de uma das repetidas vale");
        assert_eq!(project.files(FileRole::Testbench).len(), 1);
        // A marca legada da AURORA escolhe o testbench.
        assert_eq!(project.testbench(), Some(root.join("rtl/contador_tb.v")));
        let kinds: Vec<_> = project.issues().into_iter().map(|i| i.kind).collect();
        assert_eq!(
            kinds,
            [
                crate::IssueKind::DuplicateFile,
                crate::IssueKind::DuplicateFile
            ]
        );

        project
            .add_file(FileRole::Synthesizable, "novo.v", None)
            .unwrap();
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&spf).unwrap()).unwrap();
        assert_eq!(
            doc["structure"]["synthesizableFiles"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            doc["structure"]["testbenchFiles"].as_array().unwrap().len(),
            1
        );
        assert!(Project::open(&spf).unwrap().issues().is_empty());
    }

    #[test]
    fn choosing_a_testbench_clears_the_legacy_mark() {
        let (_guard, project) = project();
        let root = project.root().to_owned();
        for name in ["a_tb.v", "b_tb.v"] {
            std::fs::write(root.join(name), "module t; endmodule\n").unwrap();
        }
        let spf = project.spf_path().to_owned();
        let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&spf).unwrap()).unwrap();
        doc["structure"]["testbenchFiles"] = json!([
            {"name": "a_tb.v", "path": "a_tb.v", "isMarkedTestbench": true},
            {"name": "b_tb.v", "path": "b_tb.v"}
        ]);
        std::fs::write(&spf, serde_json::to_string(&doc).unwrap()).unwrap();
        let mut project = Project::open(&spf).unwrap();
        project.set_testbench("b_tb.v").unwrap();
        assert_eq!(
            Project::open(&spf).unwrap().testbench(),
            Some(root.join("b_tb.v"))
        );
    }

    #[test]
    fn files_can_change_place_in_their_list() {
        let (_guard, mut project) = project();
        for (file, text) in [
            ("alu.v", "module alu(input a); endmodule\n"),
            ("defs.v", "`define N 8\n"),
            ("top.v", "module top(input a); endmodule\n"),
        ] {
            project
                .add_file(FileRole::Synthesizable, file, Some(text))
                .unwrap();
        }
        let names = |files: Vec<ProjectFile>| -> Vec<String> {
            files
                .iter()
                .map(|f| f.path.file_name().unwrap().to_owned())
                .collect()
        };
        let order = project
            .reorder_file("defs.v", &ListPosition::First)
            .unwrap();
        assert_eq!(names(order), ["defs.v", "alu.v", "top.v"]);
        let order = project
            .reorder_file("defs.v", &ListPosition::After("top.v".into()))
            .unwrap();
        assert_eq!(names(order), ["alu.v", "top.v", "defs.v"]);
        let order = project
            .reorder_file("defs.v", &ListPosition::Before("top.v".into()))
            .unwrap();
        assert_eq!(names(order), ["alu.v", "defs.v", "top.v"]);
        assert!(project.reorder_file("nada.v", &ListPosition::Last).is_err());
        assert_eq!(
            names(
                Project::open(project.spf_path())
                    .unwrap()
                    .files(FileRole::Synthesizable)
            ),
            ["alu.v", "defs.v", "top.v"]
        );
    }

    #[test]
    fn what_stays_in_place_is_never_replaced() {
        let (_guard, mut project) = project();
        let root = project.root().to_owned();
        let soma = project
            .add_processor(&NewProcessor::new("soma", Language::Cmm))
            .unwrap()
            .clone();
        std::fs::create_dir_all(root.join("rtl/sub")).unwrap();
        std::fs::write(root.join("rtl/sub/a.v"), "").unwrap();
        std::fs::write(root.join("b.v"), "").unwrap();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        for (target, why) in [
            (project.spf_path().as_str(), "o .spf"),
            (root.join(".aurora8").as_str(), "a .aurora8"),
            (soma.dir.as_str(), "a pasta do processador"),
            (soma.source.as_str(), "o fonte dele"),
            (root.as_str(), "a pasta do projeto"),
            (root.join("rtl").as_str(), "uma pasta com a origem"),
        ] {
            assert!(
                project.replace_refusal("rtl/sub/a.v", target).is_some(),
                "{why}"
            );
        }
        // O resto pode sair.
        assert_eq!(project.replace_refusal("rtl/sub/a.v", "b.v"), None);
        assert_eq!(project.replace_refusal("b.v", "docs"), None);
    }

    #[cfg(unix)]
    #[test]
    fn move_does_not_overwrite_a_dangling_link() {
        let (_guard, mut project) = project();
        let root = project.root().to_owned();
        std::fs::write(root.join("ocupado.v"), "x").unwrap();
        std::os::unix::fs::symlink("/nao/existe/alvo", root.join("dangling.v")).unwrap();
        assert!(matches!(
            project.move_path("ocupado.v", "dangling.v"),
            Err(AuroraError::PathExists(_))
        ));
        assert!(matches!(
            project.move_path("nada.v", "outro.v"),
            Err(AuroraError::InvalidProject { .. })
        ));
    }
}
