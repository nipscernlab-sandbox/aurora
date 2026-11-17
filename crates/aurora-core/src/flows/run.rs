//! A composição de cada fluxo: a regra que a CLI e o Studio seguiam cada um
//! por si, numa função por fluxo.

use std::time::{Duration, SystemTime};

use camino::Utf8PathBuf;

use super::{
    FlowOptions, FlowOutcome, FlowRequest, LayoutPolicy, Phase, PortValues, Progress, ProgressFn,
    WavePolicy, WaveRequest, WaveSource,
};
use crate::fpga::{self, FpgaConfig};
use crate::history::{self, Operation};
use crate::{
    AuroraError, BuildOptions, BuildResult, CheckOptions, Control, DesignTarget, ErrorInfo,
    HierarchyOptions, OnFailure, Processor, Project, Result, SchematicOptions, SimulationOptions,
    SimulationResult, Status, Tool, Toolchain,
};

/// O relatório gravado, ou por que não foi.
type Recorded = (Option<String>, Option<String>);

/// Guarda em `outcome` o que [`Flow::record`] devolveu.
fn store(outcome: &mut FlowOutcome, recorded: Recorded) {
    (outcome.report, outcome.report_error) = recorded;
}

/// O que todo fluxo recebe.
struct Flow<'a> {
    toolchain: &'a Toolchain,
    options: &'a FlowOptions,
    progress: ProgressFn<'a>,
    control: &'a Control,
    started: SystemTime,
}

impl Flow<'_> {
    fn say(&self, progress: Progress<'_>) {
        (self.progress)(progress);
    }

    /// Compila `targets` na ordem, avisando cada build, e registra no `.spf`
    /// o que saiu. Devolve se todos deram certo.
    fn build(
        &self,
        project: &mut Project,
        targets: &[Processor],
        on_failure: OnFailure,
        outcome: &mut FlowOutcome,
    ) -> Result<bool> {
        if !targets.is_empty() {
            self.say(Progress::Phase(Phase::Build));
        }
        outcome.builds = crate::build_processors(
            self.toolchain,
            targets.iter(),
            &BuildOptions::default(),
            on_failure,
            self.control,
            |result| self.say(Progress::Built(result)),
        )?;
        let statuses: Vec<Status> = outcome.builds.iter().map(|b| b.status).collect();
        for status in statuses {
            outcome.worsen(status);
        }
        // O `.v` e o testbench que cada processador gerou entram no `.spf`,
        // mesmo com um build que falhou. Uma falha ao gravar não desfaz o
        // build.
        match project.register_generated(&outcome.builds) {
            Ok(files) => {
                self.say(Progress::Registered(&files));
                outcome.registered = files;
            }
            Err(error) => {
                tracing::warn!("The generated files could not be added to the project: {error}");
                self.say(Progress::RegisterFailed(&error));
            }
        }
        Ok(outcome.builds.iter().all(BuildResult::succeeded))
    }

    /// Grava a operação no histórico e devolve o relatório, ou o motivo de
    /// não ter gravado. Uma falha ao gravar não muda o resultado do fluxo.
    fn record(&self, project: &Project, operation: &Operation<'_>) -> Recorded {
        match history::record(project, self.toolchain, operation) {
            Ok(record) => {
                self.say(Progress::Recorded(&record.id));
                (Some(record.id), None)
            }
            Err(error) => {
                self.say(Progress::ReportNotSaved(&error));
                (None, Some(error.to_string()))
            }
        }
    }

    fn operation<'o>(&self, outcome: &'o FlowOutcome) -> Operation<'o> {
        Operation::new(&outcome.command, self.started).with_builds(&outcome.builds)
    }
}

