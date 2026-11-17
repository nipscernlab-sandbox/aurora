//! Testbenches em Python, com o cocotb.
//!
//! Um `.py` em `testbenchFiles` é um testbench cocotb, como na AURORA
//! (`js/compilation/cocotb_da_onda.ts`): o arquivo só tem os testes
//! (`@cocotb.test()`), e quem monta a simulação é a AURORA. Os testes recebem
//! como `dut` o módulo da diretiva `# aurora-toplevel: <módulo>` do `.py`
//! ([`toplevel_directive`]) ou, sem ela, o topo do projeto, com um aviso. O
//! nome do arquivo é o nome do módulo Python que o cocotb importa, então
//! precisa ser um identificador (`test_somador.py`, não `test-somador.py`).
//!
//! A simulação ([`simulate_project`](crate::simulate_project)) tem os passos
//! de um testbench Verilog, no Icarus ou no Verilator:
//!
//! ```text
//! Icarus
//! elaborate  iverilog [-g2012] -f <build>/cmds.f [-y <SAPHO>] -s <dut> -s aurora_cocotb_dump
//!            -o <build>/<dut>.vvp <design> <build>/aurora_cocotb_dump.v          cwd raiz
//! simulate   vvp -n [-i] -m <VPI do cocotb> <build>/<dut>.vvp -fst             cwd raiz
//!
//! Verilator
//! verilate   verilator --cc --exe --build --vpi --public-flat-rw --prefix Vtop
//!            -o V<dut> --timescale 1ns/1ps -LDFLAGS <VPI do cocotb> --trace ...
//!            --top-module <dut> -Mdir <build>/obj_dir_<dut> <verilator.cpp> <design>
//! simulate   <build>/obj_dir_<dut>/V<dut> --trace --trace-file <raiz>/<teste>.vcd   cwd raiz
//! ```
//!
//! em que `<build>` é `.aurora8/Temp/cocotb/<módulo de teste>/`. O `cmds.f`
//! traz `+timescale+1ns/1ps` (no Verilator, `--timescale`), o padrão do
//! runner do cocotb, para os módulos sem `` `timescale ``: sem ele, o
//! `Timer(1, "ns")` de um teste falha num design sem `` `timescale ``, cuja
//! precisão é de 1 s. No Icarus, o `aurora_cocotb_dump` grava a onda na raiz,
//! `<módulo de teste>.fst`, com todos os sinais do DUT; no Verilator, quem
//! grava é o `main` do cocotb (`verilator.cpp`), em `<módulo de teste>.vcd`,
//! com as outras flags da AURORA (`--timing`, `-O3`; ver
//! [`simulate`](crate::simulate)). A VPI do cocotb sobe o Python do bundle
//! dentro da simulação e roda os testes; o resultado sai em
//! `<build>/results.xml` (JUnit), que vira [`TestReport`], e cada teste que
//! falha vira um diagnóstico de erro no `.py`.
//!
//! Na simulação rápida ([`SimulationOptions::fast`](crate::SimulationOptions::fast))
//! os testes rodam sem onda: no Icarus sem o `aurora_cocotb_dump` e com `vvp
//! -none`, no Verilator com o modelo compilado sem `--trace`, em
//! `<build>/obj_dir_fast_<dut>`.
//!
//! O que o simulador precisa para carregar o cocotb (a VPI, a biblioteca do
//! Python e as variáveis de ambiente, que mudam do cocotb 1 para o 2) vem de
//! uma sonda rodada com o Python do componente `cocotb` (`cocotb_probe.py`),
//! guardada em `.aurora8/Temp/cocotb/probe.json` até o bundle mudar. Os `.pyc`
//! vão para a pasta de cache do usuário (`PYTHONPYCACHEPREFIX`): nem o
//! bundle, que pode ser só leitura, nem a pasta do projeto ganham
//! `__pycache__`. Sem isso, o cocotb do bundle, que não vem compilado, levava
//! de 4 a 5 s a mais em cada simulação num bundle só leitura (medido no
//! Windows em 2026-10-06: 5,3 s na primeira sonda, 0,5 s nas seguintes).
//!
//! No Windows, o Python que roda dentro do modelo do Verilator não acharia as
//! DLLs das extensões dele (a do `zlib`, que o `binascii` carrega): o Python
//! não as procura no `PATH`, e o modelo, ao contrário do `vvp`, não fica na
//! pasta do Python. A AURORA põe no `PYTHONPATH` um `sitecustomize.py` que
//! acrescenta essa pasta (`os.add_dll_directory`).

use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::control::Control;
use crate::diagnostics::{Diagnostic, Severity};
use crate::error::{AuroraError, Result};
use crate::process::{Termination, Watch};
use crate::project::Project;
use crate::simulate::Simulator;
use crate::toolchain::{Tool, Toolchain};
use crate::verilog::{ModuleInterface, Port, PortDirection, is_clock, reset_polarity};

/// O módulo que grava a onda de um testbench cocotb no Icarus.
pub(crate) const DUMP_MODULE: &str = "aurora_cocotb_dump";

/// O padrão de tempo do runner do cocotb, para os módulos sem `` `timescale ``:
/// `+timescale+` no `iverilog`, `--timescale` no Verilator.
pub(crate) const TIMESCALE: &str = "1ns/1ps";

