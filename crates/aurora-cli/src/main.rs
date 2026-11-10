//! `aurora`: a linha de comando da AURORA. Casca fina sobre o `aurora-core`:
//! parseia argumentos, chama o Core e formata a saída. Nenhuma regra de
//! negócio mora aqui.

// Os comentários de documentação das flags são o texto do `--help` (clap),
// em que `<DIR>` e `<testbench>` são marcadores, e não HTML: escapá-los
// mudaria a ajuda.
#![allow(rustdoc::invalid_html_tags)]

mod cancel;
mod chat;
mod commands;
mod experimental;
mod fpga;
mod install;
mod installation;
mod learn;
mod learn_tui;
mod mcp;
mod meter;
mod output;
mod progress;
#[cfg(test)]
mod protocol_doc;
mod report;
mod serve;
mod settings;
mod uninstall;
mod update;
mod update_check;
mod vboard;
mod wave_choice;

use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use aurora_core::CancelToken;
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand, ValueEnum};

use settings::ToolchainArgs;

/// Código de saída quando a operação rodou e não deu certo (erro de
/// compilação, simulação que falhou ou passou do prazo).
const EXIT_FAILED: u8 = 1;
/// Código de saída quando a AURORA nem conseguiu rodar (toolchain, projeto, I/O).
const EXIT_ERROR: u8 = 2;
/// Código de saída quando o usuário cancelou (Ctrl+C ou SIGTERM), o
/// mesmo que o shell usa para um programa interrompido pelo Ctrl+C.
const EXIT_CANCELLED: u8 = 130;

const AFTER_HELP: &str = "\
Examples:
  aurora new demo && cd demo
  aurora add counter.v counter_tb.v   create a module and its testbench
  aurora add test_counter.py          create a cocotb (Python) testbench
  aurora sim --open                   simulate and open the waveform
  aurora sim --fast                   simulate without waveform, on Verilator
  aurora proc add adder               create a SAPHO processor
  aurora sim -p adder                 build it with YANC and simulate it
  aurora learn init && cd aurora-learn && aurora learn
                                    learn Verilog with exercises

Exit status: 0 ok, 1 failed (build error, failed simulation, timeout),
2 could not run, 130 cancelled.";

