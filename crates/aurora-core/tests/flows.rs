//! Os fluxos compostos (`aurora_core::flows`) rodando de verdade, com o bundle
//! de `AURORA_TEST_BUNDLE`, sobre cópias dos exemplos. Sem bundle, avisam e
//! passam (ver `common/mod.rs`).

mod common;

use std::sync::Mutex;

use aurora_core::flows::{
    FlowOptions, FlowOutcome, FlowRequest, LayoutPolicy, Phase, Progress, WaveChoice, WavePolicy,
    WaveRequest, WaveSource, choose_signals, open_wave, run,
};
use aurora_core::{CancelToken, Control, Project, Simulator, Status, Toolchain};

/// Roda um fluxo guardando o que ele disse, em ordem, como etiquetas.
fn run_logged(
    toolchain: &Toolchain,
    project: &mut Project,
    request: &FlowRequest,
    options: &FlowOptions,
    control: &Control,
) -> (FlowOutcome, Vec<String>) {
    let log = Mutex::new(Vec::new());
    let progress = |p: Progress<'_>| {
        let tag = match p {
            Progress::Phase(phase) => format!("phase:{phase:?}"),
            Progress::Built(b) => format!("built:{}:{:?}", b.processor, b.status),
            Progress::Registered(files) => format!("registered:{}", files.len()),
            Progress::RegisterFailed(_) => "register_failed".to_owned(),
            Progress::NotRun { what, .. } => format!("not_run:{what:?}"),
            Progress::Checked(_) => "checked".to_owned(),
            Progress::Elaborated { not_built, .. } => format!("elaborated:{}", not_built.len()),
            Progress::Simulated { outputs, .. } => format!("simulated:{}", outputs.len()),
            Progress::Synthesized(_) => "synthesized".to_owned(),
            Progress::Drawn(_) => "drawn".to_owned(),
            Progress::FpgaBuilt(_) => "fpga_built".to_owned(),
            Progress::Programmed(_) => "programmed".to_owned(),
            Progress::Recorded(id) => format!("recorded:{id}"),
            Progress::ReportNotSaved(_) => "report_not_saved".to_owned(),
            Progress::WaveOpened(_) => "wave_opened".to_owned(),
            Progress::RtlViewer { .. } => "rtl_viewer".to_owned(),
        };
        log.lock().unwrap().push(tag);
    };
    let outcome = run(toolchain, project, request, options, &progress, control).unwrap();
    (outcome, log.into_inner().unwrap())
}

fn options(command: &str) -> FlowOptions {
    let mut options = FlowOptions::new(command);
    options.wave = WavePolicy::None;
    options
}

fn simulate(processor: Option<&str>) -> FlowRequest {
    FlowRequest::Simulate {
        processor: processor.map(str::to_owned),
        testbench: None,
        simulator: Simulator::Icarus,
        fast: false,
        timeout_s: Some(60),
        open_wave: false,
    }
}

#[test]
fn build_without_processors_succeeds_without_running_anything() {
    let Some(toolchain) = common::toolchain() else {
        return;
    };
    let (_guard, root) = common::example("contador");
    let mut project = Project::open(&root).unwrap();
    let request = FlowRequest::Build {
        processors: Vec::new(),
    };
    let (outcome, log) = run_logged(
        &toolchain,
        &mut project,
        &request,
        &options("aurora build"),
        &Control::default(),
    );
    assert!(outcome.succeeded);
    assert_eq!(outcome.status, Status::Succeeded);
    assert!(outcome.builds.is_empty());
    assert!(outcome.report.is_none(), "nada rodou, nada é gravado");
    assert!(log.is_empty(), "{log:?}");
    assert_eq!(outcome.flow, "build");
    assert_eq!(outcome.command, "aurora build");
}