/// Roda um fluxo. `Err` é a AURORA não conseguir rodar (bundle, projeto, E/S,
/// um recurso experimental desligado); um fluxo que rodou e falhou é `Ok`
/// com [`FlowOutcome::status`] diferente de `Succeeded`.
pub fn run(
    toolchain: &Toolchain,
    project: &mut Project,
    request: &FlowRequest,
    options: &FlowOptions,
    progress: ProgressFn<'_>,
    control: &Control,
) -> Result<FlowOutcome> {
    if let Some(feature) = request.feature() {
        crate::experimental::require(feature)?;
    }
    let flow = Flow {
        toolchain,
        options,
        progress,
        control,
        started: options.started.unwrap_or_else(SystemTime::now),
    };
    let mut outcome = FlowOutcome::new(request.name(), &options.command);
    match request {
        FlowRequest::Build { processors } => build(&flow, project, processors, &mut outcome)?,
        FlowRequest::Check {
            file,
            processor,
            lint,
        } => check(
            &flow,
            project,
            file.as_deref(),
            processor.as_deref(),
            *lint,
            &mut outcome,
        )?,
        FlowRequest::Hierarchy { processor } => {
            hierarchy(&flow, project, processor.as_deref(), &mut outcome)?
        }
        FlowRequest::Simulate {
            processor,
            testbench,
            simulator,
            fast,
            timeout_s,
            open_wave,
        } => {
            let mut sim = SimulationOptions::new(*simulator);
            sim.timeout = timeout_s.map(Duration::from_secs);
            sim.fast = *fast;
            simulate(
                &flow,
                project,
                processor.as_deref(),
                testbench.as_deref(),
                &sim,
                *open_wave,
                &mut outcome,
            )?
        }
        FlowRequest::Synthesize {
            processor,
            schematic,
            module,
            open_viewer,
        } => synthesize(
            &flow,
            project,
            processor.as_deref(),
            *schematic,
            module.as_deref(),
            *open_viewer,
            &mut outcome,
        )?,
        FlowRequest::Schematic {
            netlist,
            module,
            bus_widths,
        } => {
            flow.say(Progress::Phase(Phase::Schematic));
            let options = SchematicOptions {
                bus_widths: *bus_widths,
                ..SchematicOptions::default()
            };
            let result = crate::render_schematic(toolchain, netlist, module, &options, control)?;
            flow.say(Progress::Drawn(&result));
            outcome.worsen(result.status);
            outcome.schematic = Some(result);
        }
        FlowRequest::FpgaBuild => fpga_build(&flow, project, &mut outcome)?,
        FlowRequest::FpgaProgram { cable } => {
            flow.say(Progress::Phase(Phase::Program));
            let result = fpga::program(toolchain, project, cable.as_deref(), control)?;
            flow.say(Progress::Programmed(&result));
            outcome.worsen(result.status);
            outcome.fpga_program = Some(result);
        }
        FlowRequest::Learn { .. } => {
            return Err(AuroraError::Unsupported(
                "The learn flow runs in the AURORA server (aurora serve), which links aurora-learn"
                    .into(),
            ));
        }
    }
    if control.is_cancelled() {
        outcome.worsen(Status::Cancelled);
    }
    Ok(outcome)
}

/// [`run`] pedindo o bundle só depois das conferências que não precisam
/// dele: o recurso experimental do fluxo, os processadores que o pedido
/// nomeia e, num `build` de projeto sem processadores, nada a compilar (o
/// resultado sai sem o bundle). `toolchain` é como quem chama acha o bundle,
/// e o erro dele sai com o tipo de quem chama: a CLI e o servidor da AURORA
/// seguem a mesma ordem.
pub fn run_with<E: From<AuroraError>>(
    project: &mut Project,
    request: &FlowRequest,
    options: &FlowOptions,
    toolchain: impl FnOnce() -> std::result::Result<Toolchain, E>,
    progress: ProgressFn<'_>,
    control: &Control,
) -> std::result::Result<FlowOutcome, E> {
    if let Some(feature) = request.feature() {
        crate::experimental::require(feature)?;
    }
    for name in request.processors() {
        project.require_processor(name)?;
    }
    if let FlowRequest::Build { processors } = request
        && processors.is_empty()
        && project.processors().is_empty()
    {
        return Ok(FlowOutcome::new(request.name(), &options.command));
    }
    let toolchain = toolchain()?;
    Ok(run(
        &toolchain, project, request, options, progress, control,
    )?)
}

