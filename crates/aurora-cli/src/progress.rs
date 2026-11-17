//! A barra que mostra que uma ferramenta está rodando: o que ela faz, uma
//! animação e o tempo decorrido, numa linha do stderr que se reescreve.
//!
//! Só aparece em texto e com o stderr num terminal: num pipe, num log do CI
//! ou com `--json`, não há barra. É indeterminada, porque a AURORA não sabe
//! quanto falta: nem o `vvp` nem o modelo do Verilator informam o progresso
//! da simulação. Só aparece depois de [`DELAY`], para um passo rápido (um
//! compilador do YANC, alguns ms) não piscar na tela.
//!
//! A saída que sai enquanto a barra está na tela (o `$display` do testbench,
//! o `-v`) passa por [`Progress::println`], que apaga a barra antes de
//! escrever; a barra volta no próximo quadro.

use std::io::{IsTerminal, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use aurora_core::{Step, Tool};

/// Quanto um passo roda antes de a barra aparecer.
const DELAY: Duration = Duration::from_millis(300);
/// Intervalo entre quadros.
const FRAME: Duration = Duration::from_millis(100);
/// Largura da trilha e do bloco que corre nela.
const TRACK: usize = 24;
const BLOCK: usize = 6;

pub struct Progress {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    /// O passo que roda, e desde quando; `None` entre passos.
    running: Option<(String, Instant)>,
    /// Quantos caracteres a barra ocupa agora na linha (0: nada na tela).
    drawn: usize,
    frame: usize,
}

impl Progress {
    /// A barra, com a thread que a anima, se o stderr é um terminal.
    pub fn for_terminal() -> Option<Arc<Progress>> {
        if !std::io::stderr().is_terminal() {
            return None;
        }
        let progress = Arc::new(Progress {
            state: Mutex::new(State::default()),
        });
        let ticker = Arc::clone(&progress);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(FRAME);
                ticker.tick();
            }
        });
        Some(progress)
    }

    /// Um passo começou.
    pub fn start(&self, step: Step, tool: Tool) {
        let mut state = self.lock();
        clear(&mut state);
        state.running = Some((describe(step, tool), Instant::now()));
        state.frame = 0;
    }

    /// O passo terminou: a barra sai da tela.
    pub fn stop(&self) {
        let mut state = self.lock();
        clear(&mut state);
        state.running = None;
    }

    /// Uma linha no stdout, com a barra fora do caminho.
    pub fn println(&self, line: &str) {
        let mut state = self.lock();
        clear(&mut state);
        anstream::println!("{line}");
    }

    fn tick(&self) {
        let mut state = self.lock();
        let Some((label, started)) = &state.running else {
            return;
        };
        let elapsed = started.elapsed();
        if elapsed < DELAY {
            return;
        }
        // Um bloco que vai e volta na trilha.
        let span = TRACK - BLOCK;
        let position = state.frame % (2 * span);
        let offset = if position <= span {
            position
        } else {
            2 * span - position
        };
        let track: String = (0..TRACK)
            .map(|i| {
                if (offset..offset + BLOCK).contains(&i) {
                    '='
                } else {
                    ' '
                }
            })
            .collect();
        let line = format!("  {label} [{track}] {:.1} s", elapsed.as_secs_f64());
        let width = line.chars().count();
        let pad = state.drawn.saturating_sub(width);
        let mut stderr = std::io::stderr().lock();
        let _ = write!(stderr, "\r{line}{}", " ".repeat(pad));
        let _ = stderr.flush();
        state.drawn = width;
        state.frame += 1;
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Apaga a barra com espaços e volta ao começo da linha: funciona em todo
/// terminal, sem sequência de escape.
fn clear(state: &mut State) {
    if state.drawn == 0 {
        return;
    }
    let mut stderr = std::io::stderr().lock();
    let _ = write!(stderr, "\r{}\r", " ".repeat(state.drawn));
    let _ = stderr.flush();
    state.drawn = 0;
}

/// O que o passo faz, para a barra.
fn describe(step: Step, tool: Tool) -> String {
    let what = match step {
        Step::Preprocess => "Preprocessing",
        Step::Compile => "Compiling",
        Step::PreAssemble => "Counting instructions",
        Step::Assemble => "Assembling",
        Step::CheckSyntax => "Checking",
        Step::Lint => "Linting",
        Step::Analyze => "Reading the VHDL",
        Step::Elaborate if tool == Tool::Ghdl => "Analyzing and elaborating the VHDL",
        Step::Elaborate => "Compiling the Verilog",
        Step::Verilate => "Compiling the model",
        Step::Simulate => "Simulating",
        Step::Synthesize => "Synthesizing",
        Step::Graph => "Drawing the graph",
        Step::Render => "Rendering the schematic",
        Step::Fit => "Placing and routing",
        Step::Bitstream => "Writing the bitstream",
        Step::Timing => "Analyzing timing",
        Step::Program => "Programming the board",
        _ => "Running",
    };
    format!("{what} ({tool})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_step_says_what_it_does() {
        assert_eq!(describe(Step::Simulate, Tool::Vvp), "Simulating (vvp)");
        assert_eq!(
            describe(Step::Verilate, Tool::Verilator),
            "Compiling the model (verilator)"
        );
    }
}