#[test]
fn build_compiles_every_processor_even_after_a_failure_and_records() {
    let Some(toolchain) = common::toolchain() else {
        return;
    };
    let (_guard, root) = common::example("com_erro");
    let mut project = Project::open(&root).unwrap();
    let request = FlowRequest::Build {
        processors: Vec::new(),
    };
    let (outcome, log) = run_logged(
        &toolchain,
        &mut project,
        &request,
        &options("aurora build"),
        &Control::default(),
    );
    assert_eq!(outcome.builds.len(), 2, "{log:?}");
    assert!(!outcome.succeeded, "o exemplo tem um processador com erro");
    assert_eq!(outcome.status, Status::Failed);
    assert!(outcome.report.is_some(), "{outcome:#?}");
    // Dois builds, o registro e o relatório, nessa ordem.
    assert_eq!(log[0], "phase:Build");
    assert!(
        log[1].starts_with("built:") && log[2].starts_with("built:"),
        "{log:?}"
    );
    assert!(log[3].starts_with("registered:"), "{log:?}");
    assert!(log[4].starts_with("recorded:run-"), "{log:?}");
    assert_eq!(log.len(), 5);
}

#[test]
fn an_unknown_processor_is_an_error_before_anything_runs() {
    let Some(toolchain) = common::toolchain() else {
        return;
    };
    let (_guard, root) = common::example("soma");
    let mut project = Project::open(&root).unwrap();
    let request = FlowRequest::Build {
        processors: vec!["soma".into(), "nao_existe".into()],
    };
    let error = run(
        &toolchain,
        &mut project,
        &request,
        &options("aurora build"),
        &|_| {},
        &Control::default(),
    )
    .unwrap_err();
    assert_eq!(error.code(), "processor_not_found");
    assert!(
        !project.require_processor("soma").unwrap().is_built(),
        "nenhum build antes do erro"
    );
}

#[test]
fn simulating_a_processor_builds_it_reads_the_outputs_and_records() {
    let Some(toolchain) = common::toolchain() else {
        return;
    };
    let (_guard, root) = common::example("soma");
    let mut project = Project::open(&root).unwrap();
    let (outcome, log) = run_logged(
        &toolchain,
        &mut project,
        &simulate(Some("soma")),
        &options("aurora sim -p soma"),
        &Control::default(),
    );
    assert!(outcome.succeeded, "{outcome:#?}");
    assert_eq!(outcome.builds.len(), 1);
    assert_eq!(outcome.builds[0].processor, "soma");
    let simulation = outcome.simulation.as_ref().expect("simulou");
    assert!(simulation.succeeded());
    assert!(!outcome.outputs.is_empty(), "o soma escreve na porta 0");
    assert_eq!(outcome.outputs[0].port, 0);
    assert!(outcome.outputs[0].error.is_none());
    assert!(!outcome.outputs[0].values.is_empty());
    assert!(outcome.report.is_some());
    assert!(outcome.wave.is_none() && outcome.wave_error.is_none());
    assert_eq!(
        log,
        [
            "phase:Build",
            "built:soma:Succeeded",
            &format!("registered:{}", outcome.registered.len()),
            "phase:Simulate",
            &format!("simulated:{}", outcome.outputs.len()),
            &format!("recorded:{}", outcome.report.as_deref().unwrap()),
        ]
    );
    // Com limite, cada porta leva só os primeiros valores e diz quantos tem:
    // a simulação rápida da AURORA IDE pede 0.
    let every = outcome.outputs[0].values.clone();
    assert_eq!(outcome.outputs[0].count, every.len());
    for limit in [0, 1] {
        let mut limited = options("aurora sim -p soma");
        limited.port_values_limit = Some(limit);
        let (outcome, _) = run_logged(
            &toolchain,
            &mut project,
            &simulate(Some("soma")),
            &limited,
            &Control::default(),
        );
        assert!(outcome.succeeded, "{outcome:#?}");
        assert_eq!(outcome.outputs[0].values, every[..limit].to_vec());
        assert_eq!(outcome.outputs[0].count, every.len());
    }
    // O Verilog gerado entrou no .spf, e o processador é visto como compilado.
    let project = Project::open(&root).unwrap();
    assert!(project.require_processor("soma").unwrap().is_built());
    assert!(project.status().processors[0].built);
}

