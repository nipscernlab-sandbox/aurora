//! A escolha dos sinais da onda do projeto: o que a Wave Configuration da
//! AURORA faz (`js/wave/wave_config_manager.ts` e
//! `js/wave/testbench_instrumenter.ts`).
//!
//! A escolha é por testbench e fica num arquivo do projeto, para ir com ele
//! (git), com o nome do módulo do testbench:
//!
//! ```json
//! // wave/tb_contador.json
//! { "schema": 1, "signals": ["tb_contador.clk", "tb_contador.dut"] }
//! ```
//!
//! Cada item é um sinal ou um escopo (uma instância, um bloco `generate` ou
//! `begin` com nome), pelo caminho na hierarquia; um escopo vale por tudo o
//! que há dentro dele. Sem o arquivo, ou com a lista vazia, a simulação grava
//! todos os sinais, como antes.
//!
//! Com escolha, a simulação do projeto ([`simulate_project`](crate::simulate_project))
//! grava só ela, numa cópia do testbench (o arquivo do usuário não muda): o
//! primeiro `$dumpvars` do testbench vira `$dumpvars(0, <itens>)` e os
//! outros viram comentário; sem `$dumpvars`, a lista entra depois do
//! `$dumpfile`; sem dump nenhum, no lugar do `$dumpvars(0, <topo>)` que o
//! AURORA injeta. Um processador SAPHO do design que a escolha não cobre entra
//! sempre com os sinais do escopo dele (`$dumpvars(1, <processador>)`):
//! o layout das instruções, das variáveis e do I/O precisa deles, como na
//! AURORA, onde esses grupos aparecem com qualquer escolha. Um item que o
//! design não tem mais (um sinal renomeado) fica de fora, com um aviso. O
//! Verilator ignora a lista do `$dumpvars` e grava tudo; o layout mostra só
//! a escolha do mesmo jeito ([`wave_layout`](crate::wave_layout)).
//!
//! A árvore para escolher ([`wave_signals`]) vem da elaboração do testbench
//! pelo Icarus, a mesma da [`hierarchy`](crate::hierarchy): o `.vvp` lista
//! cada escopo com os sinais (`.var`, `.net`) e as portas (`.port_info`). Não
//! precisa simular, e mostra também o que a onda de agora não gravou.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::time::Instant;

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::control::Control;
use crate::diagnostics::{Diagnostic, Severity};
use crate::error::{AuroraError, Result};
use crate::pipeline::{Status, StepReport, elapsed_ms};
use crate::project::Project;
use crate::toolchain::{Tool, Toolchain};

/// A pasta, na raiz do projeto, da escolha de sinais e dos layouts salvos.
pub const WAVE_DIR: &str = "wave";
/// Versão do formato da escolha.
const SELECTION_SCHEMA: u32 = 1;

/// A escolha de sinais de um testbench (`wave/<testbench>.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SelectionFile {
    schema: u32,
    signals: Vec<String>,
}

/// O nome de arquivo do testbench `module` em `wave/`: o nome dele, com `_`
/// no lugar do que não vai bem num nome de arquivo (um identificador
/// escapado do Verilog pode ter qualquer coisa).
fn file_stem(module: &str) -> String {
    let stem: String = module
        .trim_start_matches('\\')
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '$' | '-' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if stem.is_empty() || stem.starts_with('.') {
        format!("_{stem}")
    } else {
        stem
    }
}

/// A escolha de sinais do testbench `module`: `wave/<módulo>.json`.
pub fn selection_file(project: &Project, module: &str) -> Utf8PathBuf {
    project
        .root()
        .join(WAVE_DIR)
        .join(format!("{}.json", file_stem(module)))
}

/// O layout salvo da onda do testbench `module`: `wave/<módulo>.surf.ron`
/// ([`saved_layout`](crate::saved_layout)).
pub fn layout_file(project: &Project, module: &str) -> Utf8PathBuf {
    project
        .root()
        .join(WAVE_DIR)
        .join(format!("{}.surf.ron", file_stem(module)))
}

/// A escolha de sinais do testbench `module`; vazia sem o arquivo: todos os
/// sinais.
///
/// # Erros
///
/// [`AuroraError::InvalidProject`] se o arquivo não é uma escolha válida.
pub fn read_selection(project: &Project, module: &str) -> Result<Vec<String>> {
    let path = selection_file(project, module);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(AuroraError::io("Reading", &path)(e)),
    };
    let file: SelectionFile =
        serde_json::from_str(&text).map_err(|e| AuroraError::InvalidProject {
            path: path.clone(),
            reason: format!("Invalid signal choice: {e}"),
        })?;
    if file.schema != SELECTION_SCHEMA {
        return Err(AuroraError::InvalidProject {
            path,
            reason: format!(
                "The signal choice has format {}; this AURORA reads format {SELECTION_SCHEMA}",
                file.schema
            ),
        });
    }
    Ok(normalize(&file.signals))
}

