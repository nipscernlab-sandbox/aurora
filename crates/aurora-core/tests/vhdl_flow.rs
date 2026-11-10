//! O fluxo VHDL: registrar arquivos pelos modelos, escolher topo e testbench
//! por entidade, simular no GHDL, conferir, ver a hierarquia, e o projeto
//! com Verilog e VHDL, em que cada simulação fica numa linguagem.
//!
//! Os testes da primeira metade não executam ferramenta nenhuma e rodam
//! sempre. Os da segunda executam o GHDL (e o Icarus, no projeto misto) do
//! bundle e precisam de `AURORA_TEST_BUNDLE` (ver `common/mod.rs`).

mod common;

use aurora_core::{
    CheckOptions, Control, FileRole, HierarchyOptions, Project, Severity, SimulationOptions,
    Simulator, SourceLanguage, Status, Step, Tool, WaveformFormat, check, hierarchy,
    simulate_project, waveform_path,
};
use camino::Utf8Path;

fn new_project(dir: &tempfile::TempDir) -> Project {
    let root = Utf8Path::from_path(dir.path()).unwrap();
    Project::create(root, "demo").unwrap()
}

// ------------------------------------------------------------ sem ferramenta

#[test]
fn new_vhdl_files_come_from_the_templates_and_pick_top_and_testbench() {
    let dir = tempfile::tempdir().unwrap();
    let mut project = new_project(&dir);
    let added = project
        .add_hdl_files(
            None,
            &["rtl/Contador.vhd".into(), "rtl/contador_tb.vhd".into()],
            false,
        )
        .unwrap();
    assert_eq!(added[0].role, FileRole::Synthesizable);
    assert_eq!(added[0].language, Some(SourceLanguage::Vhdl));
    assert!(
        added[0].created && added[0].selected,
        "a primeira entidade vira o topo"
    );
    assert_eq!(added[1].role, FileRole::Testbench);
    assert!(added[1].selected, "o primeiro testbench vira o escolhido");

    let tb = std::fs::read_to_string(&added[1].path).unwrap();
    assert!(
        tb.contains("dut : entity work.Contador"),
        "o testbench instancia a entidade do nome:\n{tb}"
    );
    assert!(tb.contains("std.env.finish;"));

    assert_eq!(project.top_module().unwrap().as_deref(), Some("Contador"));
    assert_eq!(
        project.testbench_module().unwrap().as_deref(),
        Some("contador_tb")
    );
    // O VHDL não diferencia maiúsculas: o topo pelo nome também não.
    project.set_top("CONTADOR").unwrap();
    assert_eq!(project.top_module().unwrap().as_deref(), Some("Contador"));
    let files = project.files(FileRole::Synthesizable);
    assert_eq!(files[0].language, Some(SourceLanguage::Vhdl));
}

#[test]
fn an_existing_vhdl_file_is_classified_by_its_content() {
    let dir = tempfile::tempdir().unwrap();
    let mut project = new_project(&dir);
    let root = project.root().to_owned();
    std::fs::write(
        root.join("bancada.vhd"),
        "entity bancada is end;\narchitecture a of bancada is begin\n  process begin wait for 5 ns; std.env.stop; end process;\nend;\n",
    )
    .unwrap();
    let added = project.add_hdl(None, "bancada.vhd", false).unwrap();
    assert_eq!(added.role, FileRole::Testbench);
    assert!(!added.created);
}

#[test]
fn a_new_vhdl_file_needs_a_vhdl_identifier() {
    let dir = tempfile::tempdir().unwrap();
    let project = new_project(&dir);
    for bad in ["rtl/signal.vhd", "rtl/a__b.vhd", "rtl/_x.vhdl"] {
        assert!(
            project.check_add_hdl(bad).is_err(),
            "{bad} não serve de entidade"
        );
    }
    assert!(project.check_add_hdl("rtl/ok_1.VHD").is_ok());
    assert!(project.check_add_hdl("rtl/notas.txt").is_err());
}

#[test]
fn unregistered_vhdl_files_are_listed() {
    let dir = tempfile::tempdir().unwrap();
    let project = new_project(&dir);
    let root = project.root().to_owned();
    std::fs::write(root.join("solto.vhd"), "entity solto is end;").unwrap();
    std::fs::write(root.join("solto.v"), "module solto2; endmodule").unwrap();
    let found = project.unregistered_hdl();
    assert_eq!(found.len(), 2, "{found:?}");
}

// ------------------------------------------------------------ com o GHDL

const CONTADOR: &str = "\
library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity contador is
  port (clk, rst : in std_logic; q : out std_logic_vector(3 downto 0));
end entity contador;

architecture rtl of contador is
  type estado is (PARADO, CONTANDO);
  signal e : estado := PARADO;
  signal n : unsigned(3 downto 0) := (others => '0');
