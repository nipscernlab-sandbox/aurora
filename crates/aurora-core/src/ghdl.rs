//! O GHDL, o simulador de VHDL: os comandos de cada passo e o que a AURORA
//! faz em volta deles. A simulação do testbench VHDL do projeto
//! ([`simulate_project`](crate::simulate_project)) roda três passos, todos em
//! VHDL-2008 (`--std=08`):
//!
//! 1. `ghdl -i` ([`Step::Analyze`](crate::Step::Analyze)): os arquivos VHDL
//!    do projeto (os sintetizáveis, na ordem do `.spf`, e o testbench)
//!    entram na biblioteca `work`, que fica em
//!    `.aurora8/Temp/ghdl/<testbench>`; os erros de sintaxe aparecem aqui;
//! 2. `ghdl -m <testbench>` ([`Step::Elaborate`](crate::Step::Elaborate)):
//!    analisa o que o testbench usa, na ordem das dependências (a AURORA não
//!    ordena nada), e elabora. No Linux e no macOS, o GHDL do OSS CAD Suite
//!    (backend LLVM) liga aqui o executável da simulação com o compilador C
//!    do sistema; no Windows, o mcode gera o código na memória;
//! 3. a execução ([`Step::Simulate`](crate::Step::Simulate)), na raiz do
//!    projeto, para os `file_open` do testbench acharem os arquivos como no
//!    Icarus: `ghdl -r` no Windows, o executável do passo 2 nos outros. A
//!    onda sai em GHW, `<raiz>/<testbench>.ghw`.
//!
//! Os arquivos vão por caminho absoluto, ou relativo à biblioteca quando o
//! caminho tem caractere fora do ASCII: a biblioteca do GHDL guarda o
//! caminho de cada arquivo, e o GHDL não relê um caminho com acento.

use camino::{Utf8Path, Utf8PathBuf};

use crate::diagnostics::{Diagnostic, Severity};
use crate::error::{AuroraError, Result};
use crate::files::FileRole;
use crate::process::Invocation;
use crate::project::Project;
use crate::toolchain::{Platform, Tool, Toolchain, component};

/// O padrão do VHDL de todos os passos. O GHDL usa o 93c sem `--std`, e o
/// mcode elabora de novo no `-r`, que precisa da mesma opção da análise.
pub(crate) const STD: &str = "--std=08";

/// A pasta da biblioteca `work` de `unit` (um testbench, ou o design no
/// `check`), em `.aurora8/Temp/ghdl/`.
pub(crate) fn work_dir(project: &Project, unit: &str) -> Utf8PathBuf {
    project
        .temp_dir()
        .join("ghdl")
        .join(unit.to_ascii_lowercase())
}

/// Os arquivos VHDL sintetizáveis do projeto, na ordem do `.spf`.
///
/// # Erros
///
/// [`AuroraError::InvalidProject`] se um deles não existe.
pub(crate) fn design_files(project: &Project) -> Result<Vec<Utf8PathBuf>> {
    let mut files = Vec::new();
    for file in project.files(FileRole::Synthesizable) {
        if !crate::hdl::is_vhdl(&file.path) {
            continue;
        }
        if !file.path.is_file() {
            return Err(AuroraError::InvalidProject {
                path: file.path,
                reason: "Synthesizable file added to the project does not exist".into(),
            });
        }
        files.push(file.path);
    }
    Ok(files)
}

/// `ghdl -i --std=08 <arquivos>`, em `work`.
pub(crate) fn import(
    toolchain: &Toolchain,
    work: &Utf8Path,
    files: &[Utf8PathBuf],
) -> Result<Invocation> {
    let mut invocation = toolchain
        .invocation(Tool::Ghdl, work)?
        .ascii_cwd()
        .arg("-i")
        .arg(STD);
    for file in files {
        invocation = invocation.tool_path_arg(file);
    }
    Ok(invocation)
}