/// Quanto a sonda pode levar. A primeira compila os `.pyc` do cocotb (uns 5
/// s); as outras, menos de 1 s.
const PROBE_TIMEOUT: Duration = Duration::from_secs(120);

/// A sonda, rodada com o Python do componente `cocotb`.
const PROBE_SOURCE: &str = include_str!("cocotb_probe.py");

/// Por que um nome de arquivo não serve para testbench cocotb.
pub(crate) const MODULE_NAME_REASON: &str = "A cocotb testbench is imported as a Python module: the file name must be a Python identifier (letters, digits and _, not starting with a digit)";

/// O resultado dos testes de um testbench cocotb, lido do `results.xml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct TestReport {
    /// O `results.xml` que o cocotb gravou.
    #[schemars(with = "String")]
    pub results: Utf8PathBuf,
    /// Os testes, na ordem em que rodaram.
    pub cases: Vec<TestCase>,
    /// Quantos passaram.
    pub passed: u32,
    /// Quantos falharam.
    pub failed: u32,
    /// Quantos não rodaram (`@cocotb.test(skip=True)`).
    pub skipped: u32,
}

/// Um teste (`@cocotb.test()`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct TestCase {
    /// `<módulo>.<teste>`, como o cocotb mostra (`test_somador.basic_test`).
    pub name: String,
    /// Como terminou.
    pub status: TestStatus,
    /// Por que falhou (a mensagem da exceção) ou não rodou; `null` quando
    /// passou.
    pub message: Option<String>,
    /// O `.py` do teste.
    #[schemars(with = "Option<String>")]
    pub file: Option<Utf8PathBuf>,
    /// Numa falha, a linha mais funda do traceback dentro do `.py`; senão, a
    /// linha do teste.
    pub line: Option<u32>,
}

/// Como um teste terminou. Em JSON: `"passed"`, `"failed"` ou `"skipped"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TestStatus {
    /// Passou.
    Passed,
    /// Falhou: um `assert` ou uma exceção.
    Failed,
    /// Não rodou.
    Skipped,
}

/// `path` é um testbench cocotb: termina em `.py`.
pub fn is_testbench(path: &Utf8Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("py"))
}

/// O módulo da diretiva `# aurora-toplevel: <módulo>` de um testbench
/// cocotb, a da AURORA (`parseCocotbToplevelDirective`, em
/// `js/compilation/compilation_helpers.ts`): uma linha só de comentário, com
/// `:` ou `=`, sem distinção de caixa. Para o Python é comentário, então o
/// mesmo `.py` roda fora da AURORA. `None` sem ela.
///
/// ```
/// use aurora_core::cocotb::toplevel_directive;
/// let source = "# aurora-toplevel: alu\nimport cocotb\n";
/// assert_eq!(toplevel_directive(source), Some("alu".into()));
/// assert_eq!(toplevel_directive("x = 1  # aurora-toplevel: alu"), None);
/// ```
pub fn toplevel_directive(source: &str) -> Option<String> {
    const KEY: &str = "aurora-toplevel";
    let blank = [' ', '\t'];
    source.lines().find_map(|line| {
        let rest = line.trim_start_matches(blank).strip_prefix('#')?;
        let rest = rest.trim_start_matches(blank);
        if !rest.get(..KEY.len())?.eq_ignore_ascii_case(KEY) {
            return None;
        }
        let rest = rest[KEY.len()..].trim_start_matches(blank);
        let rest = rest.strip_prefix([':', '='])?.trim_start_matches(blank);
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        let name = &rest[..end];
        name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            .then(|| name.to_owned())
    })
}