/// Grava a escolha de sinais do testbench `module`. Uma lista vazia apaga o
/// arquivo: a simulação volta a gravar todos os sinais. Devolve o arquivo,
/// ou `None` quando ele saiu.
///
/// A lista é gravada em ordem, sem repetições nem itens que um escopo
/// escolhido já cobre (`tb.dut.q` com `tb.dut`).
pub fn write_selection(
    project: &Project,
    module: &str,
    signals: &[String],
) -> Result<Option<Utf8PathBuf>> {
    let path = selection_file(project, module);
    let signals = normalize(signals);
    if signals.is_empty() {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(AuroraError::io("Removing", &path)(e)),
        }
        // A pasta sai junto se ficou vazia.
        if let Some(dir) = path.parent() {
            let _ = std::fs::remove_dir(dir);
        }
        return Ok(None);
    }
    let dir = path.parent().expect("o arquivo fica em wave/");
    std::fs::create_dir_all(dir).map_err(AuroraError::io("Creating directory", dir))?;
    let file = SelectionFile {
        schema: SELECTION_SCHEMA,
        signals,
    };
    let text = serde_json::to_string_pretty(&file).expect("a escolha vira JSON") + "\n";
    std::fs::write(&path, text).map_err(AuroraError::io("Writing", &path))?;
    Ok(Some(path))
}

/// A lista sem itens vazios, em ordem, sem repetições e sem o que um escopo
/// da lista já cobre.
fn normalize(signals: &[String]) -> Vec<String> {
    let set: BTreeSet<String> = signals
        .iter()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    set.iter()
        .filter(|s| !set.iter().any(|other| covers(other, s) && other != *s))
        .cloned()
        .collect()
}

/// `item` cobre `path`: é ele, ou um escopo acima dele.
pub(crate) fn covers(item: &str, path: &str) -> bool {
    path == item
        || path
            .strip_prefix(item)
            .is_some_and(|rest| rest.starts_with('.'))
}

// ---------------------------------------------------------------- a árvore

/// A árvore de sinais do testbench do projeto, para escolher o que a onda
/// grava ([`wave_signals`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct WaveSignals {
    /// O testbench elaborado (o da simulação do projeto).
    #[schemars(with = "String")]
    pub testbench: Utf8PathBuf,
    /// O módulo do testbench: a raiz da árvore e o nome dos arquivos em
    /// `wave/`.
    pub module: String,
    /// A árvore, a partir do módulo do testbench. `None` quando a elaboração
    /// falhou (os erros em `diagnostics`).
    pub root: Option<SignalScope>,
    /// A escolha gravada; vazia: a simulação grava todos os sinais.
    pub selection: Vec<String>,
    /// Onde a escolha fica (`wave/<módulo>.json`), exista ou não.
    #[schemars(with = "String")]
    pub selection_file: Utf8PathBuf,
    /// Itens da escolha que a árvore não tem (um sinal renomeado, um módulo
    /// que saiu): a simulação os deixa de fora.
    pub unknown: Vec<String>,
    /// Como a elaboração terminou.
    pub status: Status,
    /// A elaboração (`iverilog`).
    pub steps: Vec<StepReport>,
    /// Os erros e avisos da elaboração.
    pub diagnostics: Vec<Diagnostic>,
    /// Quanto levou, em milissegundos.
    pub duration_ms: u64,
}

/// Um escopo da árvore: uma instância de módulo, ou um bloco `generate` ou
/// `begin` com nome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct SignalScope {
    /// O nome (`dut`, `g[0]`).
    pub name: String,
    /// O caminho na hierarquia (`tb.dut`), como vai na escolha.
    pub path: String,
    /// Instância de módulo, bloco `generate` ou bloco com nome.
    pub kind: ScopeKind,
    /// O módulo, numa instância.
    pub module: Option<String>,
    /// É um processador SAPHO (tem `valr2` e `linetabs`): os sinais dele vão
    /// para a onda com qualquer escolha.
    pub processor: bool,
    /// Os sinais, na ordem do Icarus.
    pub signals: Vec<WaveSignal>,
    /// Os escopos de dentro.
    pub scopes: Vec<SignalScope>,
}

/// O tipo de um escopo. Em JSON, `"module"`, `"generate"` ou `"block"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ScopeKind {
    /// Uma instância de módulo.
    Module,
    /// Um bloco `generate`.
    Generate,
    /// Um bloco `begin` ou `fork` com nome.
    Block,
}

/// Um sinal da árvore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct WaveSignal {
    /// O nome.
    pub name: String,
    /// O caminho na hierarquia (`tb.dut.q`).
    pub path: String,
    /// Bits (64 num `real`).
    pub width: u32,
    /// `reg`, `wire`, `integer` ou `real`.
    pub kind: SignalKind,
    /// A direção, quando o sinal é uma porta do módulo.
    pub direction: Option<PortDirection>,
}

/// O tipo de um sinal. Em JSON, `"reg"`, `"wire"`, `"integer"` ou `"real"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SignalKind {
    /// Uma variável (`reg`, `logic`, `bit`).
    Reg,
    /// Uma rede (`wire`).
    Wire,
    /// `integer`.
    Integer,
    /// `real`.
    Real,
}

/// A direção de uma porta: a mesma do leitor de portas
/// ([`crate::verilog::PortDirection`]). Em JSON, `"input"`, `"output"` ou
/// `"inout"`.
pub use crate::verilog::PortDirection;