/// `ghdl -m --std=08 <unidade>`, em `work`. No Linux, com o `libz.so` que o
/// `-lz` da ligação procura ([`link_zlib`]).
pub(crate) fn make(toolchain: &Toolchain, work: &Utf8Path, unit: &str) -> Result<Invocation> {
    let mut invocation = toolchain
        .invocation(Tool::Ghdl, work)?
        .ascii_cwd()
        .arg("-m")
        .arg(STD);
    if let Some(dir) = link_zlib(toolchain, work)? {
        invocation = invocation.arg(format!("-Wl,-L{dir}"));
    }
    Ok(invocation.arg(unit))
}

/// A execução de `unit`, com a biblioteca em `work`, rodando em `cwd`, com
/// as opções de tempo de execução `run_options` (`--wave=...`). No Windows,
/// `ghdl -r`; no Linux e no macOS, o executável que o `ghdl -m` ligou em
/// `work`.
pub(crate) fn run(
    toolchain: &Toolchain,
    work: &Utf8Path,
    unit: &str,
    cwd: &Utf8Path,
    run_options: &[String],
) -> Result<Invocation> {
    let mut invocation = if toolchain.platform() == Platform::WindowsX64 {
        let library = crate::process::tool_path(work, cwd);
        toolchain
            .invocation(Tool::Ghdl, cwd)?
            .ascii_cwd()
            .arg("-r")
            .arg(STD)
            .arg(format!("--workdir={library}"))
            .arg(unit)
    } else {
        let executable = work.join(unit.to_ascii_lowercase());
        if !executable.is_file() {
            return Err(AuroraError::ToolchainIncomplete {
                what: format!("the simulation that ghdl -m links ({executable})"),
                path: executable,
            });
        }
        Invocation::new(executable, cwd)
    };
    for option in run_options {
        invocation = invocation.arg(option.clone());
    }
    Ok(invocation)
}

/// No Linux, a ligação do GHDL pede `-lz`, e o OSS CAD Suite só traz o
/// `libz.so.1`: sem a zlib de desenvolvimento do sistema, o `ld` não acharia
/// o `libz.so`. A AURORA cria em `work/zlib` um `libz.so` que aponta para o
/// do bundle e devolve a pasta, para o `-Wl,-L`. O executável liga com o
/// `libz.so.1`, que todo Linux tem. `None` nos outros sistemas.
fn link_zlib(toolchain: &Toolchain, work: &Utf8Path) -> Result<Option<Utf8PathBuf>> {
    if toolchain.platform() != Platform::LinuxX64 {
        return Ok(None);
    }
    let bundled = toolchain
        .component_dir(component::GHDL)?
        .join("lib/libz.so.1");
    if !bundled.is_file() {
        return Ok(None);
    }
    let dir = work.join("zlib");
    let link = dir.join("libz.so");
    if !link.exists() {
        std::fs::create_dir_all(&dir).map_err(AuroraError::io("Creating directory", &dir))?;
        symlink(&bundled, &link)?;
    }
    Ok(Some(dir))
}

#[cfg(unix)]
fn symlink(target: &Utf8Path, link: &Utf8Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link).map_err(AuroraError::io("Creating link", link))
}

#[cfg(not(unix))]
fn symlink(target: &Utf8Path, link: &Utf8Path) -> Result<()> {
    std::fs::copy(target, link)
        .map(|_| ())
        .map_err(AuroraError::io("Copying", link))
}

/// Os nomes dos módulos Verilog do projeto: os dos sintetizáveis Verilog e o
/// de cada processador SAPHO, que o YANC gera em Verilog.
pub(crate) fn verilog_units(project: &Project) -> Vec<String> {
    let mut names: Vec<String> = project
        .files(FileRole::Synthesizable)
        .into_iter()
        .filter(|f| crate::hdl::is_verilog(&f.path))
        .filter_map(|f| std::fs::read_to_string(&f.path).ok())
        .flat_map(|text| crate::verilog::modules_in(&text))
        .collect();
    names.extend(project.processors().iter().map(|p| p.name.clone()));
    names
}

