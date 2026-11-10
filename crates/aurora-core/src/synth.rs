//! Checagem de sintaxe (Icarus), síntese (Yosys) e esquemático (o `show` do
//! Yosys com o `dot` do Graphviz). A checagem e a síntese espelham a AURORA
//! (`main/ipc/rtlViewer.ts`, `main/ipc/rtlViewer_yosys_script.js`,
//! `js/compilation/checagem_de_sintaxe.ts`).
//!
//! ```text
//! checagem: iverilog [-g2012] -tnull -Wall [-y <HDL>] <sintetizáveis...>    cwd raiz
//!           e, por testbench, o mesmo com -s <testbench> <testbench>
//!           (-g2012 só com .sv; -y só com processadores; com lint,
//!           verilator --lint-only -Wall --quiet -Wno-fatal)
//! síntese:  yosys -q -l yosys.log -s yosys_script.ys                        cwd trabalho
//!           read_verilog -setattr src "<arquivo>"   (um por arquivo)
//!           hierarchy -top <topo>
//!           proc
//!           tee -q -o "<trabalho>/check.log" check  (só laços e drivers em conflito viram aviso)
//!           setundef -zero
//!           opt_clean -purge
//!           write_json "<trabalho>/hierarchy.json"
//! esquemático: yosys -q -s <módulo>.show.ys                                    cwd trabalho
//!           read_json "<trabalho>/hierarchy.json"
//!           show -format dot -prefix <módulo> [-width] <módulo>
//!           dot -Tsvg <módulo>.dot -o <módulo>.svg                         cwd trabalho
//! ```
//!
//! A AURORA desenha com o netlistsvg (JavaScript, com um fork próprio e cerca
//! de 50 skins). A AURORA usa o `show` do Yosys e o `dot` do Graphviz, que vêm
//! no bundle, em vez de exigir Node (decisão do autor, 2026-09-29). O visual é
//! o do Graphviz, diferente do da AURORA.

use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::control::Control;
use crate::diagnostics::Diagnostic;
use crate::error::{AuroraError, Result};
use crate::files::FileRole;
use crate::pipeline::{
    Artifact, ArtifactKind, ArtifactTracker, PlannedStep, Runner, Status, Step, StepReport,
    elapsed_ms, final_status,
};
use crate::process::Invocation;
use crate::project::{Processor, Project};
use crate::stats::SynthesisStatistics;
use crate::toolchain::{Tool, Toolchain};

/// O que sintetizar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "name")]
#[non_exhaustive]
pub enum DesignTarget {
    /// O módulo de topo do projeto ([`Project::top_level`]) com todos os
    /// arquivos sintetizáveis e o Verilog de todos os processadores.
    TopLevel,
    /// Um processador compilado sozinho: `<nome>/<nome>/<nome>.v` +
    /// biblioteca.
    Processor(String),
}

/// O que [`check`] verifica.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct CheckOptions {
    /// Só os módulos deste arquivo (com o resto do projeto para resolver as
    /// instâncias). `None`: o projeto inteiro.
    pub file: Option<Utf8PathBuf>,
    /// Só este processador, compilado antes: o `<nome>.v` gerado dele e o
    /// testbench que o build gerou, no lugar do design do projeto. Com
    /// `file`, os módulos do arquivo, resolvidos com o Verilog do processador.
    pub processor: Option<String>,
    /// Também o `verilator --lint-only -Wall` (larguras, sinais sem uso).
    pub lint: bool,
}

/// O resultado de [`check`]. Não há artefatos: a verificação só diz se o
/// Verilog elabora, e o que os compiladores acharam.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct CheckResult {
    /// Os módulos elaborados como raiz: os do projeto (ou os do arquivo
    /// pedido) e cada testbench.
    pub targets: Vec<String>,
    /// `Succeeded` se tudo elabora sem erro.
    pub status: Status,
    /// O primeiro passo que falhou.
    pub failed_step: Option<Step>,
    /// Um `iverilog -t null` para o projeto, um por testbench e, com
    /// `lint`, o `verilator --lint-only`.
    pub steps: Vec<StepReport>,
    /// Os erros e avisos, com arquivo e linha.
    pub diagnostics: Vec<Diagnostic>,
    /// Quanto a verificação levou, do começo ao fim, em milissegundos.
    pub duration_ms: u64,
}

impl CheckResult {
    /// `status == Succeeded`.
    pub fn succeeded(&self) -> bool {
        self.status == Status::Succeeded
    }
}

/// O resultado de [`synthesize`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct SynthesisResult {
    /// O módulo de topo sintetizado.
    pub top: String,
    /// Como a síntese terminou.
    pub status: Status,
    /// `synthesize` quando falhou.
    pub failed_step: Option<Step>,
    /// Um passo: o `yosys`. O log completo fica em `yosys.log`, ao lado do
    /// netlist.
    pub steps: Vec<StepReport>,
    /// Os erros e avisos do Yosys, com arquivo e linha quando ele informa.
    pub diagnostics: Vec<Diagnostic>,
    /// O netlist (`hierarchy.json`).
    pub artifacts: Vec<Artifact>,
    /// `hierarchy.json`, quando a síntese terminou.
    #[schemars(with = "Option<String>")]
    pub netlist: Option<Utf8PathBuf>,
    /// Os módulos do netlist, na ordem do JSON, para escolher o que desenhar.
    pub modules: Vec<String>,
    /// O que o `stat` do Yosys contou no netlist, quando a síntese terminou
    /// e o Yosys gravou o `stat.json`.
    pub statistics: Option<SynthesisStatistics>,
    /// Quanto a síntese levou, do começo ao fim, em milissegundos.
    pub duration_ms: u64,
}

impl SynthesisResult {
    /// `status == Succeeded`.
    pub fn succeeded(&self) -> bool {
        self.status == Status::Succeeded
    }
}

/// Como desenhar o esquemático.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SchematicOptions {
    /// Escreve a largura dos barramentos nas arestas (`show -width`). Padrão:
    /// sim.
    pub bus_widths: bool,
    /// O prazo do `dot`. Padrão: [`SCHEMATIC_TIMEOUT`]. `None`: sem prazo.
    /// Não há teto de ligações: um módulo grande desenha, por mais que o
    /// `dot` demore, até o prazo.
    pub timeout: Option<Duration>,
}

/// O prazo padrão do `dot` ([`SchematicOptions::timeout`]).
pub const SCHEMATIC_TIMEOUT: Duration = Duration::from_secs(60);