#[derive(Parser)]
#[command(
    name = "aurora",
    version,
    about = "Verilog, VHDL and SAPHO processor development: Icarus, Verilator, GHDL, Yosys, YANC and surfer-aurora",
    after_help = AFTER_HELP
)]
struct Cli {
    /// Project: its .spf file or any folder inside it (default: the current folder)
    #[arg(
        short = 'C',
        long = "project",
        global = true,
        default_value = ".",
        value_name = "PATH",
        help_heading = "Global options"
    )]
    project: Utf8PathBuf,

    /// JSON output on stdout
    #[arg(long, global = true, help_heading = "Global options")]
    json: bool,

    /// JSON lines on stdout: one event per line, then the result
    #[arg(long, global = true, help_heading = "Global options")]
    events: bool,

    /// More output (-vv: debug log)
    #[arg(short, long, global = true, action = clap::ArgAction::Count, help_heading = "Global options")]
    verbose: u8,

    #[command(flatten)]
    toolchain: ToolchainArgs,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new project
    New {
        /// Project name, also used for its folder and .spf file
        name: String,
        /// Where to create the project folder
        #[arg(long, default_value = ".")]
        dir: Utf8PathBuf,
    },
    /// Show the project summary
    Status,
    /// Add Verilog or VHDL files or cocotb testbenches (.py); missing ones are created from a template
    Add {
        /// Verilog (.v, .sv) or VHDL (.vhd, .vhdl) files, or cocotb testbenches (.py)
        #[arg(required = true, value_name = "FILE")]
        files: Vec<Utf8PathBuf>,
        /// Add as testbench (default: detected from content or name)
        #[arg(long)]
        tb: bool,
    },
    /// Remove files from the project (keeps them on disk)
    Remove {
        /// Verilog, VHDL or cocotb files
        #[arg(required = true, value_name = "FILE")]
        files: Vec<Utf8PathBuf>,
    },
    /// Show or set the top module
    Top {
        /// File or module name (none: show the current top)
        #[arg(value_name = "FILE|MODULE")]
        target: Option<String>,
    },
    /// Move or rename files and folders; the project file follows
    Move {
        /// Only update the project file: the sources were already moved (by an editor, or with mv)
        #[arg(long)]
        after: bool,
        /// Files or folders to move
        #[arg(required = true, value_name = "SOURCE")]
        sources: Vec<Utf8PathBuf>,
        /// An existing folder (the sources go inside it), or the new path of a single source
        #[arg(value_name = "DEST")]
        dest: Utf8PathBuf,
    },
    /// Change the place of a file in its project list (the compilers read the files in that order)
    #[command(group(clap::ArgGroup::new("position").required(true).multiple(false)))]
    Order {
        /// The file to move in the list
        #[arg(value_name = "FILE")]
        file: Utf8PathBuf,
        /// Put it first
        #[arg(long, group = "position")]
        first: bool,
        /// Put it last
        #[arg(long, group = "position")]
        last: bool,
        /// Put it right before this file
        #[arg(long, value_name = "FILE", group = "position")]
        before: Option<Utf8PathBuf>,
        /// Put it right after this file
        #[arg(long, value_name = "FILE", group = "position")]
        after: Option<Utf8PathBuf>,
    },
    /// Manage SAPHO processors
    #[command(subcommand)]
    Proc(ProcCommand),
    /// Convert the processors from the folder layout of YANC 6.x (Software/, Hardware/,
    /// Simulation/) to the one of YANC 7.1; nothing is deleted or overwritten
    Convert,
    /// Compile processors to Verilog (YANC)
    Build(BuildArgs),
    /// Check that the Verilog elaborates (iverilog -tnull)
    Check(CheckArgs),
    /// Show the module hierarchy of the design and of each testbench, as Icarus elaborates it
    Hierarchy(HierarchyArgs),
    /// Simulate the project testbench or a processor
    Sim(SimArgs),
    /// Open a waveform in surfer-aurora
    Wave(WaveArgs),
    /// Synthesize with Yosys
    Synth(SynthArgs),
    /// Show and compare the reports of past operations
    Report(ReportArgs),
    /// Learn Verilog with exercises checked on every save, like rustlings (no subcommand: watch mode)
    Learn(LearnArgs),
    /// Experimental, off by default: put the design on an FPGA board with Quartus Prime (aurora experimental enable fpga)
    #[command(subcommand)]
    Fpga(FpgaCommand),
    /// Experimental, off by default: the virtual board, the project with the board peripherals simulated, without a window (aurora experimental enable fpga)
    #[command(subcommand)]
    Vboard(VboardCommand),
    /// Turn experimental components on and off (no subcommand: list them)
    Experimental(ExperimentalArgs),
    /// Serve the AURORA API to another program over stdin and stdout: the AURORA protocol (JSON-RPC 2.0, one message per line), or MCP when the client is an AI agent
    Serve(serve::ServeArgs),
    /// Experimental, off by default: the AURORA server always in MCP, for an AI agent; aurora serve also answers MCP (aurora experimental enable assistant)
    Mcp(mcp::McpArgs),
    /// Experimental, off by default: chat with the AURORA assistant (Claude Code with the AURORA tools) in the terminal (aurora experimental enable assistant)
    Chat(chat::ChatArgs),
    /// Show the bundled tools and their versions
    Tools {
        /// Verify each executable against the manifest SHA-256
        #[arg(long)]
        verify: bool,
    },
    /// Install bundle apps (none: choose from a list)
    Install(InstallArgs),
    /// Update AURORA and its bundle to the latest release
    Update {
        /// Only compare installed and published versions
        #[arg(long)]
        check: bool,
        /// Do not ask for confirmation
        #[arg(short, long)]
        yes: bool,
        /// Run the whole installer of the new version, instead of downloading only the files that changed
        #[arg(long)]
        full: bool,
        /// Update from a local AURORA installer of the new version (folder, .tar.gz, payload/, or a folder with the release files), without network
        #[arg(long, value_name = "PATH", conflicts_with = "full")]
        from: Option<Utf8PathBuf>,
        /// Run by AURORA IDE, which closes for the update: on Windows, the setup wizard opens it again when it finishes
        #[arg(long, hide = true)]
        reopen_ide: bool,
    },
    /// Install the bundle apps from the downloaded release files (run by the Windows web installer)
    #[command(hide = true)]
    Setup {
        /// The folder with the downloaded release files (the index and the chunks)
        #[arg(long, value_name = "DIR")]
        payload: Utf8PathBuf,
        /// The apps to install, comma-separated (none: only the bundle header)
        #[arg(long, value_delimiter = ',')]
        components: Vec<String>,
        /// The installation folder; the bundle goes to <DIR>/toolchain
        #[arg(long, value_name = "DIR")]
        prefix: Utf8PathBuf,
    },
    /// Uninstall AURORA and its bundle
    Uninstall {
        /// Do not ask for confirmation
        #[arg(short, long)]
        yes: bool,
    },
    /// Generate shell completions
    #[command(hide = true)]
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