/// O testbench-modelo cocotb `<stem>.py`.
///
/// Com `dut`: a diretiva `# aurora-toplevel:` com o módulo e um teste que
/// liga o clock (porta `clk` ou `clock`, 10 ns), segura o reset nos
/// primeiros 20 ns (`rst`, `reset`, ou `rst_n` ativo em baixo), põe as outras
/// entradas em zero, espera e escreve as saídas no log. Sem `dut`, o teste
/// só espera, e os testes recebem o topo do projeto.
///
/// Vale para o cocotb 1 e o 2: `Timer` e `Clock` recebem a unidade como
/// argumento posicional, que se chama `units` num e `unit` no outro.
pub fn testbench_template(dut: Option<&ModuleInterface>) -> String {
    let Some(dut) = dut else {
        return "\
# Testbench cocotb. Sem a linha `# aurora-toplevel: <módulo>`, os testes
# recebem o módulo de topo do projeto como `dut`.

import cocotb
from cocotb.triggers import Timer


@cocotb.test()
async def basic_test(dut):
    \"\"\"Espera 10 ns. Escreva aqui o estímulo e as conferências.\"\"\"
    await Timer(10, \"ns\")
    dut._log.info(\"basic_test rodou\")
"
        .to_owned();
    };
    // Um nome escapado do Verilog (`\a.b `) não tem como virar atributo.
    let usable = |p: &&Port| !p.name.contains(['\\', ' ']);
    let inputs: Vec<&Port> = dut
        .ports
        .iter()
        .filter(|p| p.direction == PortDirection::Input)
        .filter(usable)
        .collect();
    let outputs: Vec<&Port> = dut
        .ports
        .iter()
        .filter(|p| p.direction == PortDirection::Output)
        .filter(usable)
        .collect();
    let clock = inputs.iter().copied().find(|p| is_clock(&p.name));
    let reset = inputs
        .iter()
        .copied()
        .find_map(|p| reset_polarity(&p.name).map(|high| (p, high)));

    let name = &dut.name;
    let mut out = format!(
        "# Testbench cocotb de {name}: os testes recebem o módulo da linha abaixo\n\
         # como `dut` (a diretiva da AURORA, que a AURORA também lê).\n\
         # aurora-toplevel: {name}\n\n\
         import cocotb\n"
    );
    if clock.is_some() {
        out.push_str("from cocotb.clock import Clock\n");
    }
    out.push_str(
        "from cocotb.triggers import Timer\n\n\n@cocotb.test()\nasync def basic_test(dut):\n",
    );
    let doc = match (clock.is_some(), reset.is_some()) {
        (true, true) => "Liga o clock, aplica o reset e escreve as saídas no log.",
        (true, false) => "Liga o clock e escreve as saídas no log.",
        (false, true) => "Aplica o reset e escreve as saídas no log.",
        (false, false) => "Põe as entradas em zero e escreve as saídas no log.",
    };
    out.push_str(&format!("    \"\"\"{doc}\"\"\"\n"));
    if let Some(clk) = clock {
        out.push_str(&format!(
            "    cocotb.start_soon(Clock({}, 10, \"ns\").start())\n",
            handle(&clk.name)
        ));
    }
    for port in inputs.iter().filter(|p| Some(**p) != clock) {
        // O reset começa ativo; o resto, em zero.
        let value = match reset {
            Some((r, true)) if r.name == port.name => "1",
            _ => "0",
        };
        out.push_str(&format!("    {}.value = {value}\n", handle(&port.name)));
    }
    match reset {
        Some((r, high)) => {
            let release = if high { "0" } else { "1" };
            out.push_str(&format!(
                "    await Timer(20, \"ns\")\n    {}.value = {release}\n    await Timer(100, \"ns\")\n",
                handle(&r.name)
            ));
        }
        None if clock.is_some() => out.push_str("    await Timer(100, \"ns\")\n"),
        None => out.push_str("    await Timer(10, \"ns\")\n"),
    }
    for port in &outputs {
        out.push_str(&format!(
            "    dut._log.info(\"{} = %s\", {}.value)\n",
            port.name,
            handle(&port.name)
        ));
    }
    out
}

/// O sinal `name` do DUT em Python: `dut.<name>`, ou `getattr(dut, "<name>")`
/// quando o nome é palavra reservada do Python (`in`, `from`).
fn handle(name: &str) -> String {
    if is_python_identifier(name) {
        format!("dut.{name}")
    } else {
        format!("getattr(dut, \"{name}\")")
    }
}

/// As palavras reservadas do Python 3, que não servem de nome de módulo nem
/// de atributo.
const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

/// Um identificador do Python em ASCII, fora as palavras reservadas.
pub(crate) fn is_python_identifier(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !PYTHON_KEYWORDS.contains(&name)
}

/// O módulo de teste: o nome do `.py`, que o cocotb importa.
///
/// # Erros
///
/// [`AuroraError::InvalidName`] se o nome não é identificador do Python.
pub(crate) fn test_module(testbench: &Utf8Path) -> Result<String> {
    let stem = testbench.file_stem().unwrap_or_default();
    if is_python_identifier(stem) {
        Ok(stem.to_owned())
    } else {
        Err(AuroraError::InvalidName {
            name: testbench.file_name().unwrap_or_default().to_owned(),
            reason: MODULE_NAME_REASON.into(),
        })
    }
}

/// A onda de um testbench cocotb no `simulator`: `<raiz>/<módulo de
/// teste>.fst` no Icarus, `.vcd` no Verilator.
pub(crate) fn wave_path(
    root: &Utf8Path,
    testbench: &Utf8Path,
    simulator: Simulator,
) -> Utf8PathBuf {
    root.join(format!(
        "{}.{}",
        testbench.file_stem().unwrap_or("cocotb"),
        crate::simulate::wave_extension(simulator)
    ))
}

/// O DUT de `testbench`: o da diretiva ou, sem ela, o topo do projeto, com
/// um aviso de `tool`, como a AURORA (`decideCocotbDut`).
///
/// # Erros
///
/// [`AuroraError::NoCocotbToplevel`] sem diretiva e sem topo; os de leitura.
pub(crate) fn dut(
    project: &Project,
    testbench: &Utf8Path,
    tool: Tool,
) -> Result<(String, Vec<Diagnostic>)> {
    let source = std::fs::read_to_string(testbench)
        .map_err(AuroraError::io("Reading testbench", testbench))?;
    if let Some(module) = toplevel_directive(&source) {
        return Ok((module, Vec::new()));
    }
    let Some(top) = project.top_module()? else {
        return Err(AuroraError::NoCocotbToplevel(testbench.to_owned()));
    };
    let note = Diagnostic {
        tool,
        severity: Severity::Warning,
        message: format!(
            "{} has no `# aurora-toplevel: <module>` line: the tests drive the project top module, {top}",
            testbench.file_name().unwrap_or_default()
        ),
        file: Some(testbench.to_owned()),
        line: None,
        column: None,
        raw: String::new(),
    };
    Ok((top, vec![note]))
}

/// O que a sonda diz do cocotb do bundle. Uma biblioteca que o cocotb não
/// traz vem vazia.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Probe {
    version: String,
    /// cocotb 2 (`cocotb_tools`): `GPI_USERS` e `COCOTB_*`; o 1 usa
    /// `LIBPYTHON_LOC`, `TOPLEVEL` e `MODULE`.
    modern: bool,
    python: String,
    libpython: String,
    entry_point: String,
    /// A VPI para o Icarus, que o `vvp` carrega (`-m`).
    vpi: String,
    /// A VPI para o Verilator, que o modelo liga.
    verilator_vpi: String,
    /// O `main` do modelo do Verilator (`share/lib/verilator/verilator.cpp`).
    verilator_main: String,
    libs: String,
    sys_path: Vec<String>,
}

impl Probe {
    /// O que `simulator` usa do cocotb: a VPI e, no Verilator, o `main`.
    fn files(&self, simulator: Simulator) -> Vec<&str> {
        match simulator {
            Simulator::Icarus => vec![&self.vpi],
            // O GHDL não chega aqui: o `simulate_project` recusa o cocotb
            // nele antes.
            Simulator::Verilator | Simulator::Ghdl => {
                vec![&self.verilator_vpi, &self.verilator_main]
            }
        }
    }

    /// As bibliotecas que a sonda achou continuam no lugar: um bundle
    /// reinstalado em outra pasta invalida a sonda guardada.
    fn still_valid(&self) -> bool {
        [&self.vpi, &self.verilator_vpi, &self.verilator_main]
            .into_iter()
            .filter(|path| !path.is_empty())
            .all(|path| Utf8Path::new(path).is_file())
    }
}

/// A sonda guardada, com a chave que diz se ela ainda vale.
#[derive(Serialize, Deserialize)]
struct CachedProbe {
    key: String,
    probe: Probe,
}

/// Uma simulação cocotb pronta para os passos de
/// [`simulate_project`](crate::simulate_project).
#[derive(Debug, Clone)]
pub(crate) struct CocotbRun {
    /// O `.py`.
    pub testbench: Utf8PathBuf,
    /// A pasta do testbench em `.aurora8/Temp/cocotb/`: o `.vvp` ou o
    /// `obj_dir` do Verilator, o `cmds.f`, o módulo de dump e o
    /// `results.xml`.
    pub build: Utf8PathBuf,
    /// O módulo que grava a onda no Icarus; `None` no Verilator, em que
    /// quem grava é o modelo, e na simulação rápida.
    pub dump_module: Option<Utf8PathBuf>,
    /// O arquivo de comandos do `iverilog` (`-f`), com o timescale.
    pub commands: Utf8PathBuf,
    /// A onda; `None` na simulação rápida.
    pub wave: Option<Utf8PathBuf>,
    /// O `results.xml` que o cocotb grava.
    pub results: Utf8PathBuf,
    /// A VPI do cocotb para o simulador: o `-m` do `vvp`, ou a biblioteca
    /// que o modelo do Verilator liga.
    pub vpi: String,
    /// Só no Verilator: o `main` do modelo (`verilator.cpp`).
    pub main: Option<Utf8PathBuf>,
    /// As variáveis que o simulador precisa para o cocotb.
    pub env: Vec<(String, String)>,
    /// As bibliotecas do cocotb, que vão para o `PATH` (no Windows, as DLLs
    /// que a VPI carrega).
    pub libs: Utf8PathBuf,
}

/// Prepara a simulação de `testbench` com `dut` no topo, no `simulator`:
/// roda a sonda (ou usa a guardada) e grava o `cmds.f` e, no Icarus com
/// onda, o módulo de dump em `.aurora8/Temp/cocotb/<módulo de teste>/`. `fast`
/// é a simulação rápida, sem onda. `None` se o cancelamento foi pedido
/// durante a sonda.
///
/// # Erros
///
/// - [`AuroraError::InvalidName`]: o nome do `.py` não é identificador;
/// - [`AuroraError::ComponentMissing`]: o componente `cocotb` não está
///   instalado;
/// - [`AuroraError::CocotbUnavailable`]: o Python do bundle não carrega o
///   cocotb, ou o cocotb não traz a biblioteca do simulador;
/// - os de I/O.
pub(crate) fn prepare(
    toolchain: &Toolchain,
    project: &Project,
    testbench: &Utf8Path,
    dut: &str,
    simulator: Simulator,
    fast: bool,
    control: &Control,
) -> Result<Option<CocotbRun>> {
    let module = test_module(testbench)?;
    let dir = project.temp_dir().join("cocotb");
    let build = dir.join(&module);
    std::fs::create_dir_all(&build).map_err(AuroraError::io("Creating directory", &build))?;
    let pycache = crate::paths::user_cache_dir()?.join("pycache");
    let Some((probe, command)) = probe(toolchain, &dir, &pycache, control)? else {
        return Ok(None);
    };
    if let Some(missing) = probe
        .files(simulator)
        .into_iter()
        .find(|f| f.is_empty() || !Utf8Path::new(f).is_file())
    {
        let (name, what) = match simulator {
            Simulator::Icarus => ("Icarus", "VPI library"),
            Simulator::Verilator | Simulator::Ghdl => ("Verilator", "VPI library and main"),
        };
        return Err(AuroraError::CocotbUnavailable {
            python: command,
            reason: if missing.is_empty() {
                format!("cocotb {} has no {what} for {name}", probe.version)
            } else {
                format!(
                    "cocotb {} has no {what} for {name} ({missing})",
                    probe.version
                )
            },
        });
    }

    let wave = (!fast).then(|| wave_path(project.root(), testbench, simulator));
    // No Verilator, quem grava a onda é o `main` do cocotb, pelo `--trace`.
    let dump_module = match &wave {
        Some(wave) if simulator == Simulator::Icarus => {
            let path = build.join(format!("{DUMP_MODULE}.v"));
            let dump = format!(
                "// Gerado pela AURORA: grava a onda do testbench cocotb {file}.\n\
                 module {DUMP_MODULE};\n\
                 \x20   initial begin\n\
                 \x20       $dumpfile(\"{wave}\");\n\
                 \x20       $dumpvars(0, {dut});\n\
                 \x20   end\n\
                 endmodule\n",
                file = testbench.file_name().unwrap_or_default(),
                wave = wave.file_name().unwrap_or_default(),
            );
            std::fs::write(&path, dump).map_err(AuroraError::io("Writing", &path))?;
            Some(path)
        }
        _ => None,
    };
    let commands = build.join("cmds.f");
    std::fs::write(&commands, format!("+timescale+{TIMESCALE}\n"))
        .map_err(AuroraError::io("Writing", &commands))?;
    let results = build.join("results.xml");
    // Um `results.xml` de antes não pode passar por resultado desta.
    let _ = std::fs::remove_file(&results);

    // No Windows, o Python dentro do modelo do Verilator só acha as DLLs das
    // extensões pela pasta dele (ver o começo do módulo).
    let site = (simulator == Simulator::Verilator && cfg!(windows))
        .then(|| dll_directory_site(&dir, &probe.python))
        .transpose()?;
    let tb_dir = testbench.parent().unwrap_or(project.root());
    let python_path = std::env::join_paths(
        site.iter()
            .map(|s| s.as_std_path().to_path_buf())
            .chain(
                [tb_dir.as_std_path(), project.root().as_std_path()]
                    .into_iter()
                    .map(std::path::Path::to_path_buf),
            )
            .chain(probe.sys_path.iter().map(std::path::PathBuf::from)),
    )
    .map(|p| p.to_string_lossy().into_owned())
    .unwrap_or_default();
    let mut env: Vec<(String, String)> = vec![
        ("PYGPI_PYTHON_BIN".into(), probe.python.clone()),
        ("PYTHONPATH".into(), python_path),
        ("PYTHONPYCACHEPREFIX".into(), pycache.to_string()),
        // O log do cocotb e os `print` do teste em UTF-8: com a página de
        // código do Windows, um acento derrubava o logging do Python.
        ("PYTHONUTF8".into(), "1".into()),
        ("PYTHONIOENCODING".into(), "utf-8".into()),
        ("TOPLEVEL_LANG".into(), "verilog".into()),
        ("COCOTB_RESULTS_FILE".into(), results.to_string()),
    ];
    if probe.modern {
        env.push((
            "GPI_USERS".into(),
            format!("{};{}", probe.libpython, probe.entry_point),
        ));
        env.push(("COCOTB_TOPLEVEL".into(), dut.to_owned()));
        env.push(("COCOTB_TEST_MODULES".into(), module.clone()));
    } else {
        env.push(("LIBPYTHON_LOC".into(), probe.libpython.clone()));
        env.push(("TOPLEVEL".into(), dut.to_owned()));
        env.push(("MODULE".into(), module.clone()));
    }
    let (vpi, main) = match simulator {
        Simulator::Icarus => (probe.vpi, None),
        Simulator::Verilator | Simulator::Ghdl => (
            probe.verilator_vpi,
            Some(crate::paths::native_separators(&probe.verilator_main)),
        ),
    };
    Ok(Some(CocotbRun {
        testbench: testbench.to_owned(),
        build,
        dump_module,
        commands,
        wave,
        results,
        vpi: vpi.replace('\\', "/"),
        main,
        env,
        libs: Utf8PathBuf::from(probe.libs),
    }))
}

/// Grava em `dir/site/` o `sitecustomize.py` que põe a pasta do Python do
/// bundle (a de `python`) entre as que o Windows procura ao carregar as DLLs
/// das extensões, e devolve a pasta, para o `PYTHONPATH`.
fn dll_directory_site(dir: &Utf8Path, python: &str) -> Result<Utf8PathBuf> {
    let site = dir.join("site");
    std::fs::create_dir_all(&site).map_err(AuroraError::io("Creating directory", &site))?;
    let bin = Utf8Path::new(python)
        .parent()
        .map(|p| p.as_str().replace('\\', "/"))
        .unwrap_or_default();
    let script = site.join("sitecustomize.py");
    std::fs::write(
        &script,
        format!(
            "# Gerado pela AURORA: o Python dentro do modelo do Verilator procura as\n\
             # DLLs das extensões na pasta dele, e não no PATH.\n\
             import os\n\
             os.add_dll_directory(\"{}\")\n",
            bin.replace('"', "\\\"")
        ),
    )
    .map_err(AuroraError::io("Writing", &script))?;
    Ok(site)
}

/// A sonda do cocotb, guardada em `dir/probe.json` enquanto a chave (o
/// bundle, o Python e a versão da AURORA) não muda e as bibliotecas continuam
/// no lugar. Vem com o comando que a rodou, para os erros. `None` se o
/// cancelamento foi pedido.
fn probe(
    toolchain: &Toolchain,
    dir: &Utf8Path,
    pycache: &Utf8Path,
    control: &Control,
) -> Result<Option<(Probe, String)>> {
    let script = dir.join("aurora_cocotb_probe.py");
    let invocation = toolchain
        .cocotb_python(&script, dir)?
        .env("PYTHONPYCACHEPREFIX", pycache.as_str())
        .env("PYTHONUTF8", "1");
    let key = format!(
        "{}|{}|{}",
        env!("CARGO_PKG_VERSION"),
        toolchain.manifest().bundle,
        invocation.display_command()
    );
    let python = invocation.display_command();
    let cache = dir.join("probe.json");
    if let Some(cached) = std::fs::read_to_string(&cache)
        .ok()
        .and_then(|text| serde_json::from_str::<CachedProbe>(&text).ok())
        .filter(|c| c.key == key && c.probe.still_valid())
    {
        return Ok(Some((cached.probe, python)));
    }
    std::fs::write(&script, PROBE_SOURCE).map_err(AuroraError::io("Writing", &script))?;
    let output = crate::process::run(
        &invocation,
        &Watch {
            cancel: Some(control.cancel_token()),
            timeout: Some(PROBE_TIMEOUT),
            on_line: None,
        },
    )?;
    let unavailable = |reason: String| AuroraError::CocotbUnavailable {
        python: python.clone(),
        reason,
    };
    match output.termination {
        Termination::Exited(0) => {}
        Termination::Cancelled => return Ok(None),
        Termination::TimedOut => {
            return Err(unavailable(format!(
                "the cocotb probe did not finish in {} s",
                PROBE_TIMEOUT.as_secs()
            )));
        }
        _ => return Err(unavailable(tail(&output.stderr, &output.stdout))),
    }
    let probe: Probe = output
        .stdout
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .and_then(|line| serde_json::from_str(line.trim()).ok())
        .ok_or_else(|| unavailable(tail(&output.stderr, &output.stdout)))?;
    let cached = CachedProbe {
        key,
        probe: probe.clone(),
    };
    if let Ok(text) = serde_json::to_string_pretty(&cached) {
        // Sem o cache, a próxima simulação só roda a sonda de novo.
        let _ = std::fs::write(&cache, text);
    }
    Ok(Some((probe, python)))
}

/// As últimas linhas do que a sonda escreveu, para o erro.
fn tail(stderr: &str, stdout: &str) -> String {
    let text = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let start = lines.len().saturating_sub(6);
    let tail = lines[start..].join("\n");
    if tail.is_empty() {
        "the cocotb probe wrote nothing".into()
    } else {
        tail
    }
}

/// Lê o `results.xml` depois da simulação: o relatório dos testes e um
/// diagnóstico de erro de `tool` (o simulador) por teste que falhou, mais um
/// resumo. `completed` diz se a simulação terminou com 0: sem isso, faltar o
/// `results.xml` já está explicado pela falha do passo.
pub(crate) fn finish(
    run: &CocotbRun,
    completed: bool,
    tool: Tool,
) -> (Option<TestReport>, Vec<Diagnostic>) {
    let diagnostic = |severity, message: String, line: Option<u32>| Diagnostic {
        tool,
        severity,
        message,
        file: Some(run.testbench.clone()),
        line,
        column: None,
        raw: String::new(),
    };
    let text = match std::fs::read_to_string(&run.results) {
        Ok(text) => text,
        Err(_) if !completed => return (None, Vec::new()),
        Err(_) => {
            return (
                None,
                vec![diagnostic(
                    Severity::Error,
                    format!(
                        "cocotb did not write its results ({}): no test ran",
                        run.results
                    ),
                    None,
                )],
            );
        }
    };
    let cases = match parse_results(&text, &run.testbench) {
        Ok(cases) => cases,
        Err(reason) => {
            return (
                None,
                vec![diagnostic(
                    Severity::Error,
                    format!(
                        "Could not read the cocotb results {}: {reason}",
                        run.results
                    ),
                    None,
                )],
            );
        }
    };
    let count = |status| {
        u32::try_from(cases.iter().filter(|c| c.status == status).count()).unwrap_or(u32::MAX)
    };
    let report = TestReport {
        results: run.results.clone(),
        passed: count(TestStatus::Passed),
        failed: count(TestStatus::Failed),
        skipped: count(TestStatus::Skipped),
        cases,
    };
    let mut found: Vec<Diagnostic> = report
        .cases
        .iter()
        .filter(|c| c.status == TestStatus::Failed)
        .map(|c| Diagnostic {
            tool,
            severity: Severity::Error,
            // O cocotb 2 explica o `assert` em várias linhas (os valores de
            // cada lado): a primeira vai na mensagem, o resto no `raw`.
            message: match c.message.as_deref().and_then(|m| m.lines().next()) {
                Some(first) => format!("Test {} failed: {first}", c.name),
                None => format!("Test {} failed", c.name),
            },
            file: c.file.clone(),
            line: c.line,
            column: None,
            raw: c.message.clone().unwrap_or_default(),
        })
        .collect();
    let total = report.passed + report.failed + report.skipped;
    found.push(if total == 0 {
        diagnostic(
            Severity::Error,
            format!(
                "No cocotb test ran: {} has no @cocotb.test()",
                run.testbench.file_name().unwrap_or_default()
            ),
            None,
        )
    } else {
        let skipped = if report.skipped > 0 {
            format!(", {} skipped", report.skipped)
        } else {
            String::new()
        };
        diagnostic(
            Severity::Info,
            format!(
                "cocotb: {} of {total} tests passed, {} failed{skipped}",
                report.passed, report.failed
            ),
            None,
        )
    });
    (Some(report), found)
}

/// Os testes do `results.xml` (JUnit) do cocotb 2 (`file` e `line` em
/// `<property>`) e do 1 (atributos `file` e `lineno`).
fn parse_results(text: &str, testbench: &Utf8Path) -> std::result::Result<Vec<TestCase>, String> {
    let document = roxmltree::Document::parse(text).map_err(|e| e.to_string())?;
    let mut cases = Vec::new();
    for case in document
        .descendants()
        .filter(|n| n.has_tag_name("testcase"))
    {
        let class = case.attribute("classname").unwrap_or_default();
        let name = case.attribute("name").unwrap_or_default();
        let property = |key: &str| {
            case.descendants()
                .filter(|n| n.has_tag_name("property"))
                .find(|n| n.attribute("name") == Some(key))
                .and_then(|n| n.attribute("value"))
        };
        let file = property("file")
            .or_else(|| case.attribute("file"))
            .map(crate::paths::native_separators);
        let defined = property("line")
            .or_else(|| case.attribute("lineno"))
            .and_then(|l| l.trim().parse().ok());
        let failure = case
            .children()
            .find(|n| n.has_tag_name("failure") || n.has_tag_name("error"));
        let skipped = case.children().find(|n| n.has_tag_name("skipped"));
        let (status, message, line) = if let Some(failure) = failure {
            let traceback = failure.text().unwrap_or_default();
            let message = failure
                .attribute("message")
                .map(str::trim)
                .filter(|m| !m.is_empty())
                .map(str::to_owned)
                .or_else(|| {
                    traceback
                        .lines()
                        .rev()
                        .map(str::trim)
                        .find(|l| !l.is_empty())
                        .map(str::to_owned)
                });
            // O nome pelos dois separadores: o results.xml traz o caminho
            // como o Python o viu, que pode ser de outro sistema.
            let file_name = file
                .as_ref()
                .and_then(|f| f.as_str().rsplit(['/', '\\']).next())
                .or(testbench.file_name())
                .unwrap_or_default();
            (
                TestStatus::Failed,
                message,
                traceback_line(traceback, file_name).or(defined),
            )
        } else if let Some(skipped) = skipped {
            let message = skipped
                .attribute("message")
                .map(str::trim)
                .filter(|m| !m.is_empty())
                .map(str::to_owned);
            (TestStatus::Skipped, message, defined)
        } else {
            (TestStatus::Passed, None, defined)
        };
        cases.push(TestCase {
            name: if class.is_empty() {
                name.to_owned()
            } else {
                format!("{class}.{name}")
            },
            status,
            message,
            file: file.or_else(|| Some(testbench.to_owned())),
            line,
        });
    }
    Ok(cases)
}

/// A linha do último quadro do traceback em `file_name` (`File "...", line
/// 8, in teste`): onde o teste falhou, e não onde ele começa.
fn traceback_line(traceback: &str, file_name: &str) -> Option<u32> {
    traceback.lines().rev().find_map(|line| {
        let rest = line.trim_start().strip_prefix("File \"")?;
        let (path, rest) = rest.split_once("\", line ")?;
        let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
        let same = if cfg!(windows) {
            name.eq_ignore_ascii_case(file_name)
        } else {
            name == file_name
        };
        if !same {
            return None;
        }
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_directive_is_read_like_the_aurora_reads_it() {
        assert_eq!(
            toplevel_directive("import cocotb\n  #AURORA-TOPLEVEL = alu_32 resto\n"),
            Some("alu_32".into())
        );
        assert_eq!(
            toplevel_directive("\t# aurora-toplevel:\tcpu\r\n"),
            Some("cpu".into())
        );
        // Só numa linha de comentário, e com um identificador.
        assert_eq!(toplevel_directive("x = 1 # aurora-toplevel: alu"), None);
        assert_eq!(toplevel_directive("# aurora-toplevel: 9alu"), None);
        assert_eq!(toplevel_directive("# aurora-toplevel alu"), None);
        assert_eq!(toplevel_directive("# toplevel: alu\n"), None);
        // A primeira vale.
        assert_eq!(
            toplevel_directive("# aurora-toplevel: a\n# aurora-toplevel: b\n"),
            Some("a".into())
        );
        assert_eq!(toplevel_directive("# aurora-toplevél: a"), None);
    }

    #[test]
    fn the_test_module_is_the_file_name() {
        let ok = Utf8Path::new("/p/rtl/test_alu.py");
        assert_eq!(test_module(ok).unwrap(), "test_alu");
        for bad in [
            "/p/test-alu.py",
            "/p/1test.py",
            "/p/import.py",
            "/p/teste_ação.py",
        ] {
            let err = test_module(Utf8Path::new(bad)).unwrap_err();
            assert_eq!(err.code(), "invalid_name", "{bad}");
        }
        assert_eq!(
            wave_path(Utf8Path::new("/p"), ok, Simulator::Icarus),
            Utf8PathBuf::from("/p/test_alu.fst")
        );
        assert_eq!(
            wave_path(Utf8Path::new("/p"), ok, Simulator::Verilator),
            Utf8PathBuf::from("/p/test_alu.vcd")
        );
        assert!(is_testbench(Utf8Path::new("a/t.PY")));
        assert!(!is_testbench(Utf8Path::new("a/t.v")));
    }

    fn port(name: &str, direction: PortDirection, width: u32) -> Port {
        Port {
            name: name.into(),
            direction,
            width,
            signed: false,
        }
    }

    #[test]
    fn the_template_drives_the_ports_it_knows() {
        let dut = ModuleInterface {
            name: "contador".into(),
            file: "contador.v".into(),
            ports: vec![
                port("clk", PortDirection::Input, 1),
                port("rst_n", PortDirection::Input, 1),
                port("in", PortDirection::Input, 4),
                port("q", PortDirection::Output, 8),
            ],
        };
        let py = testbench_template(Some(&dut));
        assert_eq!(toplevel_directive(&py), Some("contador".into()));
        for line in [
            "from cocotb.clock import Clock",
            "cocotb.start_soon(Clock(dut.clk, 10, \"ns\").start())",
            "dut.rst_n.value = 0",
            "getattr(dut, \"in\").value = 0",
            "await Timer(20, \"ns\")",
            "dut.rst_n.value = 1",
            "dut._log.info(\"q = %s\", dut.q.value)",
        ] {
            assert!(py.contains(line), "falta {line:?} em:\n{py}");
        }
        assert!(!py.contains("dut.clk.value"), "{py}");
        // Sem DUT, sem diretiva: vale o topo do projeto.
        let plain = testbench_template(None);
        assert_eq!(toplevel_directive(&plain), None);
        assert!(plain.contains("@cocotb.test()"));
    }

    const RESULTS_2: &str = r#"<?xml version='1.0' encoding='utf-8'?>
<testsuites name="cocotb tests"><testsuite name="test_adder" errors="0" failures="1" skipped="1" tests="3">
<testcase classname="test_adder" name="soma" time="0.000"><properties><property name="file" value="C:\p\test_adder.py" /><property name="line" value="4" /></properties></testcase>
<testcase classname="test_adder" name="falha" time="0.000"><failure message="y = 2" type="AssertionError">Traceback (most recent call last):
  File "C:\p\test_adder.py", line 15, in falha
    assert dut.y.value == 3, f"y = {dut.y.value}"
AssertionError: y = 2
</failure><properties><property name="file" value="C:\p\test_adder.py" /><property name="line" value="11" /></properties></testcase>
<testcase classname="test_adder" name="pulado" time="0.000"><skipped /><properties><property name="line" value="20" /></properties></testcase>
</testsuite></testsuites>"#;

    const RESULTS_1: &str = r#"<testsuites name="results"><testsuite name="all" package="all">
<testcase name="soma" classname="test_adder" file="/p/test_adder.py" lineno="4" time="0.1" />
<testcase name="falha" classname="test_adder" file="/p/test_adder.py" lineno="11" time="0.1"><failure /></testcase>
</testsuite></testsuites>"#;

    #[test]
    fn results_of_both_cocotb_versions() {
        let tb = Utf8Path::new("/p/test_adder.py");
        let cases = parse_results(RESULTS_2, tb).unwrap();
        let summary: Vec<(&str, TestStatus, Option<u32>)> = cases
            .iter()
            .map(|c| (c.name.as_str(), c.status, c.line))
            .collect();
        assert_eq!(
            summary,
            [
                ("test_adder.soma", TestStatus::Passed, Some(4)),
                ("test_adder.falha", TestStatus::Failed, Some(15)),
                ("test_adder.pulado", TestStatus::Skipped, Some(20)),
            ]
        );
        assert_eq!(cases[1].message.as_deref(), Some("y = 2"));
        assert_eq!(cases[2].file.as_deref(), Some(tb));

        let old = parse_results(RESULTS_1, tb).unwrap();
        assert_eq!(old[0].status, TestStatus::Passed);
        assert_eq!(old[1].status, TestStatus::Failed);
        assert_eq!(old[1].line, Some(11));
        assert_eq!(old[1].message, None);
        assert!(parse_results("<testsuites>", tb).is_err());
    }

    #[test]
    fn the_failure_line_is_the_deepest_in_the_testbench() {
        let traceback = "Traceback (most recent call last):\n  File \"/p/test_x.py\", line 8, in t\n    await helper(dut)\n  File \"/p/test_x.py\", line 3, in helper\n    assert False\n  File \"/lib/cocotb/x.py\", line 99, in y\n";
        assert_eq!(traceback_line(traceback, "test_x.py"), Some(3));
        assert_eq!(traceback_line(traceback, "outro.py"), None);
    }
}