/// Os processadores pedidos pelo nome, ou todos. Um nome desconhecido é
/// erro antes de compilar qualquer um.
fn named(project: &Project, names: &[String]) -> Result<Vec<Processor>> {
    names
        .iter()
        .map(|name| project.require_processor(name).cloned())
        .collect()
}

fn build(
    flow: &Flow<'_>,
    project: &mut Project,
    processors: &[String],
    outcome: &mut FlowOutcome,
) -> Result<()> {
    // Um projeto só de Verilog não tem o que compilar, e isso não é erro.
    if processors.is_empty() && project.processors().is_empty() {
        return Ok(());
    }
    let targets = if processors.is_empty() {
        project.processors().to_vec()
    } else {
        named(project, processors)?
    };
    // Compila todos mesmo que um falhe, para mostrar todos os erros.
    flow.build(project, &targets, OnFailure::Continue, outcome)?;
    let recorded = flow.record(project, &flow.operation(outcome));
    store(outcome, recorded);
    Ok(())
}

fn check(
    flow: &Flow<'_>,
    project: &Project,
    file: Option<&camino::Utf8Path>,
    processor: Option<&str>,
    lint: bool,
    outcome: &mut FlowOutcome,
) -> Result<()> {
    if let Some(name) = processor {
        project.require_processor(name)?;
    }
    // Só o Icarus (e o Verilator, com lint) sobre o que está no disco: a
    // verificação não recompila os processadores.
    flow.say(Progress::Phase(Phase::Check));
    let options = CheckOptions {
        file: file.map(ToOwned::to_owned),
        processor: processor.map(ToOwned::to_owned),
        lint,
    };
    let result = crate::check(flow.toolchain, project, &options, flow.control)?;
    flow.say(Progress::Checked(&result));
    if lint
        && !result.steps.iter().any(|s| s.tool == Tool::Verilator)
        && let Err(AuroraError::ComponentMissing(component)) = flow.toolchain.tool(Tool::Verilator)
    {
        outcome.lint_missing = Some(component);
    }
    outcome.worsen(result.status);
    outcome.check = Some(result);
    let recorded = flow.record(
        project,
        &flow
            .operation(outcome)
            .with_check(outcome.check.as_ref().expect("preenchido acima")),
    );
    store(outcome, recorded);
    Ok(())
}

fn hierarchy(
    flow: &Flow<'_>,
    project: &Project,
    processor: Option<&str>,
    outcome: &mut FlowOutcome,
) -> Result<()> {
    if let Some(name) = processor {
        project.require_processor(name)?;
    }
    flow.say(Progress::Phase(Phase::Hierarchy));
    let options = HierarchyOptions {
        processor: processor.map(ToOwned::to_owned),
    };
    let result = crate::hierarchy(flow.toolchain, project, &options, flow.control)?;
    // Os que ficaram de fora por não terem sido compilados.
    let not_built: Vec<String> = match processor {
        Some(_) => Vec::new(),
        None => project
            .buildable_processors()
            .into_iter()
            .filter(|p| !p.is_built())
            .map(|p| p.name.clone())
            .collect(),
    };
    flow.say(Progress::Elaborated {
        result: &result,
        not_built: &not_built,
    });
    outcome.worsen(result.status);
    outcome.hierarchy = Some(result);
    outcome.not_built = not_built;
    Ok(())
}