impl Default for SchematicOptions {
    fn default() -> Self {
        SchematicOptions {
            bus_widths: true,
            timeout: Some(SCHEMATIC_TIMEOUT),
        }
    }
}

/// O resultado de [`render_schematic`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct SchematicResult {
    /// O módulo desenhado.
    pub module: String,
    /// Como terminou. Uma mensagem de erro do `dot` conta como falha mesmo
    /// que ele saia com código 0.
    pub status: Status,
    /// `graph` (Yosys) ou `render` (`dot`) quando falhou.
    pub failed_step: Option<Step>,
    /// Dois passos: `graph` (`yosys show`) e `render` (`dot`).
    pub steps: Vec<StepReport>,
    /// Mensagens do Yosys e do Graphviz.
    pub diagnostics: Vec<Diagnostic>,
    /// O grafo (`.dot`) e o SVG.
    pub artifacts: Vec<Artifact>,
    /// O SVG, quando foi gerado. Fica ao lado do netlist, com o nome do
    /// módulo (caracteres fora de `[A-Za-z0-9_.-]` viram `_`).
    #[schemars(with = "Option<String>")]
    pub svg: Option<Utf8PathBuf>,
    /// Quanto o desenho levou, do começo ao fim, em milissegundos.
    pub duration_ms: u64,
}

impl SchematicResult {
    /// `status == Succeeded`.
    pub fn succeeded(&self) -> bool {
        self.status == Status::Succeeded
    }
}