/// O GHDL só avisa quando uma instância de componente fica sem entidade
/// (`instance "u0" of component "x" is not bound`), e a simulação roda sem
/// ela. Quando o componente é um módulo Verilog do projeto (`verilog`), o
/// aviso vira erro, que diz por quê: o GHDL simula só VHDL, e cada simulação
/// da AURORA roda numa linguagem só. Diz se algum virou.
pub(crate) fn mixed_language_errors(diagnostics: &mut [Diagnostic], verilog: &[String]) -> bool {
    let mut found = false;
    for d in diagnostics
        .iter_mut()
        .filter(|d| d.tool == Tool::Ghdl && d.severity == Severity::Warning)
    {
        let Some(component) = unbound_component(&d.message) else {
            continue;
        };
        if let Some(module) = verilog.iter().find(|m| m.eq_ignore_ascii_case(component)) {
            d.severity = Severity::Error;
            d.message = format!(
                "{} ({module} is a Verilog module; GHDL simulates only VHDL, and a VHDL testbench runs with the VHDL files of the project)",
                d.message
            );
            found = true;
        }
    }
    found
}

/// O componente de `instance "u0" of component "x" is not bound`.
fn unbound_component(message: &str) -> Option<&str> {
    let rest = message.split_once("of component \"")?.1;
    let (name, after) = rest.split_once('"')?;
    after.contains("is not bound").then_some(name)
}

/// Os arquivos que o GHDL cita relativos à biblioteca: os caminhos com
/// acento vão relativos a ela ([`import`]), e o `ghdl -r` roda em outra
/// pasta. Só os que existem.
pub(crate) fn resolve_in_work(diagnostics: &mut [Diagnostic], work: &Utf8Path) {
    for d in diagnostics.iter_mut().filter(|d| d.tool == Tool::Ghdl) {
        let Some(file) = &d.file else { continue };
        if file.is_relative() {
            let absolute = crate::paths::normalize(&work.join(file));
            if absolute.is_file() {
                d.file = Some(absolute);
            }
        }
    }
}

/// A conferência do VHDL no `check`: `ghdl -i` com `files` numa biblioteca
/// própria (`.aurora8/Temp/ghdl/check`, refeita a cada vez), depois
/// `ghdl -m` de cada unidade de `units` (as raízes do design e os
/// testbenches), que analisa e elabora. Põe cada unidade em `targets`.
pub(crate) fn check(
    toolchain: &Toolchain,
    project: &Project,
    files: &[Utf8PathBuf],
    units: &[String],
    runner: &mut crate::pipeline::Runner<'_>,
    targets: &mut Vec<String>,
) -> Result<()> {
    use crate::pipeline::{PlannedStep, Status, Step};
    let work = project.temp_dir().join("ghdl").join("check");
    // Dois `check` ao mesmo tempo (o Studio e o terminal) não dividem a
    // biblioteca, que é apagada a cada vez: um arquivo tirado do projeto não
    // fica nela.
    let _lock = crate::paths::run_lock(&work, project.name())?;
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(AuroraError::io("Creating directory", &work))?;
    let analyze = import(toolchain, &work, files)?;
    if runner.run(PlannedStep::new(Step::Analyze, Tool::Ghdl, analyze))? {
        for unit in units {
            if runner.status != Status::Succeeded {
                break;
            }
            targets.push(unit.clone());
            let elaborate = make(toolchain, &work, unit)?;
            runner.run(PlannedStep::new(Step::Elaborate, Tool::Ghdl, elaborate))?;
        }
    }
    if mixed_language_errors(&mut runner.diagnostics, &verilog_units(project)) {
        runner.fail_on_error_diagnostics(Tool::Ghdl);
    }
    resolve_in_work(&mut runner.diagnostics, &work);
    Ok(())
}