/// `aurora report`: sem subcomando, o relatório mais novo.
#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
struct ReportArgs {
    #[command(subcommand)]
    command: Option<ReportCommand>,
}

#[derive(Subcommand)]
enum ReportCommand {
    /// List the stored reports, newest first
    List {
        /// Show at most N reports
        #[arg(long, value_name = "N", value_parser = clap::value_parser!(u64).range(1..))]
        limit: Option<u64>,
    },
    /// Print a stored report
    Show {
        /// Report ID (run-000042, or just 42)
        id: String,
    },
    /// Compare synthesis statistics and simulation timings of two reports
    Compare {
        /// Current report (default: the latest)
        id: Option<String>,
        /// Baseline report (default: the latest earlier comparable one)
        #[arg(long, value_name = "ID")]
        against: Option<String>,
        /// Show only the summary
        #[arg(long)]
        summary: bool,
    },
    /// Remove stored reports (default: all of them)
    Clean {
        /// Remove only these reports (run-000042, or just 42)
        #[arg(value_name = "ID", conflicts_with = "keep")]
        ids: Vec<String>,
        /// Keep the N newest reports and remove the rest
        #[arg(long, value_name = "N")]
        keep: Option<u64>,
        /// Do not ask for confirmation
        #[arg(short, long)]
        yes: bool,
    },
}

/// `aurora learn`: sem subcomando, o modo watch.
#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
struct LearnArgs {
    #[command(subcommand)]
    command: Option<LearnCommand>,
}

#[derive(Subcommand)]
enum LearnCommand {
    /// Create the exercise folder, with every exercise of a track
    Init {
        /// Where to create it
        #[arg(default_value = learn::DEFAULT_DIR)]
        dir: Utf8PathBuf,
        /// The track
        #[arg(long, default_value = aurora_learn::DEFAULT_TRACK)]
        track: String,
    },
    /// Check an exercise (default: the current one) and record whether it is solved
    Check {
        /// Exercise name
        #[arg(conflicts_with = "all")]
        name: Option<String>,
        /// Check every exercise
        #[arg(long)]
        all: bool,
    },
    /// List the exercises and which are solved
    List,
    /// Show the hints of an exercise (default: the current one)
    Hint {
        /// Exercise name
        name: Option<String>,
    },
    /// Restore the file of an exercise to its starting point
    Reset {
        /// Exercise name
        name: String,
        /// Do not ask for confirmation
        #[arg(short, long)]
        yes: bool,
    },
    /// Open the waveform of the last check, with the first mismatch marked
    Wave {
        /// Exercise name (default: the current one)
        name: Option<String>,
    },
    /// Tools for writing tracks
    #[command(subcommand)]
    Dev(LearnDevCommand),
}

#[derive(Subcommand)]
enum LearnDevCommand {
    /// Check a track: every starting file fails and every solution passes
    Check {
        /// The track folder (default: the installed verilog track, or the one in AURORA_LEARN_DIR)
        track: Option<Utf8PathBuf>,
    },
}

/// `aurora experimental`: sem subcomando, a lista.
#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
struct ExperimentalArgs {
    #[command(subcommand)]
    command: Option<ExperimentalCommand>,
}