impl SignalScope {
    /// A árvore tem `path` (um escopo ou um sinal)?
    pub fn contains(&self, path: &str) -> bool {
        if !covers(&self.path, path) {
            return false;
        }
        self.path == path
            || self.signals.iter().any(|s| s.path == path)
            || self.scopes.iter().any(|s| s.contains(path))
    }

    /// O escopo `path`, se a árvore tem.
    pub fn scope(&self, path: &str) -> Option<&SignalScope> {
        if self.path == path {
            return Some(self);
        }
        self.scopes
            .iter()
            .filter(|s| covers(&s.path, path))
            .find_map(|s| s.scope(path))
    }

    /// A escolha `selection` sem `path`. Um escopo escolhido que contém
    /// `path` dá lugar ao que há dentro dele, menos `path` (tirar `tb.dut.q`
    /// de `[tb.dut]` deixa os outros sinais e escopos de `tb.dut`). `None`
    /// se nada da escolha cobre `path`.
    pub fn without(&self, selection: &[String], path: &str) -> Option<Vec<String>> {
        if selection.iter().any(|s| s == path) {
            return Some(selection.iter().filter(|s| *s != path).cloned().collect());
        }
        let ancestor = selection.iter().find(|item| covers(item, path))?;
        let node = self.scope(ancestor)?;
        let mut out: Vec<String> = selection
            .iter()
            .filter(|s| *s != ancestor)
            .cloned()
            .collect();
        node.expand_without(path, &mut out);
        Some(normalize(&out))
    }

    fn expand_without(&self, path: &str, out: &mut Vec<String>) {
        out.extend(
            self.signals
                .iter()
                .filter(|s| s.path != path)
                .map(|s| s.path.clone()),
        );
        for scope in &self.scopes {
            if scope.path == path {
                continue;
            }
            if covers(&scope.path, path) {
                scope.expand_without(path, out);
            } else {
                out.push(scope.path.clone());
            }
        }
    }

    /// Todos os caminhos da árvore, de escopos e de sinais.
    fn paths(&self, out: &mut HashSet<String>) {
        out.insert(self.path.clone());
        out.extend(self.signals.iter().map(|s| s.path.clone()));
        for scope in &self.scopes {
            scope.paths(out);
        }
    }

    /// Os escopos de processador, de cima para baixo.
    fn processors<'a>(&'a self, out: &mut Vec<&'a SignalScope>) {
        if self.processor {
            out.push(self);
        }
        for scope in &self.scopes {
            scope.processors(out);
        }
    }
}

/// O testbench da simulação do projeto e o módulo dele, com a regra da
/// simulação ([`Project::testbench`], [`Project::testbench_module`]).
fn project_testbench(project: &Project) -> Result<(Utf8PathBuf, String)> {
    let testbench = project
        .testbench()
        .ok_or_else(|| AuroraError::NoTestbench(project.spf_path().to_owned()))?;
    if crate::cocotb::is_testbench(&testbench) {
        return Err(AuroraError::InvalidProject {
            path: testbench,
            reason: "A cocotb testbench records every signal of the module it tests: the signal choice is for Verilog testbenches".into(),
        });
    }
    if !testbench.is_file() {
        return Err(AuroraError::InvalidProject {
            path: testbench,
            reason: "The project testbench does not exist".into(),
        });
    }
    let module = project
        .testbench_module()?
        .unwrap_or_else(|| testbench.file_stem().unwrap_or_default().to_owned());
    Ok((testbench, module))
}

/// O módulo do testbench da simulação do projeto, o nome dos arquivos dele
/// em `wave/`.
///
/// # Erros
///
/// [`AuroraError::NoTestbench`] sem testbench; [`AuroraError::InvalidProject`]
/// com um testbench cocotb ou que não existe.
pub fn wave_testbench(project: &Project) -> Result<String> {
    project_testbench(project).map(|(_, module)| module)
}