/// Verifica o Verilog do projeto com os compiladores, sem simular nem
/// sintetizar, como o botão Verilog da AURORA, porém mais completo.
///
/// Sem `options.file`:
///
/// 1. `iverilog -tnull -Wall` sobre o design inteiro (os sintetizáveis do
///    `.spf`, os `.v` de `<raiz>/TopLevel/`, legado da AURORA, e o
///    `<nome>.v` gerado de cada processador), sem `-s`: todo módulo raiz é
///    elaborado, e um erro num módulo fora da árvore do topo também aparece;
/// 2. para cada testbench registrado, `iverilog -tnull -Wall -s <testbench>`
///    com o design, que pega o erro no testbench antes de simular.
///
/// Com `options.processor`, o design é só o `<nome>.v` gerado do
/// processador, e o testbench é o que o build gerou
/// (`<nome>/<nome>/<nome>_tb.v`); os passos são os mesmos, e o lint usa o
/// processador como topo.
///
/// Com `options.file`, só os módulos daquele arquivo como raiz (com o design
/// para resolver as instâncias); se o arquivo é testbench, a elaboração dele
/// com o design. Com `options.lint`, depois, `verilator --lint-only -Wall`
/// no design (larguras, sinais sem uso), numa pasta vazia, porque o
/// Verilator procura módulos no diretório atual.
///
/// `-g2012` entra quando há um `.sv`. A biblioteca SAPHO entra por `-y` só
/// quando o projeto tem processadores ([`Toolchain::sapho_library`]), que
/// precisam ter sido compilados antes ([`build`](crate::build)). Avisos não
/// reprovam; um passo que falha interrompe os seguintes.
///
/// # Erros
///
/// - [`AuroraError::EmptyProject`]: sem arquivos Verilog e sem processadores;
/// - [`AuroraError::InvalidProject`]: um arquivo registrado ou o `file`
///   pedido não existe;
/// - [`AuroraError::NotBuilt`]: nada para verificar porque nenhum processador
///   foi compilado, um arquivo registrado que o build de um processador gera
///   e ainda não existe, ou, com `processor`, o processador não compilado.
///   `check` não compila: verifica o que está no disco, e um processador não
///   compilado num projeto com outro Verilog fica de fora, com um aviso;
/// - [`AuroraError::ModuleNotFound`]: o `file` não declara módulo;
/// - [`AuroraError::ProcessorNotFound`] / [`AuroraError::NotBuilt`]: com
///   `processor`, processador inexistente ou não compilado;
/// - ferramenta ausente ou que não executa, como nas demais operações (o
///   `lint` precisa do Verilator e do compilador do sistema);
/// - [`AuroraError::OperationInProgress`]: com `lint`, outra verificação com
///   lint do projeto está rodando.
pub fn check(
    toolchain: &Toolchain,
    project: &Project,
    options: &CheckOptions,
    control: &Control,
) -> Result<CheckResult> {
    let _span = tracing::info_span!("check").entered();
    let started = Instant::now();
    // O check não compila: o Verilog de um processador com fonte que nunca
    // foi compilado fica de fora, com um aviso. Com um arquivo pedido, só ele
    // importa, e uma instância do processador acusa o que falta.
    let unbuilt: Vec<&Processor> = if options.processor.is_none() && options.file.is_none() {
        project
            .buildable_processors()
            .into_iter()
            .filter(|p| !p.verilog_path().is_file())
            .collect()
    } else {
        Vec::new()
    };
    // O design, os testbenches elaborados com ele e o topo do lint.
    let (design, testbenches, mut lint_top) = match &options.processor {
        Some(name) => {
            let verilog = processor_verilog(project, name)?;
            let testbench = project.require_processor(name)?.testbench_path();
            let testbenches = if testbench.is_file() {
                vec![testbench]
            } else {
                Vec::new()
            };
            (vec![verilog], testbenches, Some(name.clone()))
        }
        None => {
            let mut testbenches = Vec::new();
            for tb in project.files(FileRole::Testbench) {
                // Um testbench cocotb é Python: o check elabora Verilog.
                if crate::cocotb::is_testbench(&tb.path) {
                    continue;
                }
                if !tb.path.is_file() {
                    return Err(missing_registered(
                        project,
                        tb.path,
                        "Testbench added to the project does not exist",
                    ));
                }
                testbenches.push(tb.path);
            }
            (
                project_sources(project)?,
                testbenches,
                project.top_module().ok().flatten(),
            )
        }
    };
    // O VHDL vai para o GHDL, à parte: os sintetizáveis VHDL (sem
    // processador pedido) e os testbenches VHDL.
    let (vhdl_testbenches, testbenches): (Vec<_>, Vec<_>) = testbenches
        .into_iter()
        .partition(|t| crate::hdl::is_vhdl(t));
    let vhdl_design = if options.processor.is_none() {
        crate::ghdl::design_files(project)?
    } else {
        Vec::new()
    };
    if design.is_empty()
        && testbenches.is_empty()
        && vhdl_design.is_empty()
        && vhdl_testbenches.is_empty()
        && options.file.is_none()
    {
        // Nada para verificar porque nenhum processador foi compilado: a
        // recusa diz qual compilar.
        if let Some(processor) = unbuilt.first() {
            return Err(AuroraError::NotBuilt {
                processor: processor.name.clone(),
                missing: processor.verilog_path(),
            });
        }
        return Err(AuroraError::EmptyProject(project.spf_path().to_owned()));
    }
    let library = toolchain.sapho_library(!project.processors().is_empty())?;
    let systemverilog = design
        .iter()
        .chain(&testbenches)
        .chain(options.file.iter())
        .any(|f| f.extension() == Some("sv"));
    let iverilog = || -> Result<Invocation> {
        let mut invocation = toolchain.invocation(Tool::Iverilog, project.root())?;
        if systemverilog {
            invocation = invocation.arg("-g2012");
        }
        invocation = include_paths(invocation.arg("-tnull").arg("-Wall"), None, project.root());
        if let Some(library) = &library {
            invocation = invocation.arg("-y").icarus_path_arg(library);
        }
        Ok(invocation)
    };

    let mut runner = Runner::new(control);
    let mut targets = Vec::new();
    // Para o lint: o que entra.
    let mut lint_files = design.clone();
    match &options.file {
        None => {
            if !design.is_empty() {
                let roots = design_roots(&design, lint_top.as_deref());
                let mut invocation = iverilog()?;
                // Com recursão parametrizada nenhum módulo sobra como raiz, e
                // o Icarus também não acharia: as raízes vão com `-s`.
                if roots.explicit {
                    for root in &roots.names {
                        invocation = invocation.arg("-s").arg(root);
                    }
                }
                targets.extend(roots.names);
                for file in &design {
                    invocation = invocation.icarus_path_arg(file);
                }
                runner.run(PlannedStep::new(
                    Step::CheckSyntax,
                    Tool::Iverilog,
                    invocation,
                ))?;
            }
            for testbench in &testbenches {
                if runner.status != Status::Succeeded {
                    break;
                }
                let module = crate::files::testbench_module_of(testbench)?;
                let mut invocation = iverilog()?.arg("-s").arg(&module);
                for file in &design {
                    invocation = invocation.icarus_path_arg(file);
                }
                invocation = invocation.icarus_path_arg(testbench);
                targets.push(module);
                runner.run(PlannedStep::new(
                    Step::CheckSyntax,
                    Tool::Iverilog,
                    invocation,
                ))?;
            }
            if runner.status == Status::Succeeded
                && (!vhdl_design.is_empty() || !vhdl_testbenches.is_empty())
            {
                let mut units = crate::ghdl::design_roots(&vhdl_design)?;
                for testbench in &vhdl_testbenches {
                    units.push(crate::files::testbench_module_of(testbench)?);
                }
                let mut files = vhdl_design.clone();
                files.extend(vhdl_testbenches.iter().cloned());
                crate::ghdl::check(
                    toolchain,
                    project,
                    &files,
                    &units,
                    &mut runner,
                    &mut targets,
                )?;
            }
        }
        Some(file) if crate::hdl::is_vhdl(file) => {
            let file = crate::paths::normalize(&project.root().join(file));
            if !file.is_file() {
                return Err(AuroraError::InvalidProject {
                    path: file,
                    reason: "The file to check does not exist".into(),
                });
            }
            let unit = crate::files::testbench_module_of(&file)?;
            let mut files = vhdl_design.clone();
            if !files.iter().any(|f| crate::paths::same_file(f, &file)) {
                files.push(file.clone());
            }
            lint_files = Vec::new();
            crate::ghdl::check(
                toolchain,
                project,
                &files,
                &[unit],
                &mut runner,
                &mut targets,
            )?;
        }
        Some(file) => {
            let file = crate::paths::normalize(&project.root().join(file));
            if crate::cocotb::is_testbench(&file) {
                return Err(AuroraError::InvalidName {
                    name: file.file_name().unwrap_or_default().to_owned(),
                    reason: "check verifies Verilog; a cocotb testbench (.py) runs its tests when it is simulated".into(),
                });
            }
            if !file.is_file() {
                return Err(AuroraError::InvalidProject {
                    path: file,
                    reason: "The file to check does not exist".into(),
                });
            }
            let text = std::fs::read_to_string(&file).map_err(AuroraError::io("Reading", &file))?;
            let modules = crate::verilog::modules_in(&text);
            if modules.is_empty() {
                return Err(AuroraError::ModuleNotFound {
                    name: file.file_name().unwrap_or_default().to_owned(),
                    available: Vec::new(),
                });
            }
            let registered_tb = testbenches.contains(&file);
            let registered_design = design.contains(&file);
            let is_testbench = registered_tb
                || (!registered_design
                    && crate::verilog::classify(&text, file.file_name().unwrap_or_default())
                        == FileRole::Testbench);
            let roots = if is_testbench {
                vec![crate::files::testbench_module_of(&file)?]
            } else {
                modules
            };
            let mut invocation = iverilog()?;
            for root in &roots {
                invocation = invocation.arg("-s").arg(root);
            }
            let mut files = design.clone();
            if !files.contains(&file) {
                files.push(file.clone());
            }
            for f in &files {
                invocation = invocation.icarus_path_arg(f);
            }
            if is_testbench {
                // O testbench não vai para o lint (o Verilator reclama de
                // tudo que é de testbench); só o design.
                lint_files = design.clone();
            } else {
                lint_files = files.clone();
                lint_top = roots.first().cloned();
            }
            targets = roots;
            runner.run(PlannedStep::new(
                Step::CheckSyntax,
                Tool::Iverilog,
                invocation,
            ))?;
        }
    }

    let mut diagnostics = Vec::new();
    let verilator = options.lint && runner.status == Status::Succeeded && !lint_files.is_empty();
    let verilator = match verilator.then(|| toolchain.tool(Tool::Verilator)) {
        // Sem o Verilator, a verificação vale do mesmo jeito: só o lint fica
        // de fora, com um aviso.
        Some(Err(AuroraError::ComponentMissing(component))) => {
            diagnostics.push(crate::diagnostics::Diagnostic {
                tool: Tool::Verilator,
                severity: crate::diagnostics::Severity::Warning,
                message: format!("The lint did not run: component {component} is not installed"),
                file: None,
                line: None,
                column: None,
                raw: String::new(),
            });
            false
        }
        Some(Err(error)) => return Err(error),
        Some(Ok(_)) => true,
        None => false,
    };
    if verilator {
        let dir = project.temp_dir().join("lint");
        // A pasta é apagada a cada lint: dois ao mesmo tempo (o Studio e o
        // terminal) não dividem a mesma.
        let _lock = crate::paths::run_lock(&dir, project.name())?;
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(AuroraError::io("Creating directory", &dir))?;
        let mut invocation = toolchain
            .invocation(Tool::Verilator, &dir)?
            .arg("--lint-only")
            .arg("-Wall")
            .arg("--quiet")
            .arg("-Wno-fatal");
        if let Some(top) = &lint_top {
            invocation = invocation.arg("--top-module").arg(top);
        }
        if let Some(library) = &library {
            invocation = invocation.arg("-y").tool_path_arg(library);
        }
        for file in &lint_files {
            invocation = invocation.tool_path_arg(file);
        }
        runner.run(PlannedStep::new(Step::Lint, Tool::Verilator, invocation))?;
    }

    diagnostics.extend(unbuilt.iter().map(|processor| Diagnostic {
        tool: Tool::Iverilog,
        severity: crate::diagnostics::Severity::Warning,
        message: format!(
            "Processor {} has not been built: its Verilog is not in the check",
            processor.name
        ),
        file: Some(processor.source.clone()),
        line: None,
        column: None,
        raw: String::new(),
    }));
    diagnostics.extend(runner.diagnostics);
    crate::diagnostics::tidy(&mut diagnostics, toolchain.root());
    Ok(CheckResult {
        targets,
        status: runner.status,
        failed_step: runner.failed_step,
        steps: runner.steps,
        diagnostics,
        duration_ms: elapsed_ms(started),
    })
}