#[derive(Subcommand)]
enum ExperimentalCommand {
    /// Turn an experimental component on, for the CLI and AURORA IDE
    Enable {
        /// The component
        #[arg(value_parser = experimental::component_parser())]
        component: String,
    },
    /// Turn an experimental component off, for the CLI and AURORA IDE
    Disable {
        /// The component
        #[arg(value_parser = experimental::component_parser())]
        component: String,
    },
}

#[derive(Subcommand)]
enum FpgaCommand {
    /// List the known boards, or show the signals and pins of one
    Boards {
        /// Board id (e.g. de2-115)
        board: Option<String>,
    },
    /// Check fpga.json against the board and the top module, and show the connections
    Check {
        /// Also print the generated board top (Verilog)
        #[arg(long)]
        show_top: bool,
    },
    /// Build the processors, then compile the project for the board with Quartus Prime
    Build,
    /// Program the board with the last build (Quartus Programmer, over JTAG)
    Program {
        /// Cable, as `--list` shows it (default: the first one)
        #[arg(long, value_name = "NAME")]
        cable: Option<String>,
        /// Only list the programming cables connected
        #[arg(long, conflicts_with = "cable")]
        list: bool,
    },
}

#[derive(Subcommand)]
enum VboardCommand {
    /// Build the project with the board peripherals, run it for a while and show the LEDs, displays, UART and video
    Run(VboardRunArgs),
}

#[derive(clap::Args)]
struct VboardRunArgs {
    /// Board, for a project without fpga.json: ports named like the board signals connect to them (e.g. de2-115)
    #[arg(long, value_name = "ID")]
    board: Option<String>,
    /// Simulated time to run, in milliseconds
    #[arg(long, value_name = "MS", default_value_t = 50.0)]
    time: f64,
    /// Switches up: NAME=VALUE, the value in decimal, 0x or 0b (SW=0x3); repeatable
    #[arg(long, value_name = "NAME=VALUE")]
    set: Vec<String>,
    /// Buttons held down: NAME=MASK (KEY=0b1); repeatable
    #[arg(long, value_name = "NAME=MASK")]
    press: Vec<String>,
    /// Text typed on the PS/2 keyboard (US layout keys)
    #[arg(long, value_name = "TEXT")]
    keys: Option<String>,
    /// Text sent to the UART
    #[arg(long, value_name = "TEXT")]
    uart: Option<String>,
    /// Write the last video frame to this PNG file
    #[arg(long, value_name = "FILE")]
    png: Option<camino::Utf8PathBuf>,
    /// Speed limit: 1 is real time, 0 is as fast as possible
    #[arg(long, value_name = "FACTOR", default_value_t = 0.0)]
    speed: f64,
}

#[derive(Subcommand)]
enum ProcCommand {
    /// Create a processor
    Add(ProcAddArgs),
    /// Set frequency, clocks and array export
    Set {
        /// Processor name (default: the processor of the current folder)
        name: Option<String>,
        /// Frequency in MHz
        #[arg(long)]
        freq: Option<u32>,
        /// Clock cycles the testbench simulates
        #[arg(long)]
        clocks: Option<u32>,
        /// Export arrays to the simulation
        #[arg(long, value_name = "true|false")]
        arrays: Option<bool>,
    },
    /// Remove a processor from the project; its folder stays on disk unless --delete
    Remove {
        /// Processor name
        name: String,
        /// Also move the processor folder (source, inputs, generated files) to the trash
        #[arg(long)]
        delete: bool,
        /// Do not ask for confirmation before --delete
        #[arg(short, long, requires = "delete")]
        yes: bool,
    },
}