/// Elabora o testbench do projeto com o Icarus e devolve a árvore de sinais,
/// com a escolha gravada. A elaboração vai para
/// `.aurora8/Temp/wave/signals.vvp`; nada no projeto muda.
///
/// Uma elaboração que falha não é erro: o resultado vem sem árvore, com os
/// erros do Icarus em `diagnostics`.
///
/// # Erros
///
/// - [`AuroraError::NoTestbench`]: o projeto não tem testbench;
/// - [`AuroraError::InvalidProject`]: o testbench é cocotb, ou um arquivo
///   registrado não existe, ou a escolha gravada é inválida;
/// - [`AuroraError::ComponentMissing`] sem o Icarus, ou sem o YANC num projeto
///   com processadores (a biblioteca SAPHO vem dele);
/// - [`AuroraError::OperationInProgress`]: outra árvore de sinais do projeto
///   está sendo elaborada.
pub fn wave_signals(
    toolchain: &Toolchain,
    project: &Project,
    control: &Control,
) -> Result<WaveSignals> {
    let _span = tracing::info_span!("wave_signals").entered();
    let started = Instant::now();
    let (testbench, module) = project_testbench(project)?;
    // A árvore sai da elaboração do Icarus; a de um testbench VHDL, pelo
    // GHDL, ainda não existe. A onda dele grava todos os sinais.
    if crate::hdl::is_vhdl(&testbench) {
        return Err(AuroraError::InvalidProject {
            path: testbench,
            reason: "The signal choice is not available yet for a VHDL testbench: GHDL records every signal".into(),
        });
    }
    let selection = read_selection(project, &module)?;
    // A imagem é sempre a mesma: duas árvores ao mesmo tempo (o Studio e o
    // terminal) não a dividem. A da simulação é outra ([`dump_choice`]).
    let _lock = crate::paths::run_lock(&project.temp_dir().join("wave"), project.name())?;
    let (status, steps, diagnostics, root) =
        elaborate_tree(toolchain, project, &testbench, &module, "signals", control)?;
    let unknown = match &root {
        Some(root) => split_known(root, &selection).1,
        None => Vec::new(),
    };
    Ok(WaveSignals {
        selection_file: selection_file(project, &module),
        testbench,
        module,
        root,
        selection,
        unknown,
        status,
        steps,
        diagnostics,
        duration_ms: elapsed_ms(started),
    })
}

type Elaborated = (
    Status,
    Vec<StepReport>,
    Vec<Diagnostic>,
    Option<SignalScope>,
);

/// Elabora `testbench` com o design e lê a árvore a partir de `module`, com
/// a imagem do Icarus em `.aurora8/Temp/wave/<image>.vvp`.
fn elaborate_tree(
    toolchain: &Toolchain,
    project: &Project,
    testbench: &Utf8Path,
    module: &str,
    image: &str,
    control: &Control,
) -> Result<Elaborated> {
    let mut files = crate::synth::project_sources(project)?;
    if !files.iter().any(|f| f == testbench) {
        files.push(testbench.to_owned());
    }
    let library = toolchain.sapho_library(!project.processors().is_empty())?;
    let work = project.temp_dir().join("wave");
    std::fs::create_dir_all(&work).map_err(AuroraError::io("Creating directory", &work))?;
    let image = work.join(format!("{image}.vvp"));
    let (runner, text) = crate::hierarchy::elaborate(
        toolchain,
        project,
        &files,
        &[module.to_owned()],
        library.as_deref(),
        &image,
        control,
    )?;
    let root = text.as_deref().and_then(|t| parse_signals(t, module));
    Ok((runner.status, runner.steps, runner.diagnostics, root))
}

/// Um escopo do `.vvp`, enquanto a árvore é montada.
struct RawScope {
    scope: crate::hierarchy::Scope,
    signals: Vec<(String, SignalKind, u32)>,
    ports: HashMap<String, PortDirection>,
}

/// A árvore de sinais de um `.vvp`, a partir da raiz `module` (sem ela, a
/// primeira raiz). Funções e tarefas ficam de fora, com o que há dentro
/// delas; os nomes com `*` são do Icarus (expressões sem nome) e também.
fn parse_signals(text: &str, module: &str) -> Option<SignalScope> {
    let mut raws: Vec<RawScope> = Vec::new();
    for line in text.lines() {
        if let Some(scope) = crate::hierarchy::parse_scope(line) {
            raws.push(RawScope {
                scope,
                signals: Vec::new(),
                ports: HashMap::new(),
            });
            continue;
        }
        let Some(current) = raws.last_mut() else {
            continue;
        };
        if let Some(rest) = line.trim_start().strip_prefix(".port_info ") {
            if let Some((name, direction)) = parse_port(rest) {
                current.ports.insert(name, direction);
            }
        } else if let Some(signal) = parse_signal(line)
            && !current.signals.iter().any(|s| s.0 == signal.0)
        {
            current.signals.push(signal);
        }
    }

    let index: HashMap<&str, usize> = raws
        .iter()
        .enumerate()
        .map(|(i, r)| (r.scope.label.as_str(), i))
        .collect();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); raws.len()];
    let mut roots = Vec::new();
    for (i, raw) in raws.iter().enumerate() {
        match raw.scope.parent.as_deref().and_then(|p| index.get(p)) {
            Some(&parent) => children[parent].push(i),
            None => roots.push(i),
        }
    }
    let root = roots
        .iter()
        .copied()
        .find(|&i| raws[i].scope.name == module)
        .or_else(|| roots.first().copied())?;
    build(&raws, &children, root, None)
}

/// O escopo `i` e o que há dentro dele; `None` para funções e tarefas.
fn build(
    raws: &[RawScope],
    children: &[Vec<usize>],
    i: usize,
    parent: Option<&str>,
) -> Option<SignalScope> {
    let raw = &raws[i];
    let kind = match raw.scope.kind.as_str() {
        "module" => ScopeKind::Module,
        "generate" => ScopeKind::Generate,
        "begin" | "fork" => ScopeKind::Block,
        _ => return None,
    };
    let path = match parent {
        Some(parent) => format!("{parent}.{}", raw.scope.name),
        None => raw.scope.name.clone(),
    };
    let signals: Vec<WaveSignal> = raw
        .signals
        .iter()
        .map(|(name, kind, width)| WaveSignal {
            path: format!("{path}.{name}"),
            name: name.clone(),
            width: *width,
            kind: *kind,
            direction: raw.ports.get(name).copied(),
        })
        .collect();
    let processor = ["valr2", "linetabs"]
        .iter()
        .all(|n| signals.iter().any(|s| s.name == *n));
    let scopes = children[i]
        .iter()
        .filter_map(|&c| build(raws, children, c, Some(&path)))
        .collect();
    Some(SignalScope {
        name: raw.scope.name.clone(),
        module: (kind == ScopeKind::Module).then(|| raw.scope.module.clone()),
        path,
        kind,
        processor,
        signals,
        scopes,
    })
}