/// As raízes do design ([`crate::verilog::design_roots`]), lidas dos
/// arquivos.
pub(crate) fn design_roots(
    design: &[Utf8PathBuf],
    top: Option<&str>,
) -> crate::verilog::DesignRoots {
    let texts: Vec<String> = design
        .iter()
        .filter_map(|f| std::fs::read_to_string(f).ok())
        .collect();
    crate::verilog::design_roots(&texts, top)
}

/// Sintetiza com o Yosys até um netlist JSON (`hierarchy.json`), que
/// [`render_schematic`] transforma em SVG.
///
/// A síntese é a do RTLViewer da AURORA: elabora a hierarquia, converte processos
/// em lógica e limpa, sem mapear para uma FPGA. Serve para visualizar, não
/// para estimar área. Os arquivos ficam em
/// `<projeto>/.aurora8/Temp/synth/<topo>/`: o script (`yosys_script.ys`), o log
/// (`yosys.log`) e o netlist.
///
/// Com [`DesignTarget::TopLevel`], entram os mesmos arquivos de
/// [`check`]; com [`DesignTarget::Processor`], só o Verilog do
/// processador. Nos dois casos, a biblioteca SAPHO inteira (todo `.v` de
/// `HDL/` que não é testbench) entra primeiro.
///
/// ```no_run
/// use aurora_core::*;
/// # let toolchain = Toolchain::open("/opt/aurora/toolchain")?;
/// let project = Project::open("/p/soma")?;
/// let control = Control::default();
/// let target = DesignTarget::Processor("soma".into());
/// let synth = synthesize(&toolchain, &project, &target, &control)?;
/// if let Some(netlist) = &synth.netlist {
///     let options = SchematicOptions::default();
///     let svg = render_schematic(&toolchain, netlist, "soma", &options, &control)?;
///     println!("{:?}", svg.svg);
/// }
/// # Ok::<(), aurora_core::AuroraError>(())
/// ```
///
/// # Erros
///
/// - [`AuroraError::NoTopLevel`] / [`AuroraError::InvalidName`]: como em
///   [`check`];
/// - [`AuroraError::ProcessorNotFound`] / [`AuroraError::NotBuilt`]: com
///   `DesignTarget::Processor`, processador inexistente ou não compilado;
/// - [`AuroraError::InvalidName`] também se um caminho tiver aspas ou quebra
///   de linha, que não cabem no script do Yosys;
/// - [`AuroraError::OperationInProgress`]: outra síntese ou uma simulação do
///   projeto (as duas copiam memórias para a raiz), ou o build de um
///   processador dele, está rodando.
pub fn synthesize(
    toolchain: &Toolchain,
    project: &Project,
    target: &DesignTarget,
    control: &Control,
) -> Result<SynthesisResult> {
    let started = Instant::now();
    let (top, files) = match target {
        DesignTarget::TopLevel => {
            let top = project
                .top_module()?
                .ok_or_else(|| AuroraError::NoTopLevel(project.spf_path().to_owned()))?;
            // O topo VHDL vira um módulo com o nome em minúsculas no Verilog
            // que o GHDL gera.
            (
                crate::ghdl::module_name(project, &top),
                project_sources(project)?,
            )
        }
        DesignTarget::Processor(name) => (name.clone(), vec![processor_verilog(project, name)?]),
    };
    // O VHDL do design entra como Verilog, pelo `ghdl --synth`.
    let vhdl = match target {
        DesignTarget::TopLevel => crate::ghdl::design_files(project)?,
        DesignTarget::Processor(_) => Vec::new(),
    };
    // O design do usuário, sem a biblioteca, para explicar uma queda do Yosys.
    let design = files.clone();
    let _span = tracing::info_span!("synthesize", %top).entered();
    let files = with_library(toolchain, project, files)?;

    // A raiz é a da simulação do projeto: as duas não correm juntas, nem com
    // o build de um processador.
    let _locks = crate::simulate::staging_locks(
        project,
        &project.temp_dir().join(".project"),
        project.name(),
    )?;
    let work = project.temp_dir().join("synth").join(&top);
    std::fs::create_dir_all(&work).map_err(AuroraError::io("Creating directory", &work))?;
    // O Yosys roda na raiz, e o Verilog de cada processador lê as memórias
    // pelo nome, relativo a ela.
    crate::simulate::stage_processor_files(project, project.root())?;
    let netlist = work.join("hierarchy.json");
    let stat = work.join("stat.json");
    let check_log = work.join("check.log");
    let _ = std::fs::remove_file(&check_log);
    let mut runner = Runner::new(control);
    let mut files = files;
    if !vhdl.is_empty() {
        files.extend(crate::ghdl::to_verilog(
            toolchain,
            &vhdl,
            &work.join("vhdl"),
            &mut runner,
        )?);
    }
    let script = work.join("yosys_script.ys");
    let log = work.join("yosys.log");
    std::fs::write(
        &script,
        yosys_script(&files, &top, &netlist, &stat, &check_log, project.root())?,
    )
    .map_err(AuroraError::io("Writing Yosys script", &script))?;

    let mut tracker = ArtifactTracker::new();
    tracker.expect(ArtifactKind::Netlist, &netlist, true);
    tracker.expect(ArtifactKind::SynthesisStatistics, &stat, false);
    // Na raiz do projeto, como a simulação: um `$readmemh(\"rtl/rom.mif\")`
    // relativo à raiz vale nas duas (o Yosys também procura na pasta do
    // arquivo que lê).
    let invocation = toolchain
        .invocation(Tool::Yosys, project.root())?
        .arg("-q")
        .arg("-l")
        .tool_path_arg(&log)
        .arg("-s")
        .tool_path_arg(&script);
    // Um módulo que se instancia sem `if`, `case` nem `for` no corpo nunca
    // para: o Icarus recusa, mas o Yosys gera um nível atrás do outro até
    // ser interrompido. A síntese falha antes, com o motivo.
    let endless = unconditional_recursion(&design);
    if endless.is_empty() {
        runner.run(PlannedStep::new(Step::Synthesize, Tool::Yosys, invocation))?;
    } else {
        runner.status = Status::Failed;
        runner.failed_step = Some(Step::Synthesize);
        runner.diagnostics.extend(endless);
    }

    let artifacts = tracker.finish();
    let status = final_status(runner.status, &artifacts);
    if status == Status::Succeeded {
        repair_yosys_json(&netlist)?;
    }
    let mut diagnostics = runner.diagnostics;
    if let Ok(log) = std::fs::read_to_string(&check_log) {
        diagnostics.extend(check_warnings(&log));
    }
    if status == Status::Crashed {
        diagnostics.extend(recursion_hint(&design));
    }
    let (netlist, modules) = if status == Status::Succeeded {
        let modules = netlist_modules(&netlist)?;
        (Some(netlist), modules)
    } else {
        (None, Vec::new())
    };
    // Só o `stat.json` desta síntese: um que sobrou de outra não conta.
    let statistics = artifacts
        .iter()
        .any(|a| a.kind == ArtifactKind::SynthesisStatistics && a.fresh)
        .then(|| std::fs::read_to_string(&stat).ok())
        .flatten()
        .filter(|_| status == Status::Succeeded)
        .and_then(|text| SynthesisStatistics::from_yosys_json(&text, &top));
    Ok(SynthesisResult {
        top,
        status,
        failed_step: runner.failed_step,
        steps: runner.steps,
        diagnostics,
        artifacts,
        netlist,
        modules,
        statistics,
        duration_ms: elapsed_ms(started),
    })
}