/// As raízes VHDL do design: as entidades de `files` que nenhuma
/// arquitetura deles instancia.
pub(crate) fn design_roots(files: &[Utf8PathBuf]) -> Result<Vec<String>> {
    let mut texts = Vec::new();
    for file in files {
        texts.push(std::fs::read_to_string(file).map_err(AuroraError::io("Reading", file))?);
    }
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    Ok(crate::vhdl::design_roots(&refs))
}

/// A árvore de instâncias de `units`, elaboradas pelo GHDL com `files`: a
/// biblioteca em `work`, `ghdl -i`, `ghdl -m` de cada unidade e a execução
/// por zero segundo com `--disp-tree=inst`, que imprime a árvore. Os nomes
/// vêm como estão nos arquivos (o GHDL os escreve em minúsculas); o arquivo
/// e a linha de cada entidade e de cada instância saem do leitor de VHDL.
pub(crate) fn tree<'c>(
    toolchain: &Toolchain,
    project: &Project,
    files: &[Utf8PathBuf],
    units: &[String],
    work: &Utf8Path,
    control: &'c crate::control::Control,
) -> Result<(
    crate::pipeline::Runner<'c>,
    Vec<crate::hierarchy::ModuleInstance>,
)> {
    use crate::pipeline::{PlannedStep, Runner, Status, Step};
    std::fs::create_dir_all(work).map_err(AuroraError::io("Creating directory", work))?;
    let mut runner = Runner::new(control);
    let mut roots = Vec::new();
    let analyze = import(toolchain, work, files)?;
    if !runner.run(PlannedStep::new(Step::Analyze, Tool::Ghdl, analyze))? {
        return Ok((runner, roots));
    }
    let index = Index::of(files);
    for unit in units {
        if runner.status != Status::Succeeded {
            break;
        }
        let elaborate = make(toolchain, work, unit)?;
        if !runner.run(PlannedStep::new(Step::Elaborate, Tool::Ghdl, elaborate))? {
            break;
        }
        let options = ["--disp-tree=inst".to_owned(), "--stop-time=0ns".to_owned()];
        let display = run(toolchain, work, unit, project.root(), &options)?;
        if runner.run(PlannedStep::new(Step::Elaborate, Tool::Ghdl, display))? {
            let text = runner
                .steps
                .last()
                .map(|s| s.stdout.clone())
                .unwrap_or_default();
            roots.extend(parse_tree(&text, &index));
        }
    }
    if mixed_language_errors(&mut runner.diagnostics, &verilog_units(project)) {
        runner.fail_on_error_diagnostics(Tool::Ghdl);
    }
    resolve_in_work(&mut runner.diagnostics, work);
    Ok((runner, roots))
}

/// Onde cada entidade e cada instância estão nos arquivos, pelo leitor de
/// VHDL, com os nomes em minúsculas como chave.
struct Index {
    /// Entidade: o nome como está no arquivo, o arquivo e a linha.
    entities: std::collections::HashMap<String, (String, Utf8PathBuf, u32)>,
    /// (entidade da arquitetura, rótulo): o rótulo como está, o arquivo e a
    /// linha.
    instances: std::collections::HashMap<(String, String), (String, Utf8PathBuf, u32)>,
}

impl Index {
    fn of(files: &[Utf8PathBuf]) -> Index {
        let mut index = Index {
            entities: Default::default(),
            instances: Default::default(),
        };
        for file in files {
            let Ok(text) = std::fs::read_to_string(file) else {
                continue;
            };
            for entity in crate::vhdl::entities(&text) {
                index.entities.entry(entity.name.to_lowercase()).or_insert((
                    entity.name,
                    file.clone(),
                    entity.line,
                ));
            }
            for instance in crate::vhdl::instances_in(&text) {
                index
                    .instances
                    .entry((
                        instance.within.to_lowercase(),
                        instance.label.to_lowercase(),
                    ))
                    .or_insert((instance.label, file.clone(), instance.line));
            }
        }
        index
    }
}