begin
  process (clk)
  begin
    if rising_edge(clk) then
      if rst = '1' then n <= (others => '0'); e <= PARADO;
      else n <= n + 1; e <= CONTANDO; end if;
    end if;
  end process;
  q <= std_logic_vector(n);
end architecture rtl;
";

fn testbench(check: &str) -> String {
    format!(
        "\
library ieee;
use ieee.std_logic_1164.all;

entity contador_tb is
end entity contador_tb;

architecture sim of contador_tb is
  signal clk : std_logic := '0';
  signal rst : std_logic := '1';
  signal q   : std_logic_vector(3 downto 0);
begin
  dut : entity work.contador port map (clk => clk, rst => rst, q => q);
  clk <= not clk after 5 ns;
  process
  begin
    wait for 20 ns;
    rst <= '0';
    wait for 40 ns;
    {check}
    std.env.finish;
  end process;
end architecture sim;
"
    )
}

/// O projeto do contador em VHDL, com o testbench `tb`.
fn vhdl_project(dir: &tempfile::TempDir, tb: &str) -> Project {
    let mut project = new_project(dir);
    let root = project.root().to_owned();
    std::fs::create_dir_all(root.join("rtl")).unwrap();
    std::fs::write(root.join("rtl/contador.vhd"), CONTADOR).unwrap();
    std::fs::write(root.join("rtl/contador_tb.vhd"), tb).unwrap();
    project
        .add_hdl_files(
            None,
            &["rtl/contador.vhd".into(), "rtl/contador_tb.vhd".into()],
            false,
        )
        .unwrap();
    project
}

#[test]
fn a_vhdl_testbench_runs_on_ghdl_and_writes_a_ghw_wave() {
    let Some(toolchain) = common::toolchain_with(&[Tool::Ghdl]) else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let project = vhdl_project(&dir, &testbench("report \"q = \" & to_hstring(q);"));
    // O pedido diz Icarus, a preferência de Verilog: o VHDL vai para o GHDL.
    let options = SimulationOptions::new(Simulator::Icarus);
    let result = simulate_project(&toolchain, &project, &options, &Control::default()).unwrap();
    assert_eq!(
        result.status,
        Status::Succeeded,
        "{:#?}",
        result.diagnostics
    );
    assert_eq!(result.simulator, Simulator::Ghdl);
    let steps: Vec<_> = result.steps.iter().map(|s| (s.step, s.tool)).collect();
    assert_eq!(
        steps,
        [
            (Step::Analyze, Tool::Ghdl),
            (Step::Elaborate, Tool::Ghdl),
            (Step::Simulate, Tool::Ghdl)
        ]
    );
    let wave = result.waveform.expect("a onda");
    assert_eq!(wave.format, WaveformFormat::Ghw);
    assert!(wave.path.is_file());
    assert_eq!(wave.path, waveform_path(&project, None).unwrap());
    // O `report note` é saída do testbench, não diagnóstico.
    assert!(result.steps[2].stdout.contains("q = "));
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error)
    );

    let mut fast = SimulationOptions::new(Simulator::Verilator);
    fast.fast = true;
    let result = simulate_project(&toolchain, &project, &fast, &Control::default()).unwrap();
    assert_eq!(result.status, Status::Succeeded);
    assert!(
        result.waveform.is_none(),
        "a simulação rápida não grava onda"
    );
}

#[test]
fn a_failed_assertion_fails_the_simulation_and_keeps_the_wave() {
    let Some(toolchain) = common::toolchain_with(&[Tool::Ghdl]) else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let project = vhdl_project(
        &dir,
        &testbench("assert q = x\"0\" report \"q errado\" severity error;"),
    );
    let options = SimulationOptions::new(Simulator::Ghdl);
    let result = simulate_project(&toolchain, &project, &options, &Control::default()).unwrap();
    assert_eq!(result.status, Status::Failed);
    assert_eq!(result.failed_step, Some(Step::Simulate));
    let error = result
        .diagnostics
        .iter()
        .find(|d| d.severity == Severity::Error)
        .expect("o erro da asserção");
    assert!(error.message.starts_with("q errado"), "{error:?}");
    assert_eq!(error.line, Some(19));
    assert!(
        error
            .file
            .as_ref()
            .is_some_and(|f| f.ends_with("contador_tb.vhd"))
    );
    assert!(result.waveform.is_some(), "a onda mostra por que falhou");
}

#[test]
fn a_vhdl_error_points_to_the_file_and_line() {
    let Some(toolchain) = common::toolchain_with(&[Tool::Ghdl]) else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let project = vhdl_project(&dir, &testbench("q <= sinal_que_nao_existe;"));
    let options = SimulationOptions::new(Simulator::Icarus);
    let result = simulate_project(&toolchain, &project, &options, &Control::default()).unwrap();
    assert_eq!(result.status, Status::Failed);
    assert!(matches!(
        result.failed_step,
        Some(Step::Analyze | Step::Elaborate)
    ));
    let error = result
        .diagnostics
        .iter()
        .find(|d| d.severity == Severity::Error)
        .expect("o erro");
    assert_eq!(error.line, Some(19), "{error:?}");
    assert!(
        error.raw.contains('^'),
        "a linha do fonte e o acento vêm juntos: {}",
        error.raw
    );

    let checked = check(
        &toolchain,
        &project,
        &CheckOptions::default(),
        &Control::default(),
    )
    .unwrap();
    assert_eq!(checked.status, Status::Failed);
}