#[test]
fn a_failed_build_stops_the_simulation_but_still_records() {
    let Some(toolchain) = common::toolchain() else {
        return;
    };
    let (_guard, root) = common::example("com_erro");
    let mut project = Project::open(&root).unwrap();
    let (outcome, log) = run_logged(
        &toolchain,
        &mut project,
        &simulate(None),
        &options("aurora sim"),
        &Control::default(),
    );
    assert!(!outcome.succeeded);
    assert_eq!(outcome.status, Status::Failed);
    assert!(outcome.simulation.is_none(), "a simulação não roda");
    assert!(outcome.builds.iter().any(|b| !b.succeeded()));
    assert!(outcome.report.is_some(), "o relatório sai só com os builds");
    assert!(log.contains(&"not_run:Simulate".to_owned()), "{log:?}");
    assert!(!log.contains(&"phase:Simulate".to_owned()), "{log:?}");
}

#[test]
fn check_runs_on_what_is_on_disk_without_building() {
    let Some(toolchain) = common::toolchain() else {
        return;
    };
    let (_guard, root) = common::example("contador");
    let mut project = Project::open(&root).unwrap();
    let request = FlowRequest::Check {
        file: None,
        processor: None,
        lint: false,
    };
    let (outcome, log) = run_logged(
        &toolchain,
        &mut project,
        &request,
        &options("aurora check"),
        &Control::default(),
    );
    assert!(outcome.succeeded, "{outcome:#?}");
    assert!(outcome.builds.is_empty());
    assert!(outcome.check.is_some());
    assert!(outcome.report.is_some());
    assert_eq!(log[0], "phase:Check");
    assert_eq!(log[1], "checked");
    assert!(log[2].starts_with("recorded:"));
}

#[test]
fn synthesis_keeps_its_report_when_the_schematic_module_does_not_exist() {
    let Some(toolchain) = common::toolchain() else {
        return;
    };
    let (_guard, root) = common::example("contador");
    let mut project = Project::open(&root).unwrap();
    let request = FlowRequest::Synthesize {
        processor: None,
        schematic: true,
        module: Some("nao_existe".into()),
        open_viewer: false,
    };
    let (outcome, log) = run_logged(
        &toolchain,
        &mut project,
        &request,
        &options("aurora synth --svg --module nao_existe"),
        &Control::default(),
    );
    let synthesis = outcome.synthesis.as_ref().expect("sintetizou");
    assert!(synthesis.succeeded(), "{synthesis:#?}");
    assert!(outcome.schematic.is_none());
    let error = outcome
        .schematic_error
        .as_ref()
        .expect("o módulo não existe");
    assert_eq!(error.code, "module_not_found");
    assert!(outcome.report.is_some(), "a síntese vale e é gravada");
    assert!(
        outcome.succeeded,
        "a síntese deu certo; o esquemático não rodou"
    );
    assert!(log.contains(&"synthesized".to_owned()));
    assert!(!log.contains(&"drawn".to_owned()));
}

#[test]
fn hierarchy_lists_the_processors_left_out_for_lack_of_a_build() {
    let Some(toolchain) = common::toolchain() else {
        return;
    };
    let (_guard, root) = common::example("soma");
    let mut project = Project::open(&root).unwrap();
    let build = FlowRequest::Build {
        processors: vec!["soma".into()],
    };
    let (built, _) = run_logged(
        &toolchain,
        &mut project,
        &build,
        &options("aurora build -p soma"),
        &Control::default(),
    );
    assert!(built.succeeded, "{built:#?}");
    let request = FlowRequest::Hierarchy { processor: None };
    let (outcome, log) = run_logged(
        &toolchain,
        &mut project,
        &request,
        &options("aurora hierarchy"),
        &Control::default(),
    );
    assert!(outcome.hierarchy.is_some(), "{outcome:#?}");
    assert_eq!(outcome.not_built, ["filtro"]);
    assert!(outcome.report.is_none(), "a hierarquia não grava relatório");
    assert_eq!(log, ["phase:Hierarchy", "elaborated:1"]);
}