#[derive(clap::Args)]
struct ProcAddArgs {
    /// Letters, digits and _, starting with a letter or _
    name: String,
    /// Language: cmm (C±) or cpp (C)
    #[arg(long, value_enum, default_value_t = Lang::Cmm, hide_possible_values = true)]
    lang: Lang,
    /// Input ports (#NUIOIN)
    #[arg(long, default_value_t = 1)]
    inputs: u32,
    /// Output ports (#NUIOOU)
    #[arg(long, default_value_t = 1)]
    outputs: u32,
    /// Word width (#NUBITS); default: #NBMANT + #NBEXPO + 1
    #[arg(long, help_heading = "C± only")]
    nubits: Option<u32>,
    /// Mantissa bits (#NBMANT)
    #[arg(long, help_heading = "C± only")]
    nbmant: Option<u32>,
    /// Exponent bits (#NBEXPO)
    #[arg(long, help_heading = "C± only")]
    nbexpo: Option<u32>,
    /// norm(x) gain, a power of two (#NUGAIN); default: 128
    #[arg(long, help_heading = "C± only")]
    nugain: Option<u32>,
    /// Data stack depth (#NDSTAC); default: YANC computes it from the program
    #[arg(long, help_heading = "C± only")]
    ndstac: Option<u32>,
    /// Instruction stack depth (#SDEPTH); default: YANC computes it from the program
    #[arg(long, help_heading = "C± only")]
    sdepth: Option<u32>,
}

#[derive(clap::Args)]
struct BuildArgs {
    /// Processor to build (repeatable; default: the processor of the current folder, or all)
    #[arg(short, long = "processor", value_name = "NAME")]
    processors: Vec<String>,
}

#[derive(clap::Args)]
struct InstallArgs {
    /// yanc, icarus, verilator, ghdl, cocotb, yosys, graphviz, surfer-aurora, openfpgaloader, studio, aurora-learn, rtl-viewer
    #[arg(value_name = "APP")]
    components: Vec<String>,
    /// Install from a local AURORA installer (folder, .tar.gz, payload/, or a folder with the release files) instead of downloading
    #[arg(long, value_name = "PATH")]
    from: Option<Utf8PathBuf>,
}

#[derive(clap::Args)]
struct CheckArgs {
    /// Check only the modules in this file (default: the project and each testbench)
    #[arg(value_name = "FILE", conflicts_with = "processor")]
    file: Option<Utf8PathBuf>,
    /// Check this processor and its YANC testbench (default: the processor of the current folder)
    #[arg(short, long, value_name = "NAME")]
    processor: Option<String>,
    /// Also run the Verilator linter
    #[arg(long)]
    lint: bool,
}

#[derive(clap::Args)]
struct HierarchyArgs {
    /// Show this processor and its YANC testbench (default: the processor of the current folder)
    #[arg(short, long, value_name = "NAME")]
    processor: Option<String>,
}

#[derive(clap::Args)]
struct SimArgs {
    /// Testbench to simulate (Verilog, or cocotb .py); becomes the project default
    #[arg(value_name = "TESTBENCH", conflicts_with = "processor")]
    testbench: Option<Utf8PathBuf>,
    /// Simulate this processor with its YANC testbench (default: the processor of the current folder)
    #[arg(short, long, value_name = "NAME")]
    processor: Option<String>,
    /// Use Verilator instead of Icarus
    #[arg(long)]
    verilator: bool,
    /// Fast simulation, without waveform: Verilator for a Verilog testbench or a processor; a cocotb testbench runs its tests on Icarus (on Verilator with --verilator)
    #[arg(long, conflicts_with = "open")]
    fast: bool,
    /// Stop the simulation after this many seconds
    #[arg(long, value_name = "SECONDS", value_parser = clap::value_parser!(u64).range(1..))]
    timeout: Option<u64>,
    /// Open the waveform when done
    #[arg(long)]
    open: bool,
}

#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
struct WaveArgs {
    #[command(subcommand)]
    command: Option<WaveCommand>,
    /// VCD, FST or GHW file (default: the project simulation)
    #[arg(value_name = "WAVEFORM", conflicts_with = "processor")]
    waveform: Option<Utf8PathBuf>,
    /// Open this processor's waveform (default: the processor of the current folder)
    #[arg(short, long, value_name = "NAME")]
    processor: Option<String>,
    /// Open without the layout (the SAPHO processor groups and the saved layout)
    #[arg(long)]
    no_layout: bool,
    /// Open with the generated layout, deleting the one saved in wave/<testbench>.surf.ron
    #[arg(long, conflicts_with = "no_layout")]
    reset_layout: bool,
}