/// Desenha um módulo do netlist em SVG: o Yosys gera o grafo do módulo
/// (`show -format dot`) e o `dot` do Graphviz o desenha. Os arquivos ficam ao
/// lado do netlist: `<módulo>.show.ys`, `<módulo>.dot` e `<módulo>.svg` (no
/// Linux, também o `fonts.conf` que aponta o `dot` para as fontes do bundle,
/// e o cache dele).
///
/// Os nomes possíveis estão em [`SynthesisResult::modules`]. Submódulos
/// aparecem como caixas no desenho do módulo pai; para abrir um, chame de
/// novo com o nome dele.
///
/// # Erros
///
/// - [`AuroraError::ModuleNotFound`]: o netlist não tem o módulo (um fora da
///   árvore do topo não é sintetizado); `available` lista os que tem;
/// - [`AuroraError::InvalidNetlist`]: o netlist não é JSON;
/// - [`AuroraError::ComponentMissing`]: o bundle não tem o Yosys ou o `dot`.
pub fn render_schematic(
    toolchain: &Toolchain,
    netlist: &Utf8Path,
    module: &str,
    options: &SchematicOptions,
    control: &Control,
) -> Result<SchematicResult> {
    let _span = tracing::info_span!("render_schematic", module).entered();
    let started = Instant::now();
    let available = netlist_modules(netlist)?;
    if !available.iter().any(|m| m == module) {
        return Err(AuroraError::ModuleNotFound {
            name: module.to_owned(),
            available,
        });
    }

    let dir = netlist.parent().expect("File has a parent").to_owned();
    let stem = file_safe(module);
    let dot_file = dir.join(format!("{stem}.dot"));
    let svg = dir.join(format!("{stem}.svg"));
    // Um script por módulo: dois desenhos ao mesmo tempo não gravam um no
    // script do outro.
    let script = dir.join(format!("{stem}.show.ys"));
    let width = if options.bus_widths { " -width" } else { "" };
    // O `-prefix` do `show` não aceita aspas: vai relativo ao CWD (a pasta
    // da síntese), com o nome já sem espaço nem caractere especial.
    let text = format!(
        "read_json {}\nshow -format dot -prefix {stem}{width} {}\n",
        yosys_path(netlist, &dir)?,
        yosys_selection(module),
    );
    std::fs::write(&script, text).map_err(AuroraError::io("Writing Yosys script", &script))?;

    let mut tracker = ArtifactTracker::new();
    tracker.expect(ArtifactKind::SchematicGraph, &dot_file, true);
    tracker.expect(ArtifactKind::Schematic, &svg, true);
    let mut runner = Runner::new(control);
    let graph = toolchain
        .invocation(Tool::Yosys, &dir)?
        .arg("-q")
        .arg("-s")
        .tool_path_arg(&script);
    if runner.run(PlannedStep::new(Step::Graph, Tool::Yosys, graph))? {
        let mut render = toolchain.invocation(Tool::Dot, &dir)?;
        for (key, value) in toolchain.dot_fonts(&dir)? {
            render = render.env(key, value);
        }
        let render = render
            .arg("-Tsvg")
            .tool_path_arg(&dot_file)
            .arg("-o")
            .tool_path_arg(&svg);
        runner.run(PlannedStep::new(Step::Render, Tool::Dot, render).timeout(options.timeout))?;
        // O dot pode avisar de erro de sintaxe e sair com 0 em versões antigas.
        runner.fail_on_error_diagnostics(Tool::Dot);
    }

    let artifacts = tracker.finish();
    let status = final_status(runner.status, &artifacts);
    Ok(SchematicResult {
        module: module.to_owned(),
        status,
        failed_step: runner.failed_step,
        steps: runner.steps,
        diagnostics: runner.diagnostics,
        artifacts,
        svg: (status == Status::Succeeded).then_some(svg),
        duration_ms: elapsed_ms(started),
    })
}