#[test]
fn the_wave_of_an_unsimulated_processor_does_not_exist_yet() {
    let Some(toolchain) = common::toolchain() else {
        return;
    };
    let (_guard, root) = common::example("soma");
    let mut project = Project::open(&root).unwrap();
    let build = FlowRequest::Build {
        processors: vec!["soma".into()],
    };
    run(
        &toolchain,
        &mut project,
        &build,
        &options("aurora build -p soma"),
        &|_| {},
        &Control::default(),
    )
    .unwrap();
    let request = WaveRequest {
        source: WaveSource::Project {
            processor: Some("soma".into()),
        },
        layout: LayoutPolicy::Generated,
    };
    let error = open_wave(&toolchain, Some(&project), &request).unwrap_err();
    assert_eq!(error.code(), "no_waveform");
    let info = aurora_core::ErrorInfo::from(&error);
    assert_eq!(info.data.unwrap()["processor"], "soma");
    // Um arquivo que não existe é o mesmo erro, sem processador.
    let request = WaveRequest {
        source: WaveSource::File {
            path: root.join("nao_existe.vcd"),
        },
        layout: LayoutPolicy::Generated,
    };
    let error = open_wave(&toolchain, None, &request).unwrap_err();
    assert_eq!(error.code(), "no_waveform", "{error}");
}

#[test]
fn a_cancelled_token_stops_the_flow_before_the_tools_run() {
    let Some(toolchain) = common::toolchain() else {
        return;
    };
    let (_guard, root) = common::example("soma");
    let mut project = Project::open(&root).unwrap();
    let cancel = CancelToken::new();
    cancel.cancel();
    let control = Control::new().with_cancel(cancel);
    let (outcome, _) = run_logged(
        &toolchain,
        &mut project,
        &simulate(Some("soma")),
        &options("aurora sim -p soma"),
        &control,
    );
    assert_eq!(outcome.status, Status::Cancelled, "{outcome:#?}");
    assert!(!outcome.succeeded);
    assert!(outcome.simulation.is_none());
    assert!(!project.require_processor("soma").unwrap().is_built());
}

#[test]
fn choosing_signals_checks_the_paths_against_the_elaborated_testbench() {
    let Some(toolchain) = common::toolchain() else {
        return;
    };
    let (_guard, root) = common::example("contador");
    let project = Project::open(&root).unwrap();
    let control = Control::default();
    let error = choose_signals(
        &toolchain,
        &project,
        &WaveChoice::Add(vec!["nao.existe".into()]),
        &control,
    )
    .unwrap_err();
    assert_eq!(error.code(), "signal_not_found", "{error}");
    let all = choose_signals(&toolchain, &project, &WaveChoice::All, &control).unwrap();
    assert!(all.file.is_none());
    assert!(all.selection.is_empty());
    assert!(!all.testbench.is_empty());
    let error = choose_signals(
        &toolchain,
        &project,
        &WaveChoice::Remove(vec!["nao.existe".into()]),
        &control,
    )
    .unwrap_err();
    assert_eq!(error.code(), "signal_not_chosen", "{error}");
    let error = choose_signals(
        &toolchain,
        &project,
        &WaveChoice::Set(vec!["nao.existe".into()]),
        &control,
    )
    .unwrap_err();
    assert_eq!(error.code(), "signal_not_found", "{error}");
    // A lista vazia vale como All, sem elaborar.
    let none = choose_signals(&toolchain, &project, &WaveChoice::Set(vec![]), &control).unwrap();
    assert!(none.file.is_none() && none.selection.is_empty());
    let tree = aurora_core::wave_signals(&toolchain, &project, &control).unwrap();
    let root_scope = tree.root.expect("o testbench elabora");
    let set = choose_signals(
        &toolchain,
        &project,
        &WaveChoice::Set(vec![root_scope.path.clone()]),
        &control,
    )
    .unwrap();
    assert_eq!(set.selection, [root_scope.path]);
    assert!(set.file.is_some());
}

#[test]
fn the_phase_names_match_the_studio() {
    assert_eq!(serde_json::to_value(Phase::Build).unwrap(), "build");
    assert_eq!(
        serde_json::to_value(Phase::Synthesize).unwrap(),
        "synthesize"
    );
    assert_eq!(serde_json::to_value(Phase::Fpga).unwrap(), "fpga");
    assert_eq!(serde_json::to_value(Phase::Program).unwrap(), "program");
}