/// Um nó do `--disp-tree=inst`: `nome [tipo]`.
struct Node {
    name: String,
    kind: String,
    children: Vec<Node>,
}

/// A profundidade, o nome e o tipo de uma linha da árvore do GHDL
/// (`` `-dut [instance]``, `` |  `-rtl [arch]``): cada nível tem dois
/// caracteres de desenho.
fn tree_line(line: &str) -> Option<(usize, String, String)> {
    let (rest, kind) = line.trim_end().strip_suffix(']')?.rsplit_once(" [")?;
    let start = rest.find(|c: char| !matches!(c, ' ' | '|' | '`' | '+' | '-'))?;
    Some((start / 2, rest[start..].to_owned(), kind.to_owned()))
}

fn build(items: &[(usize, String, String)], next: &mut usize, depth: usize) -> Vec<Node> {
    let mut nodes = Vec::new();
    while let Some((d, name, kind)) = items.get(*next) {
        if *d < depth {
            break;
        }
        *next += 1;
        if *d > depth {
            continue;
        }
        let children = build(items, next, depth + 1);
        nodes.push(Node {
            name: name.clone(),
            kind: kind.clone(),
            children,
        });
    }
    nodes
}

/// As raízes da árvore que o `--disp-tree=inst` imprimiu: cada entidade do
/// primeiro nível, com as instâncias da arquitetura dela. Os pacotes ficam
/// de fora; um bloco ou um `generate` não vira nó, e o nome dele vai na
/// frente do das instâncias de dentro (`gen.u0`), como no Verilog.
fn parse_tree(text: &str, index: &Index) -> Vec<crate::hierarchy::ModuleInstance> {
    let items: Vec<_> = text.lines().filter_map(tree_line).collect();
    let mut next = 0;
    build(&items, &mut next, 0)
        .iter()
        .filter(|n| n.kind == "entity")
        .map(|entity| entity_instance(&entity.name, entity, None, index))
        .collect()
}

fn entity_instance(
    name: &str,
    entity: &Node,
    instance: Option<(Utf8PathBuf, u32)>,
    index: &Index,
) -> crate::hierarchy::ModuleInstance {
    let found = index.entities.get(&entity.name.to_lowercase());
    let architecture = entity.children.iter().find(|c| c.kind == "arch");
    let children = architecture
        .map(|arch| instances_of(arch, &entity.name, "", index))
        .unwrap_or_default();
    crate::hierarchy::ModuleInstance {
        name: name.to_owned(),
        module: found.map_or_else(|| entity.name.clone(), |(n, _, _)| n.clone()),
        architecture: architecture.map(|a| a.name.clone()),
        file: found.map(|(_, f, _)| f.clone()),
        line: found.map(|(_, _, l)| *l),
        instance_file: instance.as_ref().map(|(f, _)| f.clone()),
        instance_line: instance.map(|(_, l)| l),
        library: false,
        children,
    }
}

fn instances_of(
    node: &Node,
    within: &str,
    prefix: &str,
    index: &Index,
) -> Vec<crate::hierarchy::ModuleInstance> {
    let mut out = Vec::new();
    for child in &node.children {
        match child.kind.as_str() {
            "instance" => {
                let Some(entity) = child.children.iter().find(|c| c.kind == "entity") else {
                    continue;
                };
                let place = index
                    .instances
                    .get(&(within.to_lowercase(), child.name.to_lowercase()));
                let label = place.map_or_else(|| child.name.clone(), |(l, _, _)| l.clone());
                let at = place.map(|(_, f, l)| (f.clone(), *l));
                out.push(entity_instance(
                    &format!("{prefix}{label}"),
                    entity,
                    at,
                    index,
                ));
            }
            "process" | "entity" | "arch" | "package" => {}
            _ => out.extend(instances_of(
                child,
                within,
                &format!("{prefix}{}.", child.name),
                index,
            )),
        }
    }
    out
}