/// Um nome de módulo como seleção do Yosys. Nomes gerados
/// (`$paramod\processor\NUBITS=23`) vão entre aspas, como pede a sintaxe de
/// seleção para nomes com `$`, `\\` ou `=`.
fn yosys_selection(module: &str) -> String {
    if module
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        module.to_owned()
    } else {
        format!("\"{}\"", module.replace('"', ""))
    }
}

/// O `<nome>/<nome>/<nome>.v` que o build gerou para o processador.
pub(crate) fn processor_verilog(project: &Project, name: &str) -> Result<Utf8PathBuf> {
    let processor = project.require_processor(name)?;
    let verilog = processor.verilog_path();
    if !verilog.is_file() {
        return Err(AuroraError::NotBuilt {
            processor: name.to_owned(),
            missing: verilog,
        });
    }
    Ok(verilog)
}

/// O erro de um arquivo registrado que não existe: se é um dos que o build de
/// um processador gera, o processador não foi compilado ([`AuroraError::NotBuilt`]);
/// senão, o projeto aponta para um arquivo que sumiu.
pub(crate) fn missing_registered(
    project: &Project,
    path: Utf8PathBuf,
    reason: &str,
) -> AuroraError {
    match project.generated_by(&path) {
        Some(processor) => AuroraError::NotBuilt {
            processor: processor.name.clone(),
            missing: path,
        },
        None => AuroraError::InvalidProject {
            path,
            reason: reason.into(),
        },
    }
}

/// Os fontes Verilog do projeto para checagem, síntese e hierarquia, na ordem da
/// AURORA: sintetizáveis do `.spf`, `.v` de `<raiz>/TopLevel/` (legado da
/// AURORA) e o Verilog que o build gerou para cada processador, sem
/// testbenches.
pub(crate) fn project_sources(project: &Project) -> Result<Vec<Utf8PathBuf>> {
    let mut files = Vec::new();
    for file in project.files(FileRole::Synthesizable) {
        // O VHDL vai para o GHDL ([`crate::ghdl::design_files`]): o Icarus,
        // o Verilator e o Yosys só leem Verilog.
        if crate::hdl::is_vhdl(&file.path) {
            continue;
        }
        if !file.path.is_file() {
            return Err(missing_registered(
                project,
                file.path,
                "Synthesizable file added to the project does not exist",
            ));
        }
        files.push(file.path);
    }
    let top_level_dir = project.root().join("TopLevel");
    if top_level_dir.is_dir() {
        files.extend(
            verilog_in(&top_level_dir)?
                .into_iter()
                .filter(|f| !is_test_file(f)),
        );
    }
    for processor in project.processors() {
        let verilog = processor.verilog_path();
        if verilog.is_file() {
            files.push(verilog);
        }
    }
    Ok(dedup(files))
}

/// As fontes de uma placa FPGA: as de [`with_library`] e, depois, o VHDL do
/// design, que o Quartus lê direto (`VHDL_FILE`) e mistura com o Verilog.
pub(crate) fn board_sources(toolchain: &Toolchain, project: &Project) -> Result<Vec<Utf8PathBuf>> {
    let mut files = with_library(toolchain, project, project_sources(project)?)?;
    files.extend(crate::ghdl::design_files(project)?);
    Ok(files)
}

/// `files` depois da biblioteca SAPHO, como a AURORA sintetiza: todo `.v` de
/// `SAPHO/` que não é testbench, sem repetir. Sem processadores no projeto,
/// só `files`; sem o YANC instalado, um projeto só de Verilog sintetiza sem
/// a biblioteca. É o que a síntese lê e o que vai para o Quartus.
pub(crate) fn with_library(
    toolchain: &Toolchain,
    project: &Project,
    files: Vec<Utf8PathBuf>,
) -> Result<Vec<Utf8PathBuf>> {
    let mut library = match toolchain.sapho_library(!project.processors().is_empty())? {
        Some(hdl) => verilog_in(&hdl)?,
        None => Vec::new(),
    };
    library.retain(|f| !is_test_file(f));
    library.extend(files);
    Ok(dedup(library))
}

fn verilog_in(dir: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
    let entries = dir
        .read_dir_utf8()
        .map_err(AuroraError::io("Listing directory", dir))?;
    let mut files: Vec<_> = entries
        .flatten()
        .map(|e| e.into_path())
        .filter(|p| p.extension() == Some("v") && p.is_file())
        .collect();
    // Ordem estável; a AURORA usa a ordem do sistema de arquivos.
    files.sort();
    Ok(files)
}

fn is_test_file(path: &Utf8Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_ascii_lowercase();
    name.contains("_tb") || name.contains("test")
}

/// Sem repetir, pelo caminho real: com a pasta de um processador sendo um
/// link, o Verilog dele vem do `.spf` pelo caminho real e do processador pelo
/// do link, e o Icarus e o Yosys recusariam o módulo duas vezes.
fn dedup(files: Vec<Utf8PathBuf>) -> Vec<Utf8PathBuf> {
    let mut seen = std::collections::HashSet::new();
    files
        .into_iter()
        .filter(|file| seen.insert(crate::paths::real_path(file)))
        .collect()
}

/// Um caminho entre aspas para um script do Yosys.
pub(crate) fn yosys_quoted(path: &Utf8Path) -> Result<String> {
    if path.as_str().contains(['"', '\n', '\r']) {
        return Err(AuroraError::InvalidName {
            name: path.to_string(),
            reason: "A path with quotes or line breaks cannot go into a Yosys script".into(),
        });
    }
    // O Yosys aceita `/` no Windows; `\` dentro de aspas vira escape.
    Ok(format!("\"{}\"", path.as_str().replace('\\', "/")))
}

/// Um caminho entre aspas para um script do Yosys que roda em `cwd`: no
/// Windows, relativo a ele quando tem caractere fora do ASCII
/// ([`tool_path`](crate::process::tool_path)).
pub(crate) fn yosys_path(path: &Utf8Path, cwd: &Utf8Path) -> Result<String> {
    yosys_quoted(&crate::process::tool_path(path, cwd))
}