/// `v0x.. .var "count", 3 0;` (e `.var/s`, `.var/i`, `.var/real`),
/// `v0x.. .net "q", 7 0, v0x..;` (e `.net/s`, `.net/2u`): o nome, o tipo e
/// a largura. Um nome com `*` na frente é do Icarus.
fn parse_signal(line: &str) -> Option<(String, SignalKind, u32)> {
    let (label, rest) = line.split_once(" .")?;
    if !label.starts_with('v') {
        return None;
    }
    let (directive, rest) = rest.split_once(' ')?;
    let (base, suffix) = directive.split_once('/').unwrap_or((directive, ""));
    let kind = match (base, suffix) {
        ("var" | "net", "real") => SignalKind::Real,
        ("var", "i") => SignalKind::Integer,
        ("var", _) => SignalKind::Reg,
        ("net", _) => SignalKind::Wire,
        _ => return None,
    };
    let rest = rest.trim_start();
    if rest.starts_with('*') {
        return None;
    }
    let (name, rest) = crate::hierarchy::quoted(rest)?;
    let numbers: Vec<i64> = rest
        .trim_start_matches(',')
        .split([',', ';'])
        .next()?
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    let width = match (kind, numbers.as_slice()) {
        (SignalKind::Real, _) => 64,
        (_, [msb, lsb]) => u32::try_from((msb - lsb).unsigned_abs() + 1).unwrap_or(u32::MAX),
        _ => 1,
    };
    Some((name, kind, width))
}

/// `0 /INPUT 1 "clk";`: o nome e a direção.
fn parse_port(rest: &str) -> Option<(String, PortDirection)> {
    let mut words = rest.split_whitespace();
    words.next()?;
    let direction = match words.next()? {
        "/INPUT" => PortDirection::Input,
        "/OUTPUT" => PortDirection::Output,
        "/INOUT" => PortDirection::Inout,
        _ => return None,
    };
    words.next()?;
    let quoted_name = rest.find('"').map(|i| &rest[i..])?;
    let (name, _) = crate::hierarchy::quoted(quoted_name)?;
    Some((name, direction))
}

/// Os itens da escolha que a árvore tem e os que não tem.
fn split_known(root: &SignalScope, items: &[String]) -> (Vec<String>, Vec<String>) {
    let mut paths = HashSet::new();
    root.paths(&mut paths);
    items.iter().cloned().partition(|i| paths.contains(i))
}

// ---------------------------------------------------------- a simulação

/// O que a simulação do projeto grava com a escolha de sinais do testbench.
pub(crate) struct DumpChoice {
    /// O que vai no lugar do `$dumpvars` do testbench: um comando só
    /// (`begin ... end`), para valer também depois de um `if`.
    pub(crate) calls: String,
    /// Os avisos: itens que o design não tem mais.
    pub(crate) notes: Vec<Diagnostic>,
}

/// A escolha de sinais do testbench `module` do projeto, pronta para a
/// simulação; `None` sem escolha, ou quando a elaboração falha (a simulação
/// mostra os mesmos erros) ou nenhum item existe mais (com um aviso, todos
/// os sinais).
pub(crate) fn dump_choice(
    toolchain: &Toolchain,
    project: &Project,
    testbench: &Utf8Path,
    module: &str,
    control: &Control,
) -> Result<Option<DumpChoice>> {
    let selection = read_selection(project, module)?;
    if selection.is_empty() {
        return Ok(None);
    }
    let file = selection_file(project, module);
    // Dentro da simulação do projeto, que tem a trava dele.
    let (_, _, _, root) = elaborate_tree(toolchain, project, testbench, module, "dump", control)?;
    Ok(root.map(|root| choice_for(&root, &selection, &file)))
}

/// O `$dumpvars` da escolha `selection` na árvore `root`, com os avisos
/// dos itens que ela não tem ([`dump_choice`]).
fn choice_for(root: &SignalScope, selection: &[String], file: &Utf8Path) -> DumpChoice {
    let (known, unknown) = split_known(root, selection);
    let mut notes: Vec<Diagnostic> = unknown
        .iter()
        .map(|item| {
            note(
                format!("{item} is in the signal choice, but not in the design anymore: it was left out of the waveform"),
                file,
            )
        })
        .collect();
    if known.is_empty() {
        notes.push(note(
            "No signal of the choice is in the design anymore: the waveform records every signal"
                .into(),
            file,
        ));
        return DumpChoice {
            calls: format!("begin $dumpvars(0, {}); end", verilog_path(&root.path)),
            notes,
        };
    }
    let mut processors = Vec::new();
    root.processors(&mut processors);
    let mut calls = format!(
        "begin $dumpvars(0, {});",
        known
            .iter()
            .map(|p| verilog_path(p))
            .collect::<Vec<_>>()
            .join(", ")
    );
    for scope in processors {
        if !known.iter().any(|k| covers(k, &scope.path)) {
            calls.push_str(&format!(" $dumpvars(1, {});", verilog_path(&scope.path)));
        }
    }
    calls.push_str(" end");
    DumpChoice { calls, notes }
}