/// O arquivo de opções de onda do GHDL (`--read-wave-opt`) com os sinais
/// escolhidos: `$ version 1.1` e um caminho por linha, `/tb/dut/q`. A
/// escolha da AURORA guarda `tb.dut.q`.
pub(crate) fn wave_options(selection: &[String]) -> String {
    let mut text = String::from("$ version 1.1\n");
    for path in selection {
        text.push('/');
        text.push_str(&path.replace('.', "/"));
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warning(message: &str) -> Diagnostic {
        Diagnostic {
            tool: Tool::Ghdl,
            severity: Severity::Warning,
            message: message.into(),
            file: None,
            line: None,
            column: None,
            raw: String::new(),
        }
    }

    #[test]
    fn an_unbound_verilog_component_becomes_an_error() {
        let mut found = vec![
            warning("instance \"u0\" of component \"modulo_verilog\" is not bound [-Wbinding]"),
            warning("instance \"u1\" of component \"outro\" is not bound [-Wbinding]"),
            warning("(in default configuration of misto_tb(sim))"),
        ];
        assert!(mixed_language_errors(
            &mut found,
            &["Modulo_Verilog".into()]
        ));
        assert_eq!(found[0].severity, Severity::Error);
        assert!(
            found[0]
                .message
                .contains("Modulo_Verilog is a Verilog module")
        );
        assert_eq!(
            found[1].severity,
            Severity::Warning,
            "sem módulo Verilog, fica o aviso"
        );
        assert!(!mixed_language_errors(&mut found[2..], &["x".into()]));
    }

    #[test]
    fn the_disp_tree_becomes_the_instance_tree() {
        // A saída do GHDL 6.0.0 com `--disp-tree=inst` (proc mostraria os
        // processos, que ficam de fora).
        let text = "\
contador_tb [entity]
`-sim [arch]
  +-dut [instance]
  | `-contador [entity]
  |   `-rtl [arch]
  `-gen [for-generate]
    `-u1 [instance]
      `-celula [entity]
        `-rtl [arch]
std.standard [package]
ghdl:info: simulation stopped by --stop-time @0ms
";
        let index = Index {
            entities: [(
                "contador".to_owned(),
                (
                    "Contador".to_owned(),
                    Utf8PathBuf::from("/p/contador.vhd"),
                    5,
                ),
            )]
            .into_iter()
            .collect(),
            instances: [(
                ("contador_tb".to_owned(), "dut".to_owned()),
                (
                    "DUT".to_owned(),
                    Utf8PathBuf::from("/p/contador_tb.vhd"),
                    12,
                ),
            )]
            .into_iter()
            .collect(),
        };
        let roots = parse_tree(text, &index);
        assert_eq!(roots.len(), 1, "o pacote fica de fora");
        let tb = &roots[0];
        assert_eq!(
            (tb.name.as_str(), tb.architecture.as_deref()),
            ("contador_tb", Some("sim"))
        );
        let names: Vec<_> = tb
            .children
            .iter()
            .map(|c| (c.name.as_str(), c.module.as_str()))
            .collect();
        assert_eq!(names, [("DUT", "Contador"), ("gen.u1", "celula")]);
        assert_eq!(tb.children[0].line, Some(5));
        assert_eq!(tb.children[0].instance_line, Some(12));
        assert_eq!(tb.children[0].architecture.as_deref(), Some("rtl"));
    }

    #[test]
    fn the_wave_choice_becomes_ghdl_paths() {
        assert_eq!(
            wave_options(&["tb.dut.q".into(), "tb.clk".into()]),
            "$ version 1.1\n/tb/dut/q\n/tb/clk\n"
        );
    }
}