fn yosys_script(
    files: &[Utf8PathBuf],
    top: &str,
    netlist: &Utf8Path,
    stat: &Utf8Path,
    check: &Utf8Path,
    root: &Utf8Path,
) -> Result<String> {
    // O Yosys roda na raiz: os caminhos com acento vão relativos a ela.
    let quoted = |path: &Utf8Path| yosys_path(path, root);
    let mut script = String::new();
    // `-I <raiz>`: um `include` relativo à raiz do projeto, como no Icarus
    // (`include_paths`); o relativo à pasta do arquivo o Yosys já acha.
    // `-sv` só nos `.sv`: em Verilog-2001, `logic` e `bit` podem ser nomes.
    let include = format!("-I {}", quoted(root)?);
    for file in files {
        let sv = if file.extension() == Some("sv") {
            " -sv"
        } else {
            ""
        };
        script.push_str(&format!(
            "read_verilog -setattr src {include}{sv} {}\n",
            quoted(file)?
        ));
    }
    // O `check` do Yosys acha laço combinacional, que nem o Icarus nem a
    // síntese apontam. Vai para um arquivo (`tee -q`): no processador SAPHO
    // ele avisa de cada fio sem driver da biblioteca, e só o laço e o driver
    // em conflito interessam (`check_warnings`).
    script.push_str(&format!(
        "hierarchy -top {top}\nproc\ntee -q -o {} check\nsetundef -zero\nopt_clean -purge\n",
        quoted(check)?
    ));
    script.push_str(&format!("write_json {}\n", quoted(netlist)?));
    // As estatísticas do relatório (`stats.rs`), do mesmo netlist, só no
    // arquivo: `-q` tira do log.
    script.push_str(&format!(
        "tee -q -o {} stat -json -top {top}\n",
        quoted(stat)?
    ));
    Ok(script)
}

/// Os caminhos de busca do `include` no Icarus: a pasta do arquivo que
/// inclui (`-grelative-include`, como o Yosys faz), depois `before`, se
/// houver, e por fim a raiz do projeto. Sem isso, o Icarus procura no CWD, e
/// o mesmo `include` passava na simulação e quebrava na síntese, ou o
/// contrário.
///
/// `before` é a pasta do testbench original quando o Icarus compila a cópia
/// com o dump injetado, que fica em `.aurora8/Temp`: o `include` relativo à
/// pasta dele continua achando o arquivo. Os caminhos vão com `/`
/// ([`icarus_path`](crate::process::icarus_path)), sem o que o
/// `-grelative-include` não funciona no Windows.
pub(crate) fn include_paths(
    invocation: Invocation,
    before: Option<&Utf8Path>,
    root: &Utf8Path,
) -> Invocation {
    let mut invocation = invocation.arg("-grelative-include");
    // O testbench gerado de um processador fica na pasta onde ele roda: a
    // mesma pasta duas vezes não muda a busca.
    if let Some(dir) = before.filter(|dir| *dir != root) {
        invocation = invocation.arg("-I").icarus_path_arg(dir);
    }
    invocation.arg("-I").icarus_path_arg(root)
}

/// Conserta o texto fora do ASCII que o `write_json` do Yosys estraga: cada
/// byte acima de 127 sai como `\uFFFFFFxx` (o `char` com sinal estendido),
/// o que um leitor de JSON lê como `\uFFFF` seguido de texto, e o
/// `read_json` do próprio Yosys recusa ("Unsupported \uXXXX sequence").
/// Os bytes seguidos voltam a ser o texto UTF-8 que eram.
fn repair_yosys_json(netlist: &Utf8Path) -> Result<()> {
    let text =
        std::fs::read_to_string(netlist).map_err(AuroraError::io("Reading netlist", netlist))?;
    if !text.contains("\\uFFFFFF") {
        return Ok(());
    }
    let repaired = repair_escaped_bytes(&text);
    std::fs::write(netlist, repaired).map_err(AuroraError::io("Writing netlist", netlist))
}