fn note(message: String, file: &Utf8Path) -> Diagnostic {
    Diagnostic {
        tool: Tool::Vvp,
        severity: Severity::Warning,
        raw: message.clone(),
        message,
        file: Some(file.to_owned()),
        line: None,
        column: None,
    }
}

/// Um caminho da árvore como referência hierárquica do Verilog: cada parte
/// que não é um identificador simples (com um índice de `generate`, `g[0]`)
/// vai escapada (`\nome `).
fn verilog_path(path: &str) -> String {
    path.split('.')
        .map(|part| {
            let (base, index) = match part.find('[') {
                Some(i) if part.ends_with(']') => (&part[..i], &part[i..]),
                _ => (part, ""),
            };
            let simple = base
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && base
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
                && index
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .chars()
                    .all(|c| c.is_ascii_digit());
            if simple {
                part.to_owned()
            } else {
                format!("\\{part} ")
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

impl DumpChoice {
    /// O testbench com a escolha: o primeiro `$dumpvars` vira
    /// [`calls`](Self::calls) e os outros viram comentário; sem `$dumpvars`,
    /// a escolha entra logo depois do primeiro `$dumpfile`. As linhas do
    /// arquivo continuam as mesmas. `None` sem `$dumpvars` nem `$dumpfile`.
    pub(crate) fn apply(&self, testbench: &str) -> Option<String> {
        let calls = system_calls(testbench, "$dumpvars");
        if calls.is_empty() {
            let (_, end) = *system_calls(testbench, "$dumpfile").first()?;
            return Some(format!(
                "{} {}{}",
                &testbench[..end],
                self.calls,
                &testbench[end..]
            ));
        }
        let mut out = testbench.to_owned();
        for (n, &(start, end)) in calls.iter().enumerate().rev() {
            let original = &testbench[start..end];
            let lines = "\n".repeat(original.matches('\n').count());
            let replacement = if n == 0 {
                format!("{}{lines}", self.calls)
            } else {
                format!(
                    "/* AURORA, {}: {} */",
                    WAVE_DIR,
                    original.replace("*/", "* /")
                )
            };
            out.replace_range(start..end, &replacement);
        }
        Some(out)
    }
}

/// Onde estão as chamadas de `name` (`$dumpvars`, `$dumpfile`) no texto,
/// fora de comentário e de texto entre aspas: do nome até o `;` depois dos
/// argumentos, inclusive.
fn system_calls(text: &str, name: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
            }
            b'"' => i = skip_string(bytes, i),
            b'$' if text[i..].starts_with(name)
                && !bytes
                    .get(i + name.len())
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_') =>
            {
                let start = i;
                let mut j = i + name.len();
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if bytes.get(j) == Some(&b'(') {
                    let mut depth = 0;
                    while j < bytes.len() {
                        match bytes[j] {
                            b'(' => depth += 1,
                            b')' => {
                                depth -= 1;
                                if depth == 0 {
                                    j += 1;
                                    break;
                                }
                            }
                            b'"' => {
                                j = skip_string(bytes, j);
                                continue;
                            }
                            _ => {}
                        }
                        j += 1;
                    }
                    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                        j += 1;
                    }
                }
                if bytes.get(j) == Some(&b';') {
                    found.push((start, j + 1));
                }
                i = j.max(i + 1);
            }
            _ => i += 1,
        }
    }
    found
}