fn simulate(
    flow: &Flow<'_>,
    project: &mut Project,
    processor: Option<&str>,
    testbench: Option<&camino::Utf8Path>,
    options: &SimulationOptions,
    open_wave: bool,
    outcome: &mut FlowOutcome,
) -> Result<()> {
    // O testbench escolhido grava o `.spf` antes de compilar.
    if let Some(testbench) = testbench {
        project.set_testbench(testbench)?;
    }
    // Pelo nome: o build grava no `.spf` o que gerou, e o processador é
    // procurado de novo depois.
    let name = processor
        .map(|name| project.require_processor(name).map(|p| p.name.clone()))
        .transpose()?;
    let targets: Vec<Processor> = match &name {
        Some(name) => vec![project.require_processor(name)?.clone()],
        None => project
            .buildable_processors()
            .into_iter()
            .cloned()
            .collect(),
    };
    let built = flow.build(project, &targets, OnFailure::Stop, outcome)?;
    if !built {
        flow.say(Progress::NotRun {
            what: Phase::Simulate,
            builds: &outcome.builds,
        });
        let recorded = flow.record(project, &flow.operation(outcome));
        store(outcome, recorded);
        return Ok(());
    }
    let processor = name
        .as_deref()
        .map(|name| project.require_processor(name))
        .transpose()?;

    flow.say(Progress::Phase(Phase::Simulate));
    let result = match processor {
        Some(p) => crate::simulate(flow.toolchain, p, options, flow.control)?,
        None => crate::simulate_project(flow.toolchain, project, options, flow.control)?,
    };
    let outputs = match processor {
        Some(p) if result.succeeded() => port_values(p, &result, flow.options.port_values_limit),
        _ => Vec::new(),
    };
    flow.say(Progress::Simulated {
        result: &result,
        outputs: &outputs,
    });
    outcome.worsen(result.status);
    outcome.outputs = outputs;
    outcome.simulation = Some(result);
    let recorded = flow.record(
        project,
        &flow
            .operation(outcome)
            .with_simulation(outcome.simulation.as_ref().expect("preenchido acima")),
    );
    store(outcome, recorded);

    // Como o botão Wave da AURORA e o `aurora sim --open`: a onda abre quando
    // a simulação a entrega e o relatório foi gravado.
    if open_wave
        && flow.options.wave == WavePolicy::Window
        && let Some(wave) = outcome
            .simulation
            .as_ref()
            .and_then(|s| s.waveform.as_ref())
    {
        flow.say(Progress::Phase(Phase::Wave));
        let request = WaveRequest {
            source: WaveSource::File {
                path: wave.path.clone(),
            },
            layout: LayoutPolicy::Generated,
        };
        match super::open_wave(flow.toolchain, Some(project), &request) {
            Ok(opened) => {
                flow.say(Progress::WaveOpened(&opened));
                outcome.wave = Some(opened);
            }
            Err(error) => outcome.wave_error = Some(ErrorInfo::from(error)),
        }
    }
    Ok(())
}

/// Os valores de cada porta de saída que a simulação do processador
/// escreveu, na ordem das portas, até `limit` por porta
/// ([`FlowOptions::port_values_limit`](super::FlowOptions::port_values_limit)).
/// Uma porta que não dá para ler (um `x` do simulador numa divisão por zero)
/// vem com o motivo em `error`, sem derrubar o resto.
fn port_values(
    processor: &Processor,
    result: &SimulationResult,
    limit: Option<usize>,
) -> Vec<PortValues> {
    processor
        .output_ports(result)
        .into_iter()
        .map(|port| {
            let path = processor.output_path(port);
            let (values, count, error) =
                match crate::files::read_data_file_head(&path, limit.unwrap_or(usize::MAX)) {
                    Ok((values, count)) => (values, count, None),
                    Err(error) => (Vec::new(), 0, Some(ErrorInfo::from(error))),
                };
            PortValues {
                port,
                path,
                values,
                count,
                error,
            }
        })
        .collect()
}