fn repair_escaped_bytes(text: &str) -> String {
    const MARK: &str = "\\uFFFFFF";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(MARK) {
        out.push_str(&rest[..at]);
        let mut bytes = Vec::new();
        let mut tail = &rest[at..];
        while let Some(hex) = tail.strip_prefix(MARK).and_then(|t| t.get(..2)) {
            let Ok(byte) = u8::from_str_radix(hex, 16) else {
                break;
            };
            bytes.push(byte);
            tail = &tail[MARK.len() + 2..];
        }
        if bytes.is_empty() {
            out.push_str(MARK);
            rest = &rest[at + MARK.len()..];
            continue;
        }
        out.push_str(&String::from_utf8_lossy(&bytes));
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// Os módulos do netlist que dá para desenhar: sem as caixas-pretas (um
/// módulo vazio vira `blackbox` e fica no netlist mesmo fora da árvore do
/// topo, e o `show` do Yosys não tem o que mostrar nele).
fn netlist_modules(netlist: &Utf8Path) -> Result<Vec<String>> {
    let text =
        std::fs::read_to_string(netlist).map_err(AuroraError::io("Reading netlist", netlist))?;
    let doc: Value = serde_json::from_str(&text).map_err(|e| AuroraError::InvalidNetlist {
        path: netlist.to_owned(),
        reason: e.to_string(),
    })?;
    let blackbox = |module: &Value| {
        module["attributes"]["blackbox"]
            .as_str()
            .is_some_and(|bits| bits.contains('1'))
            || module["attributes"]["blackbox"]
                .as_u64()
                .is_some_and(|n| n != 0)
    };
    Ok(doc["modules"]
        .as_object()
        .map(|m| {
            m.iter()
                .filter(|(_, module)| !blackbox(module))
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default())
}

/// Os avisos do `check` do Yosys que valem para o usuário: laço
/// combinacional e drivers em conflito, com o primeiro `source:` como
/// local. O resto (fio sem driver, que o `setundef -zero` resolve) fica no
/// `check.log`.
fn check_warnings(log: &str) -> Vec<Diagnostic> {
    let mut found: Vec<Diagnostic> = Vec::new();
    let mut open = false;
    for line in log.lines() {
        if let Some(message) = line.strip_prefix("Warning: ") {
            open = message.starts_with("found logic loop")
                || message.starts_with("multiple conflicting drivers");
            if open {
                found.push(Diagnostic {
                    tool: Tool::Yosys,
                    severity: crate::diagnostics::Severity::Warning,
                    message: message.trim_end_matches(':').to_owned(),
                    file: None,
                    line: None,
                    column: None,
                    raw: line.to_owned(),
                });
            }
            continue;
        }
        if !open || !line.starts_with(' ') {
            open = false;
            continue;
        }
        let Some(last) = found.last_mut() else {
            continue;
        };
        last.raw.push('\n');
        last.raw.push_str(line);
        // `cell $not$lp.v:1$2 ($not) source: lp.v:1.50-1.58`
        if last.file.is_none()
            && let Some(place) = line.split("source: ").nth(1)
            && let Some((file, rest)) = place.rsplit_once(':')
            && let Some(number) = rest.split(['.', '-']).next().and_then(|n| n.parse().ok())
        {
            last.file = Some(crate::paths::native_separators(file.trim()));
            last.line = Some(number);
        }
    }
    found
}

/// Um erro para cada módulo do design que se instancia sem condição
/// ([`crate::verilog::unconditional_recursion`]).
fn unconditional_recursion(design: &[Utf8PathBuf]) -> Vec<Diagnostic> {
    design
        .iter()
        .filter_map(|f| Some((f, std::fs::read_to_string(f).ok()?)))
        .flat_map(|(file, text)| {
            crate::verilog::unconditional_recursion(&text)
                .into_iter()
                .map(move |module| Diagnostic {
                    tool: Tool::Yosys,
                    severity: crate::diagnostics::Severity::Error,
                    message: format!(
                        "Module {module} instantiates itself with no if, case or for around it: the recursion never ends, and Yosys would run until stopped"
                    ),
                    file: Some(file.clone()),
                    line: None,
                    column: None,
                    raw: String::new(),
                })
        })
        .collect()
}

/// Um aviso para quando o Yosys cai (sinal 11) e um módulo do design se
/// instancia: uma recursão sem condição de parada (ou com parâmetros que
/// nunca chegam a ela) esgota a pilha do Yosys, e o resumo só diria
/// "crashed".
fn recursion_hint(design: &[Utf8PathBuf]) -> Vec<Diagnostic> {
    let texts: Vec<(Utf8PathBuf, String)> = design
        .iter()
        .filter_map(|f| Some((f.clone(), std::fs::read_to_string(f).ok()?)))
        .collect();
    texts
        .iter()
        .flat_map(|(file, text)| {
            crate::verilog::self_instantiating(text)
                .into_iter()
                .map(move |module| Diagnostic {
                    tool: Tool::Yosys,
                    severity: crate::diagnostics::Severity::Error,
                    message: format!(
                        "Yosys crashed, and module {module} instantiates itself: a recursion that never reaches its stopping condition exhausts Yosys. Check the generate condition and the parameters of each level"
                    ),
                    file: Some(file.clone()),
                    line: None,
                    column: None,
                    raw: String::new(),
                })
        })
        .collect()
}

/// Nome de módulo do Yosys (`$paramod\processor\NUBITS=...`) como nome de
/// arquivo.
fn file_safe(module: &str) -> String {
    let cleaned: String = module
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    cleaned
        .trim_matches('_')
        .chars()
        .take(120)
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_matches_aurora() {
        let script = yosys_script(
            &[
                "/h/core.v".into(),
                "C:\\p\\soma.v".into(),
                "/p/reg.sv".into(),
            ],
            "soma",
            Utf8Path::new("/t/hierarchy.json"),
            Utf8Path::new("/t/stat.json"),
            Utf8Path::new("/t/check.log"),
            Utf8Path::new("/p"),
        )
        .unwrap();
        // O da AURORA, com o `-I` da raiz e o `-sv` dos `.sv`, mais o `stat`
        // do relatório no fim.
        assert_eq!(
            script,
            "read_verilog -setattr src -I \"/p\" \"/h/core.v\"\n\
             read_verilog -setattr src -I \"/p\" \"C:/p/soma.v\"\n\
             read_verilog -setattr src -I \"/p\" -sv \"/p/reg.sv\"\n\
             hierarchy -top soma\nproc\ntee -q -o \"/t/check.log\" check\nsetundef -zero\nopt_clean -purge\nwrite_json \"/t/hierarchy.json\"\n\
             tee -q -o \"/t/stat.json\" stat -json -top soma\n"
        );
        assert!(
            yosys_script(
                &["/a\"b.v".into()],
                "x",
                Utf8Path::new("/t/h.json"),
                Utf8Path::new("/t/s.json"),
                Utf8Path::new("/t/c.log"),
                Utf8Path::new("/p"),
            )
            .is_err()
        );
    }

    #[test]
    fn names() {
        assert_eq!(
            file_safe("$paramod\\processor\\NUBITS=23"),
            "paramod_processor_NUBITS_23"
        );
        assert!(is_test_file(Utf8Path::new("x/soma_tb.v")));
        assert!(!is_test_file(Utf8Path::new("x/core.v")));
    }

    #[test]
    fn yosys_json_bytes_are_repaired() {
        // O que o `write_json` do Yosys grava para "Ação".
        let broken = r#"{"src": "/p/A\uFFFFFFC3\uFFFFFFA7\uFFFFFFC3\uFFFFFFA3o/x.v:4.1-4.2"}"#;
        let fixed = repair_escaped_bytes(broken);
        assert_eq!(fixed, r#"{"src": "/p/Ação/x.v:4.1-4.2"}"#);
        let doc: Value = serde_json::from_str(&fixed).unwrap();
        assert_eq!(doc["src"], "/p/Ação/x.v:4.1-4.2");
        assert_eq!(repair_escaped_bytes("sem nada"), "sem nada");
    }

    #[test]
    fn logic_loops_come_from_the_check_log() {
        let log = "\n4. Executing CHECK pass (checking for obvious problems).\n\
                   Checking module lp...\n\
                   Warning: found logic loop in module lp:\n\
                   \x20   cell $not$lp.v:1$2 ($not) source: /p/lp.v:1.50-1.58\n\
                   \x20     A[0] --> Y[0]\n\
                   Warning: Wire laco.\\in [9] is used but has no driver.\n\
                   Found and reported 2 problems.\n";
        let found = check_warnings(log);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].message, "found logic loop in module lp");
        assert_eq!(found[0].file.as_deref(), Some(Utf8Path::new("/p/lp.v")));
        assert_eq!(found[0].line, Some(1));
    }

    #[test]
    fn blackboxes_are_not_offered_for_the_schematic() {
        let dir = tempfile::tempdir().unwrap();
        let netlist = Utf8PathBuf::from_path_buf(dir.path().join("hierarchy.json")).unwrap();
        std::fs::write(
            &netlist,
            r#"{"modules": {"top": {"attributes": {}}, "vazio": {"attributes": {"blackbox": "00000000000000000000000000000001"}}}}"#,
        )
        .unwrap();
        assert_eq!(netlist_modules(&netlist).unwrap(), ["top"]);
    }
}