#[test]
fn check_and_hierarchy_read_a_vhdl_design() {
    let Some(toolchain) = common::toolchain_with(&[Tool::Ghdl]) else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let project = vhdl_project(&dir, &testbench(""));
    let checked = check(
        &toolchain,
        &project,
        &CheckOptions::default(),
        &Control::default(),
    )
    .unwrap();
    assert_eq!(
        checked.status,
        Status::Succeeded,
        "{:#?}",
        checked.diagnostics
    );
    assert!(checked.steps.iter().all(|s| s.tool == Tool::Ghdl));

    let tree = hierarchy(
        &toolchain,
        &project,
        &HierarchyOptions::default(),
        &Control::default(),
    )
    .unwrap();
    assert_eq!(tree.status, Status::Succeeded, "{:#?}", tree.diagnostics);
    let design = tree.design.expect("o design");
    assert_eq!(design.roots[0].module, "contador");
    assert_eq!(design.roots[0].architecture.as_deref(), Some("rtl"));
    let bench = &tree.testbenches[0].roots[0];
    assert_eq!(bench.module, "contador_tb");
    assert_eq!(bench.children[0].name, "dut");
    assert_eq!(bench.children[0].module, "contador");
    assert_eq!(bench.children[0].instance_line, Some(12));

    // A escolha de sinais de um testbench VHDL ainda não existe: a recusa diz
    // por quê, em vez de um erro do Icarus.
    let choice = aurora_core::wave_signals(&toolchain, &project, &Control::default()).unwrap_err();
    assert_eq!(choice.code(), "invalid_project");
    assert!(choice.to_string().contains("VHDL testbench"), "{choice}");
}

#[test]
fn each_simulation_of_a_mixed_project_keeps_to_its_language() {
    let Some(toolchain) = common::toolchain_with(&[Tool::Ghdl, Tool::Iverilog]) else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let mut project = vhdl_project(&dir, &testbench(""));
    let root = project.root().to_owned();
    std::fs::write(
        root.join("rtl/porta.v"),
        "module porta(input a, output y); assign y = a; endmodule\n",
    )
    .unwrap();
    std::fs::write(
        root.join("rtl/porta_tb.v"),
        "module porta_tb; reg a = 0; wire y; porta dut(.a(a), .y(y)); initial begin #5 $finish; end endmodule\n",
    )
    .unwrap();
    project
        .add_hdl_files(
            None,
            &["rtl/porta.v".into(), "rtl/porta_tb.v".into()],
            false,
        )
        .unwrap();

    project.set_testbench(root.join("rtl/porta_tb.v")).unwrap();
    let options = SimulationOptions::new(Simulator::Icarus);
    let verilog = simulate_project(&toolchain, &project, &options, &Control::default()).unwrap();
    assert_eq!(
        verilog.status,
        Status::Succeeded,
        "o Icarus não recebe o VHDL: {:#?}",
        verilog.diagnostics
    );
    assert_eq!(verilog.simulator, Simulator::Icarus);

    // Um testbench VHDL que instancia o módulo Verilog: o GHDL só avisa, a
    // AURORA reprova e diz por quê.
    std::fs::write(
        root.join("rtl/misto_tb.vhd"),
        "library ieee; use ieee.std_logic_1164.all;\nentity misto_tb is end;\narchitecture sim of misto_tb is\n  component porta port (a : in std_logic; y : out std_logic); end component;\n  signal a, y : std_logic := '0';\nbegin\n  u0 : porta port map (a => a, y => y);\n  process begin wait for 5 ns; std.env.finish; end process;\nend;\n",
    )
    .unwrap();
    project.add_hdl(None, "rtl/misto_tb.vhd", true).unwrap();
    project
        .set_testbench(root.join("rtl/misto_tb.vhd"))
        .unwrap();
    let mixed = simulate_project(&toolchain, &project, &options, &Control::default()).unwrap();
    assert_eq!(mixed.status, Status::Failed);
    assert!(
        mixed
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error
                && d.message.contains("porta is a Verilog module")),
        "{:#?}",
        mixed.diagnostics
    );

    // O GHDL pedido para um testbench Verilog é recusado.
    project.set_testbench(root.join("rtl/porta_tb.v")).unwrap();
    let ghdl = SimulationOptions::new(Simulator::Ghdl);
    let error = simulate_project(&toolchain, &project, &ghdl, &Control::default()).unwrap_err();
    assert_eq!(error.code(), "simulator_language");
}