/// `aurora wave signals|select|unselect`: a escolha dos sinais da onda do
/// testbench do projeto.
#[derive(Subcommand)]
enum WaveCommand {
    /// The signals of the project testbench, with the ones the waveform records marked
    Signals,
    /// Record in the waveform only the chosen signals and scopes; these are added to the choice
    Select {
        /// Signals or scopes, by their path (mux_tb.dut, mux_tb.dut.sel)
        #[arg(value_name = "PATH", required_unless_present = "all")]
        paths: Vec<String>,
        /// Clear the choice: the waveform records every signal again
        #[arg(long, conflicts_with = "paths")]
        all: bool,
    },
    /// Take signals or scopes out of the choice
    Unselect {
        /// Signals or scopes, by their path
        #[arg(value_name = "PATH", required = true)]
        paths: Vec<String>,
    },
}

#[derive(clap::Args)]
struct SynthArgs {
    /// Synthesize this processor (default: the processor of the current folder, or the top module)
    #[arg(short, long, value_name = "NAME")]
    processor: Option<String>,
    /// Draw the schematic as SVG (needs Graphviz)
    #[arg(long)]
    svg: bool,
    /// Module to draw (default: top; -v lists them)
    #[arg(long, requires = "svg")]
    module: Option<String>,
    /// Open the interactive schematic of the netlist (RTL viewer) in the browser
    #[arg(long)]
    open: bool,
    /// Aceito e ignorado: o esquemático não tem mais teto de ligações. Fica
    /// para não quebrar quem já o passa.
    #[arg(long, hide = true)]
    no_schematic_limit: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum Lang {
    Cmm,
    Cpp,
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => return usage_error(&error),
    };
    init_tracing(cli.verbose);
    let mode = if cli.events {
        output::Mode::Events
    } else if cli.json {
        output::Mode::Json
    } else {
        output::Mode::Text
    };
    let out = output::Output::new(mode, cli.verbose > 0);
    // O `aurora serve` encerra do jeito dele no primeiro sinal (cancela,
    // espera, sai); sair na hora no segundo deixaria o que roda órfão.
    let cancel = cancel_on_signals(!matches!(
        cli.command,
        Command::Serve(_) | Command::Mcp(_) | Command::Chat(_)
    ));
    cancel::set(cancel.clone());
    let control = out.control(cancel.clone());
    // A consulta da última versão roda enquanto o comando roda.
    let update_check = update_check::start(out.is_text() && cli.command.checks_for_updates());

    let code = match commands::run(&cli, &out, &control) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(EXIT_FAILED),
        Err(error) => {
            out.error(&error);
            ExitCode::from(EXIT_ERROR)
        }
    };
    if cancel.is_cancelled() {
        ExitCode::from(EXIT_CANCELLED)
    } else {
        update_check::finish(update_check);
        code
    }
}

impl Command {
    /// O comando avisa no fim quando há AURORA mais nova (`update_check`). Fora
    /// ficam o `aurora update`, que já compara as versões, o `aurora uninstall`
    /// e os comandos internos.
    fn checks_for_updates(&self) -> bool {
        !matches!(
            self,
            Command::Update { .. }
                | Command::Uninstall { .. }
                | Command::Setup { .. }
                | Command::Serve(_)
                | Command::Mcp(_)
                | Command::Chat(_)
                | Command::Completions { .. }
        )
    }
}

/// Um erro que o clap achou nos argumentos. A ajuda e a versão saem como o
/// clap as escreve. Os erros saem com `Error:` e `Tip:` em maiúscula, como os
/// da AURORA, e com cor nas mesmas condições que o clap usaria (o `anstream`
/// tira os códigos fora de um terminal ou com `NO_COLOR`).
fn usage_error(error: &clap::Error) -> ExitCode {
    use clap::error::ErrorKind;
    if matches!(
        error.kind(),
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    ) {
        error.exit();
    }
    let text = capitalize_labels(&error.render().ansi().to_string());
    let _ = std::io::Write::write_all(&mut anstream::stderr(), text.as_bytes());
    ExitCode::from(u8::try_from(error.exit_code()).unwrap_or(EXIT_ERROR))
}