fn synthesize(
    flow: &Flow<'_>,
    project: &mut Project,
    processor: Option<&str>,
    schematic: bool,
    module: Option<&str>,
    open_viewer: bool,
    outcome: &mut FlowOutcome,
) -> Result<()> {
    // O topo do projeto sintetiza com o Verilog de todos os processadores;
    // com um processador, só o daquele.
    let (target, targets): (DesignTarget, Vec<Processor>) = match processor {
        Some(name) => {
            let p = project.require_processor(name)?;
            (DesignTarget::Processor(p.name.clone()), vec![p.clone()])
        }
        None => (
            DesignTarget::TopLevel,
            project
                .buildable_processors()
                .into_iter()
                .cloned()
                .collect(),
        ),
    };
    let built = flow.build(project, &targets, OnFailure::Stop, outcome)?;
    if !built {
        flow.say(Progress::NotRun {
            what: Phase::Synthesize,
            builds: &outcome.builds,
        });
        let recorded = flow.record(project, &flow.operation(outcome));
        store(outcome, recorded);
        return Ok(());
    }

    flow.say(Progress::Phase(Phase::Synthesize));
    let result = crate::synthesize(flow.toolchain, project, &target, flow.control)?;
    flow.say(Progress::Synthesized(&result));

    // Um módulo fora do netlist não desfaz a síntese: o relatório dela é
    // gravado, e o motivo vai em `schematic_error`. Outro erro do
    // esquemático é erro do fluxo, antes do relatório.
    if schematic && let Some(netlist) = &result.netlist {
        flow.say(Progress::Phase(Phase::Schematic));
        let module = module.unwrap_or(&result.top);
        match crate::render_schematic(
            flow.toolchain,
            netlist,
            module,
            &SchematicOptions::default(),
            flow.control,
        ) {
            Ok(drawn) => {
                flow.say(Progress::Drawn(&drawn));
                outcome.worsen(drawn.status);
                outcome.schematic = Some(drawn);
            }
            Err(error @ AuroraError::ModuleNotFound { .. }) => {
                outcome.schematic_error = Some(ErrorInfo::from(error));
            }
            Err(error) => return Err(error),
        }
    }
    outcome.worsen(result.status);
    outcome.synthesis = Some(result);
    let recorded = {
        let mut operation = flow
            .operation(outcome)
            .with_synthesis(outcome.synthesis.as_ref().expect("preenchido acima"));
        if let Some(drawn) = &outcome.schematic {
            operation = operation.with_schematic(drawn);
        }
        flow.record(project, &operation)
    };
    store(outcome, recorded);

    // O RTL viewer: o HTML do netlist, aberto no navegador (ou só gravado,
    // para quem chamou abrir). Um erro aqui vem depois do relatório gravado
    // e não desfaz a síntese.
    if open_viewer
        && let Some(synthesis) = &outcome.synthesis
        && synthesis.succeeded()
        && let Some(netlist) = &synthesis.netlist
    {
        flow.say(Progress::Phase(Phase::RtlViewer));
        let opener = flow
            .options
            .viewer_opener
            .as_deref()
            .filter(|o| !o.is_empty());
        let title = format!("{} (RTL viewer)", synthesis.top);
        let written = (|| -> Result<Utf8PathBuf> {
            let template = crate::rtl_viewer::template(flow.toolchain)?;
            let html = crate::rtl_viewer::write(&template, netlist, &title)?;
            if opener.is_none() {
                crate::open_with_system(&html)?;
            }
            Ok(html)
        })();
        match written {
            Ok(html) => {
                flow.say(Progress::RtlViewer {
                    path: &html,
                    opener,
                });
                outcome.rtl_viewer = Some(html);
            }
            Err(error) => outcome.rtl_viewer_error = Some(ErrorInfo::from(error)),
        }
    }
    Ok(())
}

fn fpga_build(flow: &Flow<'_>, project: &mut Project, outcome: &mut FlowOutcome) -> Result<()> {
    // O `fpga.json`, a placa e o Quartus antes de compilar qualquer coisa:
    // faltar um deles não deve custar o build dos processadores.
    let config = FpgaConfig::read(project.root())?;
    fpga::board(&config.board)?;
    if flow.toolchain.quartus().is_none() {
        return Err(AuroraError::QuartusMissing);
    }
    let targets: Vec<Processor> = project
        .buildable_processors()
        .into_iter()
        .cloned()
        .collect();
    let built = flow.build(project, &targets, OnFailure::Stop, outcome)?;
    if !built {
        flow.say(Progress::NotRun {
            what: Phase::Fpga,
            builds: &outcome.builds,
        });
        return Ok(());
    }
    flow.say(Progress::Phase(Phase::Fpga));
    let result = fpga::build(flow.toolchain, project, flow.control)?;
    flow.say(Progress::FpgaBuilt(&result));
    outcome.worsen(result.status);
    outcome.fpga = Some(result);
    Ok(())
}