/// O índice logo depois do texto entre aspas que começa em `i`.
fn skip_string(bytes: &[u8], mut i: usize) -> usize {
    i += 1;
    while i < bytes.len() && bytes[i] != b'"' && bytes[i] != b'\n' {
        i += if bytes[i] == b'\\' { 2 } else { 1 };
    }
    i + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    const VVP: &str = r#"S_0000027b24389420 .scope module, "tb_proc_placa" "tb_proc_placa" 2 12;
 .timescale -9 -12;
v0000027b243fd3a0_0 .var "CLOCK_50", 0 0;
v0000027b243fc900_0 .net "HEX0", 6 0, v0000027b243f8270_0;  1 drivers
v0000027b243fd620_0 .var "KEY", 1 0;
v0000027b243fbe60_0 .var "SW", 17 0;
S_0000027b24389670 .scope module, "dut" "proc_placa" 2 20, 3 12 0, S_0000027b24389420;
 .timescale -9 -12;
    .port_info 0 /INPUT 1 "CLOCK_50";
    .port_info 1 /INPUT 2 "KEY";
    .port_info 3 /OUTPUT 7 "HEX0";
P_0000027b243967c0 .param/l "N" 0 3 12, +C4<00000000000000000000000000000100>;
v0000027b243fd6c0_0 .net "CLOCK_50", 0 0, v0000027b243fd3a0_0;  1 drivers
v0000027b243fb820_0 .net "HEX0", 6 0, v0000027b243f8270_0;  alias, 1 drivers
v0000027b243fcb80_0 .net "KEY", 1 0, v0000027b243fd620_0;  1 drivers
v0000027b243fc4a0_0 .net/2u *"_ivl_10", 6 0, L_0000027b246000d0;  1 drivers
v0000027b243fc180_0 .net/s "acc", 15 0, v0000027b243f8450_0;  1 drivers
S_0000027b2438a480 .scope module, "u_proc" "my_proc" 3 22, 4 12 0, S_0000027b24389670;
v0000027b2435e9f0_0 .var "valr2", 31 0;
v0000027b2435eb30_0 .var "linetabs", 31 0;
v0000027b2435eb31_0 .var/real "me2_f_main_v_x_e_", 0 0;
v0000027b2435eb32_0 .var/i "i", 31 0;
S_0000027b2438a481 .scope task, "envia" "envia" 4 40, 4 40 0, S_0000027b2438a480;
v0000027b2435eb33_0 .var "t", 0 0;
S_0000027b2438a482 .scope generate, "g[0]" "g[0]" 3 30, 3 30 0, S_0000027b24389670;
v0000027b2435eb34_0 .var "z", 3 0;
"#;

    #[test]
    fn the_tree_comes_from_the_scopes_and_signals_of_the_vvp() {
        let root = parse_signals(VVP, "tb_proc_placa").unwrap();
        assert_eq!(root.path, "tb_proc_placa");
        assert_eq!(root.kind, ScopeKind::Module);
        let names: Vec<&str> = root.signals.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["CLOCK_50", "HEX0", "KEY", "SW"]);
        assert_eq!(root.signals[3].width, 18);
        assert_eq!(root.signals[1].kind, SignalKind::Wire);

        let dut = &root.scopes[0];
        assert_eq!(dut.path, "tb_proc_placa.dut");
        assert_eq!(dut.module.as_deref(), Some("proc_placa"));
        // Sem o `_ivl_10` do Icarus; com as portas.
        let names: Vec<&str> = dut.signals.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["CLOCK_50", "HEX0", "KEY", "acc"]);
        assert_eq!(dut.signals[0].direction, Some(PortDirection::Input));
        assert_eq!(dut.signals[1].direction, Some(PortDirection::Output));
        assert_eq!(dut.signals[3].direction, None);

        // O processador, sem a tarefa; o `generate` com o índice no nome.
        let proc = &dut.scopes[0];
        assert!(proc.processor);
        assert!(proc.scopes.is_empty(), "{:?}", proc.scopes);
        let real = proc
            .signals
            .iter()
            .find(|s| s.kind == SignalKind::Real)
            .unwrap();
        assert_eq!(real.width, 64);
        assert!(proc.signals.iter().any(|s| s.kind == SignalKind::Integer));
        assert_eq!(dut.scopes[1].kind, ScopeKind::Generate);
        assert_eq!(dut.scopes[1].path, "tb_proc_placa.dut.g[0]");
    }

    #[test]
    fn the_choice_is_normalized_and_checked_against_the_tree() {
        let list = |items: &[&str]| items.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            normalize(&list(&[
                " tb.dut.q",
                "tb.dut",
                "tb.clk",
                "tb.dut",
                "",
                "tb.dutx"
            ])),
            ["tb.clk", "tb.dut", "tb.dutx"]
        );
        assert!(covers("tb.dut", "tb.dut.q"));
        assert!(!covers("tb.dut", "tb.dutx"));

        let root = parse_signals(VVP, "tb_proc_placa").unwrap();
        let (known, unknown) = split_known(
            &root,
            &list(&[
                "tb_proc_placa.SW",
                "tb_proc_placa.dut.g[0]",
                "tb_proc_placa.velho",
            ]),
        );
        assert_eq!(known, ["tb_proc_placa.SW", "tb_proc_placa.dut.g[0]"]);
        assert_eq!(unknown, ["tb_proc_placa.velho"]);
    }

    #[test]
    fn taking_out_a_path_inside_a_chosen_scope_keeps_the_rest_of_it() {
        let root = parse_signals(VVP, "tb_proc_placa").unwrap();
        let list = |items: &[&str]| items.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(root.contains("tb_proc_placa.dut.acc"));
        assert!(root.contains("tb_proc_placa.dut.g[0]"));
        assert!(!root.contains("tb_proc_placa.dut.nada"));
        assert!(!root.contains("outro.dut"));
        assert_eq!(
            root.without(
                &list(&["tb_proc_placa.SW", "tb_proc_placa.KEY"]),
                "tb_proc_placa.SW"
            ),
            Some(list(&["tb_proc_placa.KEY"]))
        );
        assert_eq!(
            root.without(
                &list(&["tb_proc_placa.dut"]),
                "tb_proc_placa.dut.u_proc.valr2"
            ),
            Some(list(&[
                "tb_proc_placa.dut.CLOCK_50",
                "tb_proc_placa.dut.HEX0",
                "tb_proc_placa.dut.KEY",
                "tb_proc_placa.dut.acc",
                "tb_proc_placa.dut.g[0]",
                "tb_proc_placa.dut.u_proc.i",
                "tb_proc_placa.dut.u_proc.linetabs",
                "tb_proc_placa.dut.u_proc.me2_f_main_v_x_e_",
            ]))
        );
        assert_eq!(
            root.without(&list(&["tb_proc_placa.SW"]), "tb_proc_placa.KEY"),
            None
        );
    }

    #[test]
    fn a_processor_outside_the_choice_comes_with_its_own_scope() {
        let root = parse_signals(VVP, "tb_proc_placa").unwrap();
        let file = Utf8Path::new("wave/tb_proc_placa.json");
        let list = |items: &[&str]| items.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let choice = choice_for(
            &root,
            &list(&["tb_proc_placa.SW", "tb_proc_placa.velho"]),
            file,
        );
        assert_eq!(
            choice.calls,
            "begin $dumpvars(0, tb_proc_placa.SW); $dumpvars(1, tb_proc_placa.dut.u_proc); end"
        );
        assert_eq!(choice.notes.len(), 1);
        assert!(choice.notes[0].message.contains("tb_proc_placa.velho"));
        // Com o processador dentro de um escopo escolhido, nada a mais.
        let choice = choice_for(&root, &list(&["tb_proc_placa.dut"]), file);
        assert_eq!(choice.calls, "begin $dumpvars(0, tb_proc_placa.dut); end");
        // Nada que exista: todos os sinais, com o aviso.
        let choice = choice_for(&root, &list(&["tb_proc_placa.velho"]), file);
        assert_eq!(choice.calls, "begin $dumpvars(0, tb_proc_placa); end");
        assert_eq!(choice.notes.len(), 2);
    }

    #[test]
    fn verilog_paths_escape_what_is_not_a_simple_name() {
        assert_eq!(verilog_path("tb.dut.g[0].u"), "tb.dut.g[0].u");
        assert_eq!(verilog_path("tb.a+b.q"), "tb.\\a+b .q");
    }

    #[test]
    fn the_first_dumpvars_becomes_the_choice_and_the_rest_comments() {
        let choice = DumpChoice {
            calls: "begin $dumpvars(0, tb.a); end".into(),
            notes: Vec::new(),
        };
        let tb = "module tb;\n// $dumpvars(0, tb) no comentário\ninitial begin\n  $dumpfile(\"tb.vcd\");\n  $dumpvars(0,\n    tb);\n  $display(\"$dumpvars\");\n  $dumpvars(1, tb.b);\nend\nendmodule\n";
        let out = choice.apply(tb).unwrap();
        assert_eq!(out.lines().count(), tb.lines().count(), "{out}");
        assert!(
            out.contains("  begin $dumpvars(0, tb.a); end\n\n  $display(\"$dumpvars\");"),
            "{out}"
        );
        assert!(
            out.contains("/* AURORA, wave: $dumpvars(1, tb.b); */"),
            "{out}"
        );
        assert!(out.contains("// $dumpvars(0, tb) no comentário"), "{out}");

        // Só com o `$dumpfile`: a escolha entra depois dele.
        let only_file = "module tb; initial $dumpfile(\"x.vcd\"); endmodule";
        assert_eq!(
            choice.apply(only_file).unwrap(),
            "module tb; initial $dumpfile(\"x.vcd\"); begin $dumpvars(0, tb.a); end endmodule"
        );
        assert_eq!(choice.apply("module tb; endmodule"), None);
        // `$dumpvars;` sem argumentos também é chamada.
        assert_eq!(system_calls("initial $dumpvars;", "$dumpvars"), [(8, 18)]);
        assert!(system_calls("$dumpvarsx(1);", "$dumpvars").is_empty());
    }

    #[test]
    fn the_choice_file_round_trips_and_an_empty_list_removes_it() {
        let guard = tempfile::tempdir().unwrap();
        let dir = crate::paths::canonicalize(Utf8Path::from_path(guard.path()).unwrap()).unwrap();
        let project = Project::create(&dir, "p").unwrap();
        assert!(read_selection(&project, "tb").unwrap().is_empty());
        let items = vec![
            "tb.dut".to_owned(),
            "tb.dut.q".to_owned(),
            "tb.clk".to_owned(),
        ];
        let path = write_selection(&project, "tb", &items).unwrap().unwrap();
        assert_eq!(path, project.root().join("wave/tb.json"));
        assert_eq!(
            read_selection(&project, "tb").unwrap(),
            ["tb.clk", "tb.dut"]
        );
        assert_eq!(write_selection(&project, "tb", &[]).unwrap(), None);
        assert!(!path.exists());
        assert!(!project.root().join(WAVE_DIR).exists());
        assert_eq!(file_stem("\\a b"), "a_b");
        assert_eq!(
            layout_file(&project, "tb"),
            project.root().join("wave/tb.surf.ron")
        );
    }
}