/// As linhas que começam com `error:` ou `tip:` (depois de espaços e
/// códigos ANSI) ganham maiúscula no rótulo e no começo da mensagem:
/// `error: unexpected argument` vira `Error: Unexpected argument`.
fn capitalize_labels(text: &str) -> String {
    text.split_inclusive('\n')
        .map(|line| {
            let start = visible_start(line);
            for label in ["error:", "tip:"] {
                if let Some(rest) = line[start..].strip_prefix(label) {
                    let at = visible_start(rest);
                    return format!(
                        "{}{}{}{}",
                        &line[..start],
                        capitalize(label),
                        &rest[..at],
                        capitalize(&rest[at..])
                    );
                }
            }
            line.to_owned()
        })
        .collect()
}

/// Onde começa o texto visível: depois dos espaços e das sequências ANSI
/// (`ESC [ ... letra`).
fn visible_start(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut i = 0;
    loop {
        match bytes.get(i) {
            Some(b' ') => i += 1,
            Some(0x1b) => {
                i += 1;
                if bytes.get(i) == Some(&b'[') {
                    i += 1;
                    while let Some(b) = bytes.get(i) {
                        i += 1;
                        if b.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            }
            _ => return i,
        }
    }
}

/// O texto com a primeira letra em maiúscula.
fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// O pedido de cancelamento que o Ctrl+C (SIGINT) e o SIGTERM fazem; no
/// Unix, também o SIGHUP de um terminal fechado. O primeiro sinal pede o
/// cancelamento: o Core encerra a ferramenta que roda, com tudo o que ela
/// iniciou, e o comando termina mostrando o que chegou a rodar. Com
/// `exit_on_second`, o segundo sai na hora, com 130.
///
/// Sem isto, o sinal mataria só a AURORA: as ferramentas rodam num grupo de
/// processos próprio e continuariam rodando sozinhas.
fn cancel_on_signals(exit_on_second: bool) -> CancelToken {
    use signal_hook::consts::signal::{SIGINT, SIGTERM};
    let flag = Arc::new(AtomicBool::new(false));
    #[cfg(unix)]
    let signals = [SIGINT, SIGTERM, signal_hook::consts::signal::SIGHUP];
    #[cfg(not(unix))]
    let signals = [SIGINT, SIGTERM];
    for signal in signals {
        // A ordem importa: a saída imediata confere a flag antes de a
        // segunda chamada marcá-la, então só dispara a partir do segundo sinal.
        let registered = if exit_on_second {
            signal_hook::flag::register_conditional_shutdown(
                signal,
                i32::from(EXIT_CANCELLED),
                Arc::clone(&flag),
            )
            .map(|_| ())
        } else {
            Ok(())
        }
        .and_then(|()| signal_hook::flag::register(signal, Arc::clone(&flag)));
        if let Err(error) = registered {
            tracing::warn!(signal, %error, "No signal handler, Ctrl+C will not cancel");
        }
    }
    CancelToken::from(flag)
}

/// O log do Core (`tracing`) vai para o stderr: avisos por padrão, `info`
/// com -v, `debug` com -vv. `RUST_LOG` tem precedência.
fn init_tracing(verbose: u8) {
    let default = match verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default));
    // Cor no log só num terminal e sem NO_COLOR, como no resto da saída.
    let ansi = std::io::IsTerminal::is_terminal(&std::io::stderr())
        && std::env::var_os("NO_COLOR").is_none();
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_ansi(ansi)
        .init();
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    /// Confere nomes duplicados, conflitos e flags de todos os subcomandos,
    /// coisa que o clap só verificaria ao rodar cada um.
    #[test]
    fn cli_definition_is_valid() {
        super::Cli::command().debug_assert();
    }

    #[test]
    fn clap_labels_are_capitalized() {
        let plain = "error: unexpected argument '--x' found\n\n  \
                     tip: a similar argument exists: '--json'\n\nUsage: aurora\n";
        assert_eq!(
            super::capitalize_labels(plain),
            "Error: Unexpected argument '--x' found\n\n  \
             Tip: A similar argument exists: '--json'\n\nUsage: aurora\n"
        );
        let colored = "\x1b[1m\x1b[31merror:\x1b[0m 'aurora' requires a subcommand\n";
        assert_eq!(
            super::capitalize_labels(colored),
            "\x1b[1m\x1b[31mError:\x1b[0m 'aurora' requires a subcommand\n"
        );
        let usage = "  <NAME>\n";
        assert_eq!(super::capitalize_labels(usage), usage);
    }
}
