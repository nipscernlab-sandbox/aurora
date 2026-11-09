// GERADO por scripts/gen-aurora-types.mjs a partir de aurora/docs/schema/.
// Não edite à mão: rode `npm run gen:types`.
//
// São os tipos do Core da AURORA serializados (o mesmo JSON do `--json` da
// CLI). Os nomes seguem os tipos Rust: BuildResult, SimulationResult,
// Diagnostic, Event, RunComparison.

/* eslint-disable */
/**
 * Em qual lista do `.spf` um arquivo fica. Em JSON: `"synthesizable"` ou
 * `"testbench"`.
 *
 * [`Project::add_verilog`] decide o papel pelo conteúdo, como a AURORA
 * ([`classify`](crate::verilog::classify)); [`Project::add_file`] recebe o
 * papel de quem chama. Um arquivo fica numa lista só.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FileRole".
 */
export type FileRole = 'synthesizable' | 'testbench';
/**
 * Um passo de uma operação. Em JSON, em `snake_case` (`"pre_assemble"`).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Step".
 */
export type Step =
  | 'preprocess'
  | 'compile'
  | 'pre_assemble'
  | 'assemble'
  | 'check_syntax'
  | 'lint'
  | 'elaborate'
  | 'verilate'
  | 'simulate'
  | 'synthesize'
  | 'graph'
  | 'render'
  | 'fit'
  | 'bitstream'
  | 'timing'
  | 'program';
/**
 * Linguagem do programa de um processador. Em JSON: `"cmm"` ou `"cpp"`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Language".
 */
export type Language = 'cmm' | 'cpp';
/**
 * Como uma operação terminou. Em JSON, em `snake_case` (`"succeeded"`).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Status".
 */
export type Status = 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
/**
 * As ferramentas que a AURORA sabe orquestrar.
 *
 * Em JSON e em [`Display`](fmt::Display), cada uma aparece pelo nome do
 * programa ([`Tool::binary_name`]).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Tool".
 */
export type Tool =
  | 'cmmcomp'
  | 'appcomp'
  | 'asmcomp'
  | 'cpppp'
  | 'cppcomp'
  | 'iverilog'
  | 'vvp'
  | 'verilator'
  | 'yosys'
  | 'dot'
  | 'surfer'
  | 'openfpgaloader'
  | 'quartus'
  | 'perl';
/**
 * Como um processo terminou.
 *
 * Em JSON: `{"kind": "exited", "value": 1}`, `{"kind": "signaled",
 * "value": 11}`, `{"kind": "exception", "value": 3221225477}`,
 * `{"kind": "cancelled"}`, `{"kind": "timed_out"}`, `{"kind": "unknown"}`.
 *
 * Um código de saída e um sinal são coisas diferentes: `msg_internal` do
 * `cppcomp` chama `abort()`, e um `asmcomp` que perde um `fopen` morre por
 * segfault. Nenhum dos dois é um "erro de compilação" do usuário.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Termination".
 */
export type Termination =
  | {
      kind: 'exited';
      value: number;
    }
  | {
      kind: 'signaled';
      value: number;
    }
  | {
      kind: 'exception';
      value: number;
    }
  | {
      kind: 'cancelled';
    }
  | {
      kind: 'timed_out';
    }
  | {
      kind: 'unknown';
    };
/**
 * Gravidade de um [`Diagnostic`]. A ordem (`Error < Warning < Info <
 * Unknown`) serve para ordenar do mais grave ao menos grave.
 *
 * Em JSON: `"error"`, `"warning"`, `"info"`, `"unknown"`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Severity".
 */
export type Severity = 'error' | 'warning' | 'info' | 'unknown';
/**
 * O papel de um arquivo produzido por uma operação. Em JSON, em
 * `snake_case` (`"data_memory"`).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ArtifactKind".
 */
export type ArtifactKind =
  | 'assembly'
  | 'verilog'
  | 'data_memory'
  | 'instruction_memory'
  | 'testbench'
  | 'preprocessed_source'
  | 'program_counter_map'
  | 'source_translation'
  | 'opcode_translation'
  | 'compiler_log'
  | 'pre_assembler_log'
  | 'icarus_image'
  | 'verilated_model'
  | 'waveform'
  | 'simulation_output'
  | 'netlist'
  | 'schematic_graph'
  | 'schematic'
  | 'synthesis_statistics'
  | 'board_top'
  | 'quartus_project'
  | 'sram_object'
  | 'raw_binary'
  | 'serial_vector_format';
/**
 * O que aconteceu durante uma operação, avisado na hora em que acontece.
 *
 * Em JSON, com o tipo no campo `event`:
 *
 * ```json
 * { "event": "step_started", "step": "simulate", "tool": "vvp", "command": { "...": "..." } }
 * { "event": "output", "step": "simulate", "tool": "vvp", "stream": "stdout",
 *   "line": "t=10 y=1", "diagnostic": false }
 * { "event": "step_finished", "step": "simulate", "tool": "vvp",
 *   "termination": { "kind": "exited", "value": 0 }, "duration_ms": 12 }
 * ```
 *
 * Os mesmos dados chegam depois no resultado da operação
 * ([`StepReport`](crate::StepReport)); os eventos só os adiantam.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Event".
 */
export type Event =
  | {
      /**
       * Um passo de uma operação. Em JSON, em `snake_case` (`"pre_assemble"`).
       */
      step:
        | 'preprocess'
        | 'compile'
        | 'pre_assemble'
        | 'assemble'
        | 'check_syntax'
        | 'lint'
        | 'elaborate'
        | 'verilate'
        | 'simulate'
        | 'synthesize'
        | 'graph'
        | 'render'
        | 'fit'
        | 'bitstream'
        | 'timing'
        | 'program';
      /**
       * A ferramenta.
       */
      tool:
        | 'cmmcomp'
        | 'appcomp'
        | 'asmcomp'
        | 'cpppp'
        | 'cppcomp'
        | 'iverilog'
        | 'vvp'
        | 'verilator'
        | 'yosys'
        | 'dot'
        | 'surfer'
        | 'openfpgaloader'
        | 'quartus'
        | 'perl';
      command: Invocation2;
      event: 'step_started';
    }
  | {
      /**
       * Um passo de uma operação. Em JSON, em `snake_case` (`"pre_assemble"`).
       */
      step:
        | 'preprocess'
        | 'compile'
        | 'pre_assemble'
        | 'assemble'
        | 'check_syntax'
        | 'lint'
        | 'elaborate'
        | 'verilate'
        | 'simulate'
        | 'synthesize'
        | 'graph'
        | 'render'
        | 'fit'
        | 'bitstream'
        | 'timing'
        | 'program';
      /**
       * A ferramenta que escreveu.
       */
      tool:
        | 'cmmcomp'
        | 'appcomp'
        | 'asmcomp'
        | 'cpppp'
        | 'cppcomp'
        | 'iverilog'
        | 'vvp'
        | 'verilator'
        | 'yosys'
        | 'dot'
        | 'surfer'
        | 'openfpgaloader'
        | 'quartus'
        | 'perl';
      /**
       * stdout ou stderr.
       */
      stream: 'stdout' | 'stderr';
      /**
       * A linha, decodificada como UTF-8 (bytes inválidos viram `U+FFFD`).
       */
      line: string;
      /**
       * A linha é uma mensagem da ferramenta, que volta interpretada em
       * `diagnostics` no resultado: tudo do stderr, todo o stdout dos
       * compiladores, e no stdout de uma simulação as linhas do próprio
       * simulador (`$finish called at`, `VCD info:`, o `ERROR:` de um
       * `$error`). `false` é saída do programa, como o `$display` do
       * testbench. Decidido linha a linha: a continuação indentada de uma
       * mensagem conta como `false`.
       */
      diagnostic: boolean;
      event: 'output';
    }
  | {
      /**
       * Um passo de uma operação. Em JSON, em `snake_case` (`"pre_assemble"`).
       */
      step:
        | 'preprocess'
        | 'compile'
        | 'pre_assemble'
        | 'assemble'
        | 'check_syntax'
        | 'lint'
        | 'elaborate'
        | 'verilate'
        | 'simulate'
        | 'synthesize'
        | 'graph'
        | 'render'
        | 'fit'
        | 'bitstream'
        | 'timing'
        | 'program';
      /**
       * A ferramenta.
       */
      tool:
        | 'cmmcomp'
        | 'appcomp'
        | 'asmcomp'
        | 'cpppp'
        | 'cppcomp'
        | 'iverilog'
        | 'vvp'
        | 'verilator'
        | 'yosys'
        | 'dot'
        | 'surfer'
        | 'openfpgaloader'
        | 'quartus'
        | 'perl';
      /**
       * Como terminou.
       */
      termination:
        | {
            kind: 'exited';
            value: number;
          }
        | {
            kind: 'signaled';
            value: number;
          }
        | {
            kind: 'exception';
            value: number;
          }
        | {
            kind: 'cancelled';
          }
        | {
            kind: 'timed_out';
          }
        | {
            kind: 'unknown';
          };
      /**
       * Duração, em milissegundos.
       */
      duration_ms: number;
      event: 'step_finished';
    };
/**
 * De qual saída do processo veio uma linha. Em JSON: `"stdout"` ou
 * `"stderr"`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Stream".
 */
export type Stream = 'stdout' | 'stderr';
/**
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ResultTag".
 */
export type ResultTag = 'result';
/**
 * Um recurso experimental. Em JSON e na CLI, pelo identificador (`fpga`,
 * `assistant`, `git`).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Feature".
 */
export type Feature = 'fpga' | 'assistant' | 'git';
/**
 * Um periférico da placa: o tipo e os sinais dele. A placa virtual simula
 * cada um nos pinos, como a placa de verdade os liga ao FPGA.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Peripheral".
 */
export type Peripheral =
  | {
      /**
       * O sinal de entrada.
       */
      signal: string;
      kind: 'switches';
    }
  | {
      /**
       * O sinal de entrada.
       */
      signal: string;
      kind: 'buttons';
    }
  | {
      /**
       * O sinal de saída.
       */
      signal: string;
      /**
       * A cor dos LEDs.
       */
      color: 'red' | 'green' | 'blue' | 'yellow';
      kind: 'leds';
    }
  | {
      /**
       * Os dígitos, da esquerda para a direita.
       */
      digits: string[];
      kind: 'seven_segment';
    }
  | {
      /**
       * O canal vermelho.
       */
      red: string;
      /**
       * O canal verde.
       */
      green: string;
      /**
       * O canal azul.
       */
      blue: string;
      /**
       * O sincronismo horizontal.
       */
      hsync: string;
      /**
       * O sincronismo vertical.
       */
      vsync: string;
      /**
       * O vídeo ativo (`VGA_BLANK_N`, `DE`), se a placa tem.
       */
      blank?: string | null;
      /**
       * O relógio de pixel que o projeto entrega ao conversor
       * (`VGA_CLK`), se a placa tem.
       */
      clock?: string | null;
      kind: 'vga';
    }
  | {
      /**
       * O relógio do teclado (entrada do FPGA).
       */
      clock: string;
      /**
       * O dado do teclado (entrada do FPGA).
       */
      data: string;
      kind: 'ps2';
    }
  | {
      /**
       * O que o projeto transmite (saída do FPGA).
       */
      tx: string;
      /**
       * O que o projeto recebe (entrada do FPGA), se a placa liga.
       */
      rx?: string | null;
      /**
       * A velocidade padrão, em bits por segundo.
       */
      baud: number;
      kind: 'uart';
    };
/**
 * O cabo de gravação embutido na placa.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Cable".
 */
export type Cable = 'usb-blaster' | 'usb-blasterII';
/**
 * A direção de um sinal da placa, do ponto de vista do FPGA.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SignalDirection".
 */
export type SignalDirection = 'input' | 'output';
/**
 * O padrão de I/O de um sinal.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "IoStandard".
 */
export type IoStandard = string | string[];
/**
 * A cor de um LED.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LedColor".
 */
export type LedColor = 'red' | 'green' | 'blue' | 'yellow';
/**
 * De onde vem um bit de entrada.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "BitSource".
 */
export type BitSource =
  | {
      /**
       * O sinal.
       */
      signal: string;
      /**
       * O bit.
       */
      bit: number;
      /**
       * Invertido (`!`).
       */
      invert: boolean;
      kind: 'pin';
    }
  | {
      /**
       * O valor.
       */
      value: boolean;
      kind: 'constant';
    };
/**
 * O que a correção percebeu sobre a causa do erro.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Finding".
 */
export type Finding =
  | {
      kind: 'interface_changed';
    }
  | {
      /**
       * A saída.
       */
      output: string;
      kind: 'undriven_output';
    }
  | {
      kind: 'reset_only';
    }
  | {
      /**
       * O sinal (`modulo.sinal` quando é de um submódulo).
       */
      signal: string;
      /**
       * A linha do `always` que gerou o latch, no arquivo do aluno.
       */
      line: number | null;
      kind: 'latch_inferred';
    };
/**
 * Como terminou a correção.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Verdict".
 */
export type Verdict =
  'solved' | 'compile_error' | 'mismatch' | 'incomplete' | 'timed_out' | 'cancelled';
/**
 * Para onde [`Project::reorder_file`] leva um arquivo na lista dele. Em
 * JSON: `{"kind": "first"}`, `{"kind": "before", "path": "rtl/b.v"}`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ListPosition".
 */
export type ListPosition =
  | {
      kind: 'first';
    }
  | {
      kind: 'last';
    }
  | {
      kind: 'before';
      path: string;
    }
  | {
      kind: 'after';
      path: string;
    };
/**
 * O que o cliente pede. Os alvos são sempre explícitos: a CLI resolve o
 * processador da pasta atual antes de montar o pedido.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FlowRequest".
 */
export type FlowRequest =
  | {
      /**
       * Os processadores; vazio é todos os do projeto.
       */
      processors?: string[];
      flow: 'build';
    }
  | {
      /**
       * Só os módulos deste arquivo como raiz.
       */
      file?: string | null;
      /**
       * Só este processador: o Verilog dele e o testbench que o YANC
       * gerou, no lugar do projeto.
       */
      processor?: string | null;
      /**
       * Acrescenta o `verilator --lint-only`.
       */
      lint?: boolean;
      flow: 'check';
    }
  | {
      /**
       * Só este processador.
       */
      processor?: string | null;
      flow: 'hierarchy';
    }
  | {
      /**
       * Simula este processador com o testbench do YANC; `None`, a
       * simulação do projeto.
       */
      processor?: string | null;
      /**
       * Troca o testbench da simulação do projeto antes de simular
       * (grava o `.spf`).
       */
      testbench?: string | null;
      /**
       * Icarus ou Verilator. Na simulação rápida, só vale para um
       * testbench cocotb.
       */
      simulator: 'icarus' | 'verilator';
      /**
       * A simulação rápida: sem onda, com o testbench Verilog no
       * Verilator.
       */
      fast?: boolean;
      /**
       * Prazo do passo `simulate`, em segundos.
       */
      timeout_s?: number | null;
      /**
       * Abrir a onda se a simulação a entregar, conforme
       * [`FlowOptions::wave`]. A simulação rápida não tem onda.
       */
      open_wave?: boolean;
      flow: 'simulate';
    }
  | {
      /**
       * Sintetiza só este processador; `None`, o topo do projeto.
       */
      processor?: string | null;
      /**
       * Desenha o esquemático (Yosys `show` e Graphviz) depois.
       */
      schematic?: boolean;
      /**
       * O módulo do esquemático; `None`, o topo da síntese.
       */
      module?: string | null;
      /**
       * Grava o HTML do RTL viewer ao lado do netlist e o abre conforme
       * [`FlowOptions::viewer_opener`].
       */
      open_viewer?: boolean;
      flow: 'synthesize';
    }
  | {
      /**
       * O `hierarchy.json` da síntese.
       */
      netlist: string;
      /**
       * O módulo.
       */
      module: string;
      /**
       * Escrever a largura dos barramentos nas arestas.
       */
      bus_widths?: boolean;
      flow: 'schematic';
    }
  | {
      flow: 'fpga_build';
    }
  | {
      /**
       * O cabo; `None`, o primeiro que o Quartus lista.
       */
      cable?: string | null;
      flow: 'fpga_program';
    }
  | {
      /**
       * A pasta de exercícios (ou uma pasta dentro dela).
       */
      root: string;
      /**
       * O exercício.
       */
      exercise: string;
      flow: 'learn';
    };
/**
 * Qual simulador usar. Em JSON: `"icarus"` ou `"verilator"`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Simulator".
 */
export type Simulator = 'icarus' | 'verilator';
/**
 * Como um fluxo abre a onda que a simulação entregou.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WavePolicy".
 */
export type WavePolicy = 'none' | 'window';
/**
 * Qual onda abrir.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WaveSource".
 */
export type WaveSource =
  | {
      /**
       * O arquivo.
       */
      path: string;
      kind: 'file';
    }
  | {
      /**
       * O processador; `None`, o projeto.
       */
      processor?: string | null;
      kind: 'project';
    };
/**
 * O que fazer com o layout dos processadores SAPHO ao abrir a onda.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LayoutPolicy".
 */
export type LayoutPolicy =
  | {
      kind: 'generated';
    }
  | {
      kind: 'none';
    }
  | {
      kind: 'reset';
    }
  | {
      /**
       * O arquivo (`.surf.ron` ou `.sucl`).
       */
      path: string;
      kind: 'file';
    };
/**
 * Uma mudança na escolha de sinais da onda do testbench do projeto
 * ([`choose_signals`]).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WaveChoice".
 */
export type WaveChoice =
  | {
      kind: 'all';
    }
  | {
      kind: 'add';
      paths: string[];
    }
  | {
      kind: 'remove';
      paths: string[];
    }
  | {
      kind: 'set';
      paths: string[];
    };
/**
 * Quais relatórios [`plan_cleanup`] escolhe para apagar. Em JSON:
 * `{"kind": "all"}`, `{"kind": "keep_latest", "value": 5}`,
 * `{"kind": "reports", "value": ["run-000042", "7"]}`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Cleanup".
 */
export type Cleanup =
  | {
      kind: 'all';
    }
  | {
      kind: 'keep_latest';
      value: number;
    }
  | {
      kind: 'reports';
      value: string[];
    };
/**
 * O valor de uma ligação no `fpga.json`: um sinal da placa (`"LEDR[7:0]"`,
 * `"!KEY[0]"`, `"0"`), ou uma lista deles para uma saída que vai a vários.
 * Só descreve o formato; o [`FpgaConfig`] guarda as ligações em [`Link`].
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LinkTargets".
 */
export type LinkTargets = string | string[];
/**
 * O id de um pedido JSON-RPC, como o cliente o mandou: número ou texto.
 * Só para o schema de `vboard.input`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "RequestId".
 */
export type RequestId = number | string;
/**
 * Uma entrada para a placa virtual em execução.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "BoardInput".
 */
export type BoardInput =
  | {
      /**
       * O periférico (`SW`, `KEY`).
       */
      device: string;
      /**
       * Os bits, do bit 0 em diante.
       */
      value: number;
      kind: 'set';
    }
  | {
      /**
       * O periférico PS/2.
       */
      device: string;
      /**
       * O código da tecla.
       */
      code: string;
      /**
       * Apertada (`true`) ou solta (`false`).
       */
      pressed: boolean;
      kind: 'key';
    }
  | {
      /**
       * O teclado PS/2 ou a UART.
       */
      device: string;
      /**
       * O texto.
       */
      text: string;
      kind: 'text';
    }
  | {
      /**
       * O teclado PS/2 ou a UART.
       */
      device: string;
      /**
       * Os bytes.
       */
      bytes: number[];
      kind: 'bytes';
    }
  | {
      kind: 'pause';
    }
  | {
      kind: 'resume';
    }
  | {
      /**
       * Quantos flancos (dois por ciclo de um relógio).
       */
      edges: number;
      kind: 'step';
    }
  | {
      /**
       * Segundos simulados por segundo de relógio; 0 sem limite.
       */
      limit: number;
      kind: 'speed';
    };
/**
 * Uma versão de um arquivo, para `git.read_file`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Revision".
 */
export type Revision =
  | {
      kind: 'index';
    }
  | {
      kind: 'head';
    }
  | {
      kind: 'commit';
      oid: string;
    };
/**
 * O que volta de um pedido que deu certo. Serializa como o valor em si (o
 * `result` do JSON-RPC), sem etiqueta: cada método diz qual é o dele.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Response".
 */
export type Response =
  | ServerInfo
  | ProjectStatus
  | AddedFile[]
  | RemoveOutcome
  | TopOutcome
  | ProjectFile[]
  | MovedPath1
  | LayoutConversion
  | NameCheck
  | Processor
  | ProcessorRemoval
  | number[]
  | NewProcessor1
  | string
  | WaveLayoutInfo
  | boolean[]
  | ModuleInterface
  | BoardLayout
  | BoardRun
  | FlowOutcome
  | WaveOpened1
  | WaveSignals
  | WaveChoiceOutcome
  | boolean
  | RunSummary[]
  | ReportView
  | RunComparison
  | string[]
  | CleanOutcome
  | ToolchainInfo
  | FileMismatch[]
  | ExperimentalInfo
  | Board[]
  | (FpgaConfig | null)
  | FpgaConfig1
  | Prepared
  | BitstreamState
  | null
  | UpdateReport
  | InstallReport
  | LearnCheckOutcome
  | LearnTracks
  | WorkspaceSnapshot
  | GitStatus
  | FileVersion
  | CommitResult
  | DiscardResult
  | InitResult
  | GitLog
  | CommitFiles
  | Branches
  | SyncResult
  | (Account | null)
  | DeviceCode
  | (
      | {
          /**
           * O intervalo, em segundos (o GitHub o aumenta se a pergunta vier
           * cedo demais).
           */
          interval: number;
          state: 'pending';
        }
      | {
          account: Account1;
          state: 'done';
        }
      | {
          state: 'expired';
        }
      | {
          state: 'denied';
        }
    );
/**
 * Direção de uma porta.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "PortDirection".
 */
export type PortDirection = 'input' | 'output' | 'inout';
/**
 * Como o bundle foi achado.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Origin".
 */
export type Origin = 'settings' | 'environment' | 'bundled' | 'installation' | 'path' | 'beside';
/**
 * Como `aurora update` atualizou.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "UpdateMethod".
 */
export type UpdateMethod = 'components' | 'installer';
/**
 * Uma operação pela metade no repositório.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Operation".
 */
export type Operation = 'merge' | 'rebase' | 'cherry_pick' | 'revert';
/**
 * O tipo de um [`ProjectIssue`]. Em JSON, `snake_case`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "IssueKind".
 */
export type IssueKind =
  | 'rescued_path'
  | 'selection_not_registered'
  | 'testbench_as_top'
  | 'invalid_processor_name'
  | 'duplicate_file'
  | 'nested_project'
  | 'legacy_layout';
/**
 * Por que a placa virtual terminou.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "EndReason".
 */
export type EndReason = 'stopped' | 'finished' | 'crashed';
/**
 * O formato do conteúdo de uma onda: FST no Icarus, VCD no Verilator. A
 * extensão é a do formato, menos quando o testbench nomeia a onda com uma
 * expressão que a AURORA não resolve: aí o Icarus grava VCD no nome que o
 * testbench der.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WaveformFormat".
 */
export type WaveformFormat = 'vcd' | 'fst';
/**
 * Como um teste terminou. Em JSON: `"passed"`, `"failed"` ou `"skipped"`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "TestStatus".
 */
export type TestStatus = 'passed' | 'failed' | 'skipped';
/**
 * O tipo de um escopo. Em JSON, `"module"`, `"generate"` ou `"block"`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ScopeKind".
 */
export type ScopeKind = 'module' | 'generate' | 'block';
/**
 * O tipo de um sinal. Em JSON, `"reg"`, `"wire"`, `"integer"` ou `"real"`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SignalKind".
 */
export type SignalKind = 'reg' | 'wire' | 'integer' | 'real';
/**
 * Quanto de uma parte um relatório tem.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Availability".
 */
export type Availability = 'available' | 'partial' | 'unavailable';
/**
 * Uma contagem do resumo do projeto.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SynthesisMetric".
 */
export type SynthesisMetric =
  | 'modules'
  | 'wires'
  | 'wire_bits'
  | 'public_wires'
  | 'public_wire_bits'
  | 'memories'
  | 'memory_bits'
  | 'processes'
  | 'cells';
/**
 * O que aconteceu com um número de um relatório para o outro.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Change".
 */
export type Change =
  'unchanged' | 'increased' | 'decreased' | 'added' | 'removed' | 'not_comparable';
/**
 * O que `aurora update` fez.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "UpdateAction".
 */
export type UpdateAction = 'checked' | 'up_to_date' | 'updated' | 'wizard_opened';
/**
 * O tipo de mudança de um arquivo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Change2".
 */
export type Change2 =
  | 'added'
  | 'untracked'
  | 'modified'
  | 'deleted'
  | 'renamed'
  | 'copied'
  | 'type_changed'
  | 'conflicted';
/**
 * A resposta de [`login_poll`].
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LoginPoll".
 */
export type LoginPoll =
  | {
      /**
       * O intervalo, em segundos (o GitHub o aumenta se a pergunta vier
       * cedo demais).
       */
      interval: number;
      state: 'pending';
    }
  | {
      account: Account1;
      state: 'done';
    }
  | {
      state: 'expired';
    }
  | {
      state: 'denied';
    };
/**
 * O que sai de uma operação enquanto ela roda, na ordem em que aconteceu.
 * Os nomes são os que a AURORA IDE manda à interface dele.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Notification".
 */
export type Notification =
  | {
      /**
       * Qual.
       */
      phase:
        | 'build'
        | 'check'
        | 'hierarchy'
        | 'simulate'
        | 'synthesize'
        | 'schematic'
        | 'rtl_viewer'
        | 'wave'
        | 'learn'
        | 'fpga'
        | 'program';
      type: 'phase';
    }
  | {
      /**
       * Os eventos (`step_started`, `output`, `step_finished`).
       */
      events: Event[];
      type: 'events';
    }
  | {
      /**
       * Um passo de uma operação. Em JSON, em `snake_case` (`"pre_assemble"`).
       */
      step:
        | 'preprocess'
        | 'compile'
        | 'pre_assemble'
        | 'assemble'
        | 'check_syntax'
        | 'lint'
        | 'elaborate'
        | 'verilate'
        | 'simulate'
        | 'synthesize'
        | 'graph'
        | 'render'
        | 'fit'
        | 'bitstream'
        | 'timing'
        | 'program';
      /**
       * A ferramenta.
       */
      tool:
        | 'cmmcomp'
        | 'appcomp'
        | 'asmcomp'
        | 'cpppp'
        | 'cppcomp'
        | 'iverilog'
        | 'vvp'
        | 'verilator'
        | 'yosys'
        | 'dot'
        | 'surfer'
        | 'openfpgaloader'
        | 'quartus'
        | 'perl';
      /**
       * Quantas.
       */
      lines: number;
      type: 'output_omitted';
    }
  | {
      result: BuildResult1;
      type: 'build';
    }
  | {
      layout: BoardLayout2;
      type: 'vboard_ready';
    }
  | {
      state: BoardState;
      type: 'vboard_state';
    }
  | {
      frame: Frame;
      type: 'vboard_frame';
    }
  | {
      /**
       * As linhas, na ordem.
       */
      lines: string[];
      /**
       * Quantas ficaram de fora deste lote, depois de `lines`.
       */
      omitted: number;
      type: 'vboard_output';
    }
  | {
      /**
       * O evento, em JSON.
       */
      event: {
        [k: string]: unknown | undefined;
      };
      /**
       * A linha do console para o evento, com os nomes dos componentes;
       * ausente no que é só barra de progresso.
       */
      line?: string | null;
      type: 'install';
    };
/**
 * A fase de um fluxo que começou. Os nomes são os que o Studio mostra;
 * `Learn` é a correção de um exercício do aurora-learn, que o Studio nomeia
 * com o mesmo tipo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Phase".
 */
export type Phase =
  | 'build'
  | 'check'
  | 'hierarchy'
  | 'simulate'
  | 'synthesize'
  | 'schematic'
  | 'rtl_viewer'
  | 'wave'
  | 'learn'
  | 'fpga'
  | 'program';
/**
 * Como um pedido terminou, do ponto de vista de quem o atende.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ShutdownReason".
 */
export type ShutdownReason = 'requested' | 'signal' | 'updated';
/**
 * Uma ferramenta em `aurora tools`: onde está, ou por que não está.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ToolEntry".
 */
export type ToolEntry =
  | {
      /**
       * O executável.
       */
      path: string;
      /**
       * Vem do sistema, não do bundle (só o Perl do Verilator, no Linux e
       * no macOS).
       */
      system: boolean;
    }
  | {
      error: ErrorInfo1;
    };
/**
 * O que um cliente pede. Em JSON, `{"method": "...", "params": {...}}`;
 * um método sem parâmetros vai sem `params`. Os caminhos são absolutos.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Request".
 */
export type Request =
  | {
      method: 'initialize';
      params: {
        /**
         * O cliente e a versão dele (`aurora-vscode 0.1.0`).
         */
        client?: string | null;
      };
    }
  | {
      method: 'shutdown';
    }
  | {
      method: 'project.create';
      params: {
        /**
         * A pasta de cima, que precisa existir.
         */
        parent: string;
        /**
         * O nome do projeto.
         */
        name: string;
      };
    }
  | {
      method: 'project.status';
      params: {
        /**
         * O `.spf` ou a pasta do projeto.
         */
        project: string;
      };
    }
  | {
      method: 'project.add';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * Os arquivos.
         */
        files: string[];
        /**
         * Registrar como testbench em vez de decidir pelo conteúdo.
         */
        testbench?: boolean;
      };
    }
  | {
      method: 'project.remove';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * Os arquivos.
         */
        files: string[];
      };
    }
  | {
      method: 'project.top';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * O arquivo ou o módulo.
         */
        target?: string | null;
      };
    }
  | {
      method: 'project.testbench';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * O testbench.
         */
        path: string;
      };
    }
  | {
      method: 'project.order';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * O arquivo.
         */
        file: string;
        /**
         * Para onde.
         */
        position:
          | {
              kind: 'first';
            }
          | {
              kind: 'last';
            }
          | {
              kind: 'before';
              path: string;
            }
          | {
              kind: 'after';
              path: string;
            };
      };
    }
  | {
      method: 'project.move';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * De onde.
         */
        from: string;
        /**
         * Para onde (o caminho final).
         */
        to: string;
        /**
         * O movimento já aconteceu fora da AURORA.
         */
        after?: boolean;
        /**
         * Se o destino existe, ele vai para a lixeira do sistema e o
         * movimento acontece; sem isso, `path_exists`. O que a AURORA mantém
         * no lugar, a pasta do projeto e uma pasta que contém a origem não
         * são substituídos (`cannot_replace`). Não vale com `after`.
         */
        overwrite?: boolean;
      };
    }
  | {
      method: 'project.convert';
      params: {
        /**
         * O projeto.
         */
        project: string;
      };
    }
  | {
      method: 'project.check_name';
      params: {
        /**
         * O nome.
         */
        name: string;
      };
    }
  | {
      method: 'processor.check_name';
      params: {
        /**
         * O nome.
         */
        name: string;
      };
    }
  | {
      method: 'processor.add';
      params: {
        /**
         * O projeto.
         */
        project: string;
        spec: NewProcessor2;
      };
    }
  | {
      method: 'processor.set';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * O processador.
         */
        name: string;
        config: ProcessorConfig1;
      };
    }
  | {
      method: 'processor.remove';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * O processador.
         */
        name: string;
        /**
         * Mandar a pasta dele para a lixeira do sistema.
         */
        to_trash?: boolean;
      };
    }
  | {
      method: 'processor.outputs';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * O processador.
         */
        processor: string;
        /**
         * A porta.
         */
        port: number;
      };
    }
  | {
      method: 'processor.defaults';
    }
  | {
      method: 'flow.run';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * O fluxo.
         */
        request:
          | {
              /**
               * Os processadores; vazio é todos os do projeto.
               */
              processors?: string[];
              flow: 'build';
            }
          | {
              /**
               * Só os módulos deste arquivo como raiz.
               */
              file?: string | null;
              /**
               * Só este processador: o Verilog dele e o testbench que o YANC
               * gerou, no lugar do projeto.
               */
              processor?: string | null;
              /**
               * Acrescenta o `verilator --lint-only`.
               */
              lint?: boolean;
              flow: 'check';
            }
          | {
              /**
               * Só este processador.
               */
              processor?: string | null;
              flow: 'hierarchy';
            }
          | {
              /**
               * Simula este processador com o testbench do YANC; `None`, a
               * simulação do projeto.
               */
              processor?: string | null;
              /**
               * Troca o testbench da simulação do projeto antes de simular
               * (grava o `.spf`).
               */
              testbench?: string | null;
              /**
               * Icarus ou Verilator. Na simulação rápida, só vale para um
               * testbench cocotb.
               */
              simulator: 'icarus' | 'verilator';
              /**
               * A simulação rápida: sem onda, com o testbench Verilog no
               * Verilator.
               */
              fast?: boolean;
              /**
               * Prazo do passo `simulate`, em segundos.
               */
              timeout_s?: number | null;
              /**
               * Abrir a onda se a simulação a entregar, conforme
               * [`FlowOptions::wave`]. A simulação rápida não tem onda.
               */
              open_wave?: boolean;
              flow: 'simulate';
            }
          | {
              /**
               * Sintetiza só este processador; `None`, o topo do projeto.
               */
              processor?: string | null;
              /**
               * Desenha o esquemático (Yosys `show` e Graphviz) depois.
               */
              schematic?: boolean;
              /**
               * O módulo do esquemático; `None`, o topo da síntese.
               */
              module?: string | null;
              /**
               * Grava o HTML do RTL viewer ao lado do netlist e o abre conforme
               * [`FlowOptions::viewer_opener`].
               */
              open_viewer?: boolean;
              flow: 'synthesize';
            }
          | {
              /**
               * O `hierarchy.json` da síntese.
               */
              netlist: string;
              /**
               * O módulo.
               */
              module: string;
              /**
               * Escrever a largura dos barramentos nas arestas.
               */
              bus_widths?: boolean;
              flow: 'schematic';
            }
          | {
              flow: 'fpga_build';
            }
          | {
              /**
               * O cabo; `None`, o primeiro que o Quartus lista.
               */
              cable?: string | null;
              flow: 'fpga_program';
            }
          | {
              /**
               * A pasta de exercícios (ou uma pasta dentro dela).
               */
              root: string;
              /**
               * O exercício.
               */
              exercise: string;
              flow: 'learn';
            };
        /**
         * As opções da interface; sem elas, o comando é o da CLI
         * equivalente, a onda abre em janela e o RTL viewer no navegador.
         */
        options?: FlowOptions | null;
        /**
         * Receber a saída das ferramentas ao vivo (`events`). Sem isso, o
         * `vvp` roda com buffer, que é mais rápido.
         */
        output?: boolean;
        /**
         * Quantos bytes do `stdout` e do `stderr` de cada passo voltam no
         * resultado, do fim; sem isso, tudo. Com `output`, cada linha já
         * foi pelas notificações `events`.
         */
        step_output_limit?: number | null;
      };
    }
  | {
      method: 'flow.command';
      params: {
        /**
         * O fluxo.
         */
        request:
          | {
              /**
               * Os processadores; vazio é todos os do projeto.
               */
              processors?: string[];
              flow: 'build';
            }
          | {
              /**
               * Só os módulos deste arquivo como raiz.
               */
              file?: string | null;
              /**
               * Só este processador: o Verilog dele e o testbench que o YANC
               * gerou, no lugar do projeto.
               */
              processor?: string | null;
              /**
               * Acrescenta o `verilator --lint-only`.
               */
              lint?: boolean;
              flow: 'check';
            }
          | {
              /**
               * Só este processador.
               */
              processor?: string | null;
              flow: 'hierarchy';
            }
          | {
              /**
               * Simula este processador com o testbench do YANC; `None`, a
               * simulação do projeto.
               */
              processor?: string | null;
              /**
               * Troca o testbench da simulação do projeto antes de simular
               * (grava o `.spf`).
               */
              testbench?: string | null;
              /**
               * Icarus ou Verilator. Na simulação rápida, só vale para um
               * testbench cocotb.
               */
              simulator: 'icarus' | 'verilator';
              /**
               * A simulação rápida: sem onda, com o testbench Verilog no
               * Verilator.
               */
              fast?: boolean;
              /**
               * Prazo do passo `simulate`, em segundos.
               */
              timeout_s?: number | null;
              /**
               * Abrir a onda se a simulação a entregar, conforme
               * [`FlowOptions::wave`]. A simulação rápida não tem onda.
               */
              open_wave?: boolean;
              flow: 'simulate';
            }
          | {
              /**
               * Sintetiza só este processador; `None`, o topo do projeto.
               */
              processor?: string | null;
              /**
               * Desenha o esquemático (Yosys `show` e Graphviz) depois.
               */
              schematic?: boolean;
              /**
               * O módulo do esquemático; `None`, o topo da síntese.
               */
              module?: string | null;
              /**
               * Grava o HTML do RTL viewer ao lado do netlist e o abre conforme
               * [`FlowOptions::viewer_opener`].
               */
              open_viewer?: boolean;
              flow: 'synthesize';
            }
          | {
              /**
               * O `hierarchy.json` da síntese.
               */
              netlist: string;
              /**
               * O módulo.
               */
              module: string;
              /**
               * Escrever a largura dos barramentos nas arestas.
               */
              bus_widths?: boolean;
              flow: 'schematic';
            }
          | {
              flow: 'fpga_build';
            }
          | {
              /**
               * O cabo; `None`, o primeiro que o Quartus lista.
               */
              cable?: string | null;
              flow: 'fpga_program';
            }
          | {
              /**
               * A pasta de exercícios (ou uma pasta dentro dela).
               */
              root: string;
              /**
               * O exercício.
               */
              exercise: string;
              flow: 'learn';
            };
        /**
         * O programa que começa o comando; sem ele, `aurora`.
         */
        program?: string | null;
      };
    }
  | {
      method: 'wave.open';
      params: {
        /**
         * O projeto, para uma onda do projeto; `None` para um arquivo.
         */
        project?: string | null;
        request: WaveRequest1;
      };
    }
  | {
      method: 'wave.signals';
      params: {
        /**
         * O projeto.
         */
        project: string;
      };
    }
  | {
      method: 'wave.choose';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * A mudança.
         */
        choice:
          | {
              kind: 'all';
            }
          | {
              kind: 'add';
              paths: string[];
            }
          | {
              kind: 'remove';
              paths: string[];
            }
          | {
              kind: 'set';
              paths: string[];
            };
      };
    }
  | {
      method: 'wave.reset_layout';
      params: {
        /**
         * A onda.
         */
        waveform: string;
      };
    }
  | {
      method: 'wave.layout';
      params: {
        /**
         * A onda.
         */
        waveform: string;
      };
    }
  | {
      method: 'verilog.modules';
      params: {
        /**
         * O texto.
         */
        text: string;
      };
    }
  | {
      method: 'verilog.testbench_names';
      params: {
        /**
         * Os nomes, sem pasta.
         */
        names: string[];
      };
    }
  | {
      method: 'history.list';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * Quantos, no máximo.
         */
        limit?: number | null;
      };
    }
  | {
      method: 'history.show';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * `run-000042` ou `42`.
         */
        id?: string | null;
      };
    }
  | {
      method: 'history.compare';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * O atual (o mais novo sem ele).
         */
        id?: string | null;
        /**
         * A referência (o anterior comparável sem ela).
         */
        against?: string | null;
      };
    }
  | {
      method: 'history.plan_cleanup';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * A regra.
         */
        cleanup:
          | {
              kind: 'all';
            }
          | {
              kind: 'keep_latest';
              value: number;
            }
          | {
              kind: 'reports';
              value: string[];
            };
      };
    }
  | {
      method: 'history.clean';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * Os identificadores (do plano).
         */
        ids: string[];
      };
    }
  | {
      method: 'toolchain.info';
    }
  | {
      method: 'toolchain.verify';
    }
  | {
      method: 'experimental.list';
    }
  | {
      method: 'experimental.set';
      params: {
        /**
         * O recurso (`fpga`).
         */
        feature: string;
        /**
         * Ligado.
         */
        enabled: boolean;
      };
    }
  | {
      method: 'fpga.boards';
      params: {
        /**
         * Uma placa; `None`, todas.
         */
        board?: string | null;
      };
    }
  | {
      method: 'fpga.config';
      params: {
        /**
         * O projeto.
         */
        project: string;
      };
    }
  | {
      method: 'fpga.config_set';
      params: {
        /**
         * O projeto.
         */
        project: string;
        config: FpgaConfig2;
      };
    }
  | {
      method: 'fpga.connect';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * A placa, num projeto sem `fpga.json`.
         */
        board?: string | null;
        change: Reconnect1;
      };
    }
  | {
      method: 'fpga.check';
      params: {
        /**
         * O projeto.
         */
        project: string;
      };
    }
  | {
      method: 'fpga.modules';
      params: {
        /**
         * O projeto.
         */
        project: string;
      };
    }
  | {
      method: 'fpga.status';
      params: {
        /**
         * O projeto.
         */
        project: string;
      };
    }
  | {
      method: 'fpga.cables';
    }
  | {
      method: 'fpga.top';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * O módulo; `None`, o `top` do `fpga.json`, senão o do projeto.
         */
        top?: string | null;
      };
    }
  | {
      method: 'vboard.layout';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * A placa, num projeto sem `fpga.json`: as portas com o nome de um
         * sinal dela ligam a ele.
         */
        board?: string | null;
      };
    }
  | {
      method: 'vboard.run';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * A placa, num projeto sem `fpga.json` (como em `vboard.layout`).
         */
        board?: string | null;
        /**
         * Segundos simulados por segundo de relógio, no máximo: 1 é o
         * tempo real (o padrão), 0 é o mais rápido possível.
         */
        limit?: number | null;
        /**
         * Quadros de vídeo por segundo, no máximo (padrão 30).
         */
        fps?: number | null;
      };
    }
  | {
      method: 'vboard.input';
      params: {
        /**
         * O id do pedido `vboard.run`, como o cliente o mandou.
         */
        run: number | string;
        /**
         * A entrada.
         */
        input:
          | {
              /**
               * O periférico (`SW`, `KEY`).
               */
              device: string;
              /**
               * Os bits, do bit 0 em diante.
               */
              value: number;
              kind: 'set';
            }
          | {
              /**
               * O periférico PS/2.
               */
              device: string;
              /**
               * O código da tecla.
               */
              code: string;
              /**
               * Apertada (`true`) ou solta (`false`).
               */
              pressed: boolean;
              kind: 'key';
            }
          | {
              /**
               * O teclado PS/2 ou a UART.
               */
              device: string;
              /**
               * O texto.
               */
              text: string;
              kind: 'text';
            }
          | {
              /**
               * O teclado PS/2 ou a UART.
               */
              device: string;
              /**
               * Os bytes.
               */
              bytes: number[];
              kind: 'bytes';
            }
          | {
              kind: 'pause';
            }
          | {
              kind: 'resume';
            }
          | {
              /**
               * Quantos flancos (dois por ciclo de um relógio).
               */
              edges: number;
              kind: 'step';
            }
          | {
              /**
               * Segundos simulados por segundo de relógio; 0 sem limite.
               */
              limit: number;
              kind: 'speed';
            };
      };
    }
  | {
      method: 'update.check';
    }
  | {
      method: 'update.apply';
      params: {
        /**
         * O instalador inteiro, sem tentar por componentes.
         */
        full?: boolean;
        /**
         * Um instalador no disco, sem rede.
         */
        from?: string | null;
        /**
         * No Windows, quando o assistente de instalação roda: abrir a AURORA
         * Studio quando ele terminar (o `--reopen-ide` da CLI). O Studio
         * pede isso e fecha, para o assistente trocar os arquivos dele.
         */
        reopen_ide?: boolean;
      };
    }
  | {
      method: 'install.add';
      params: {
        /**
         * Os componentes (pelo menos um).
         */
        components: string[];
      };
    }
  | {
      method: 'learn.check';
      params: {
        /**
         * A pasta de exercícios, ou uma pasta dentro dela.
         */
        root: string;
        /**
         * O exercício; `None`, o atual.
         */
        exercise?: string | null;
        /**
         * Todos os exercícios.
         */
        all?: boolean;
      };
    }
  | {
      method: 'learn.tracks';
    }
  | {
      method: 'learn.status';
      params: {
        /**
         * A pasta de exercícios, ou uma pasta dentro dela.
         */
        root: string;
        /**
         * O idioma dos enunciados, quando a trilha tem tradução.
         */
        lang?: string | null;
      };
    }
  | {
      method: 'learn.init';
      params: {
        /**
         * A pasta, que pode existir vazia.
         */
        dir: string;
        /**
         * A trilha; `None`, a padrão.
         */
        track?: string | null;
        /**
         * O idioma dos enunciados.
         */
        lang?: string | null;
      };
    }
  | {
      method: 'learn.set_current';
      params: {
        /**
         * A pasta de exercícios.
         */
        root: string;
        /**
         * O exercício.
         */
        name: string;
        /**
         * O idioma dos enunciados.
         */
        lang?: string | null;
      };
    }
  | {
      method: 'learn.reset';
      params: {
        /**
         * A pasta de exercícios.
         */
        root: string;
        /**
         * O exercício.
         */
        name: string;
      };
    }
  | {
      method: 'git.status';
      params: {
        /**
         * O projeto.
         */
        project: string;
      };
    }
  | {
      method: 'git.diff';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * Um arquivo, relativo ao topo do repositório, com `/`; `None`,
         * todos.
         */
        path?: string | null;
      };
    }
  | {
      method: 'git.read_file';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * A versão.
         */
        revision:
          | {
              kind: 'index';
            }
          | {
              kind: 'head';
            }
          | {
              kind: 'commit';
              oid: string;
            };
        /**
         * O arquivo, relativo ao topo do repositório, com `/`.
         */
        path: string;
      };
    }
  | {
      method: 'git.commit';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * A primeira linha da mensagem.
         */
        summary: string;
        /**
         * O resto da mensagem.
         */
        description?: string | null;
        /**
         * Os arquivos, relativos ao topo, com `/`; `None`, todos os mudados.
         */
        paths?: string[] | null;
        /**
         * Refaz o último commit.
         */
        amend?: boolean;
      };
    }
  | {
      method: 'git.discard';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * Os arquivos, relativos ao topo, com `/`.
         */
        paths: string[];
      };
    }
  | {
      method: 'git.init';
      params: {
        /**
         * O projeto.
         */
        project: string;
      };
    }
  | {
      method: 'git.log';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * Quantos commits pular, dos mais novos.
         */
        skip?: number | null;
        /**
         * Quantos devolver (até 500; 50 sem o campo).
         */
        limit?: number | null;
      };
    }
  | {
      method: 'git.commit_files';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * O identificador do commit.
         */
        oid: string;
      };
    }
  | {
      method: 'git.branches';
      params: {
        /**
         * O projeto.
         */
        project: string;
      };
    }
  | {
      method: 'git.switch';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * Um ramo local, um remoto (`origin/x`) ou, com `create`, o novo.
         */
        branch: string;
        /**
         * Cria o ramo a partir do commit atual.
         */
        create?: boolean;
      };
    }
  | {
      method: 'git.fetch';
      params: {
        /**
         * O projeto.
         */
        project: string;
      };
    }
  | {
      method: 'git.pull';
      params: {
        /**
         * O projeto.
         */
        project: string;
      };
    }
  | {
      method: 'git.push';
      params: {
        /**
         * O projeto.
         */
        project: string;
      };
    }
  | {
      method: 'git.set_identity';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * O nome.
         */
        name: string;
        /**
         * O e-mail.
         */
        email: string;
        /**
         * Na configuração global do git, para todos os repositórios desta
         * máquina; sem o campo, só neste.
         */
        global?: boolean;
      };
    }
  | {
      method: 'git.set_remote';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * O nome (`origin`).
         */
        name: string;
        /**
         * O endereço.
         */
        url: string;
      };
    }
  | {
      method: 'git.remove_remote';
      params: {
        /**
         * O projeto.
         */
        project: string;
        /**
         * O nome.
         */
        name: string;
      };
    }
  | {
      method: 'github.account';
    }
  | {
      method: 'github.login_start';
    }
  | {
      method: 'github.login_poll';
      params: {
        /**
         * O `device_code` de `github.login_start`.
         */
        device_code: string;
      };
    }
  | {
      method: 'github.logout';
    };

/**
 * O que [`Project::add_verilog`] fez com um arquivo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "AddedFile".
 */
export interface AddedFile {
  /**
   * Caminho absoluto.
   */
  path: string;
  /**
   * Em qual lista ficou.
   */
  role: 'synthesizable' | 'testbench';
  /**
   * O arquivo não existia e foi criado a partir do modelo.
   */
  created: boolean;
  /**
   * Virou o módulo de topo (era o primeiro sintetizável) ou o testbench
   * escolhido.
   */
  selected: boolean;
}
/**
 * O resultado de [`build`]. Ver o módulo `pipeline` para os campos comuns a
 * toda operação.
 *
 * Em JSON (resumido):
 *
 * ```json
 * { "processor": "soma", "language": "cmm", "status": "succeeded",
 *   "failed_step": null, "frequency_mhz": 100, "clocks": 2000,
 *   "steps": [ { "step": "compile", "tool": "cmmcomp", "...": "..." } ],
 *   "diagnostics": [],
 *   "artifacts": [ { "kind": "verilog", "path": "/p/soma/soma/soma.v",
 *                    "required": true, "fresh": true } ] }
 * ```
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "BuildResult".
 */
export interface BuildResult {
  /**
   * O processador compilado.
   */
  processor: string;
  /**
   * A linguagem do fonte, que decidiu o pipeline.
   */
  language: 'cmm' | 'cpp';
  /**
   * Como o build terminou.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * O passo que falhou ou quebrou, quando for o caso.
   */
  failed_step: Step | null;
  /**
   * A frequência usada (do `.spf` ou de [`BuildOptions`]).
   */
  frequency_mhz: number;
  /**
   * Os clocks usados (do `.spf` ou de [`BuildOptions`]).
   */
  clocks: number;
  /**
   * Um relatório por compilador executado: `compile` e `assemble`
   * ([`build_steps`]).
   */
  steps: StepReport[];
  /**
   * Todas as mensagens de todos os passos, na ordem em que saíram. Hoje
   * os compiladores param no primeiro erro, mas a API já é uma lista.
   */
  diagnostics: Diagnostic[];
  /**
   * Obrigatórios, na pasta de saída: `<nome>.asm`, `<nome>.v`,
   * `<nome>_data.mif`, `<nome>_inst.mif` e o testbench `<nome>_tb.v`.
   * Intermediários (`pc_<nome>_mem.txt`, `src_lines.txt`,
   * `trad_opcode.txt`, `prog_log.txt`, `app_log.txt`), quando existem.
   */
  artifacts: Artifact[];
  /**
   * Quanto o build levou, do começo ao fim, em milissegundos.
   */
  duration_ms: number;
}
/**
 * O que aconteceu num passo: o comando exato, como terminou e tudo o que
 * escreveu. É o registro de auditoria da operação; os `diagnostics` do
 * resultado são a versão interpretada de `stdout` e `stderr`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "StepReport".
 */
export interface StepReport {
  /**
   * Um passo de uma operação. Em JSON, em `snake_case` (`"pre_assemble"`).
   */
  step:
    | 'preprocess'
    | 'compile'
    | 'pre_assemble'
    | 'assemble'
    | 'check_syntax'
    | 'lint'
    | 'elaborate'
    | 'verilate'
    | 'simulate'
    | 'synthesize'
    | 'graph'
    | 'render'
    | 'fit'
    | 'bitstream'
    | 'timing'
    | 'program';
  /**
   * A ferramenta executada. Para o executável que o Verilator gera, é
   * [`Tool::Verilator`].
   */
  tool:
    | 'cmmcomp'
    | 'appcomp'
    | 'asmcomp'
    | 'cpppp'
    | 'cppcomp'
    | 'iverilog'
    | 'vvp'
    | 'verilator'
    | 'yosys'
    | 'dot'
    | 'surfer'
    | 'openfpgaloader'
    | 'quartus'
    | 'perl';
  command: Invocation;
  /**
   * Como o processo terminou.
   */
  termination:
    | {
        kind: 'exited';
        value: number;
      }
    | {
        kind: 'signaled';
        value: number;
      }
    | {
        kind: 'exception';
        value: number;
      }
    | {
        kind: 'cancelled';
      }
    | {
        kind: 'timed_out';
      }
    | {
        kind: 'unknown';
      };
  /**
   * Tudo que o processo escreveu no stdout, decodificado como UTF-8 (bytes
   * inválidos viram `U+FFFD`). Numa simulação, inclui o `$display` do
   * testbench.
   */
  stdout: string;
  /**
   * Tudo que o processo escreveu no stderr.
   */
  stderr: string;
  /**
   * Duração, do início do processo até o fim, em milissegundos.
   */
  duration_ms: number;
}
/**
 * O que foi executado, com CWD e ambiente.
 */
export interface Invocation {
  /**
   * Caminho absoluto do executável. Nunca é procurado no `PATH`.
   */
  program: string;
  /**
   * Argumentos, na ordem. Caminhos já estão no formato que a ferramenta
   * aceita: o nativo do sistema, menos os do `iverilog` no Windows, que
   * vão com `/`.
   */
  args: string[];
  /**
   * Diretório de trabalho. Faz parte do contrato de várias ferramentas (o
   * testbench do YANC lê e grava os arquivos dele relativo a ele, e grava
   * a onda nele), por isso nunca é herdado.
   */
  cwd: string;
  /**
   * Variáveis definidas pela AURORA, somadas às de `INHERITED_ENV`.
   */
  env: [unknown, unknown][];
  /**
   * Nomes de variáveis copiadas do ambiente da AURORA, quando existirem.
   * Só os nomes aparecem no relatório.
   */
  inherit: string[];
}
/**
 * Uma mensagem de uma ferramenta, com os campos que deu para extrair.
 *
 * `file`, `line` e `column` são opcionais porque o `cmmcomp` não informa
 * arquivo nem coluna, e várias mensagens não informam nem a linha. Quando a
 * ferramenta não cita o arquivo mas a AURORA sabe qual é (o `cmmcomp` compila
 * um fonte só), `file` vem preenchido pela AURORA.
 *
 * Em JSON:
 *
 * ```json
 * { "tool": "cmmcomp", "severity": "error",
 *   "message": "c'mon dude, declare the variable 'total' properly!",
 *   "file": "/p/conta/conta.cmm", "line": 15, "column": null,
 *   "raw": "Error on line 15: c'mon dude, declare the variable 'total' properly!" }
 * ```
 *
 * No fluxo C, o `cppcomp` cita o arquivo e a linha do fonte, ou do
 * cabeçalho incluído onde está o erro.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Diagnostic".
 */
export interface Diagnostic {
  /**
   * A ferramenta que escreveu a mensagem.
   */
  tool:
    | 'cmmcomp'
    | 'appcomp'
    | 'asmcomp'
    | 'cpppp'
    | 'cppcomp'
    | 'iverilog'
    | 'vvp'
    | 'verilator'
    | 'yosys'
    | 'dot'
    | 'surfer'
    | 'openfpgaloader'
    | 'quartus'
    | 'perl';
  /**
   * A gravidade.
   */
  severity: 'error' | 'warning' | 'info' | 'unknown';
  /**
   * A mensagem, sem o prefixo de gravidade e de localização. Vem no idioma
   * da ferramenta (inglês: a AURORA sempre pede inglês ao YANC).
   */
  message: string;
  /**
   * O arquivo a que a mensagem se refere, como a ferramenta o escreveu
   * (normalmente absoluto, porque a AURORA passa caminhos absolutos).
   */
  file: string | null;
  /**
   * Linha, a partir de 1.
   */
  line: number | null;
  /**
   * Coluna, a partir de 1 (Verilator e compilador C++).
   */
  column: number | null;
  /**
   * O texto exatamente como a ferramenta o escreveu (várias linhas quando
   * a ferramenta continua a mensagem em linhas indentadas).
   */
  raw: string;
}
/**
 * Um arquivo que a operação deveria produzir.
 *
 * `fresh` é calculado comparando o horário de modificação antes e depois da
 * operação. Um `.v` que sobrou de um build anterior aparece com
 * `fresh = false`, e não como sucesso. Artefatos obrigatórios aparecem
 * sempre; intermediários, só se existirem.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Artifact".
 */
export interface Artifact {
  /**
   * O papel do arquivo.
   */
  kind:
    | 'assembly'
    | 'verilog'
    | 'data_memory'
    | 'instruction_memory'
    | 'testbench'
    | 'preprocessed_source'
    | 'program_counter_map'
    | 'source_translation'
    | 'opcode_translation'
    | 'compiler_log'
    | 'pre_assembler_log'
    | 'icarus_image'
    | 'verilated_model'
    | 'waveform'
    | 'simulation_output'
    | 'netlist'
    | 'schematic_graph'
    | 'schematic'
    | 'synthesis_statistics'
    | 'board_top'
    | 'quartus_project'
    | 'sram_object'
    | 'raw_binary'
    | 'serial_vector_format';
  /**
   * Caminho absoluto.
   */
  path: string;
  /**
   * Sem ele a operação não está completa.
   */
  required: boolean;
  /**
   * Existe e foi escrito por esta operação (não é sobra de uma anterior).
   * O executável do Verilator, que o `make` só refaz quando o modelo
   * muda, conta como escrito quando a compilação terminou bem: ele está
   * em dia com a entrada.
   */
  fresh: boolean;
}
/**
 * Uma execução de programa, totalmente especificada: o registro exato do
 * que a AURORA rodou, presente em todo [`StepReport`](crate::StepReport).
 *
 * Só o Core monta invocações; para os clientes o tipo é somente leitura.
 * Reproduzir um passo à mão é rodar `program` com `args`, dentro de `cwd`,
 * com um ambiente vazio mais `env` e as variáveis de `inherit`.
 *
 * Em JSON:
 *
 * ```json
 * { "program": "/opt/yanc/bin/cmmcomp",
 *   "args": ["-i", "soma.cmm", "-n", "soma", "-p", "/p/soma", "..."],
 *   "cwd": "/p/soma",
 *   "env": [],
 *   "inherit": [] }
 * ```
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Invocation".
 */
export interface Invocation1 {
  /**
   * Caminho absoluto do executável. Nunca é procurado no `PATH`.
   */
  program: string;
  /**
   * Argumentos, na ordem. Caminhos já estão no formato que a ferramenta
   * aceita: o nativo do sistema, menos os do `iverilog` no Windows, que
   * vão com `/`.
   */
  args: string[];
  /**
   * Diretório de trabalho. Faz parte do contrato de várias ferramentas (o
   * testbench do YANC lê e grava os arquivos dele relativo a ele, e grava
   * a onda nele), por isso nunca é herdado.
   */
  cwd: string;
  /**
   * Variáveis definidas pela AURORA, somadas às de `INHERITED_ENV`.
   */
  env: [unknown, unknown][];
  /**
   * Nomes de variáveis copiadas do ambiente da AURORA, quando existirem.
   * Só os nomes aparecem no relatório.
   */
  inherit: string[];
}
/**
 * O resultado de [`check`]. Não há artefatos: a verificação só diz se o
 * Verilog elabora, e o que os compiladores acharam.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "CheckResult".
 */
export interface CheckResult {
  /**
   * Os módulos elaborados como raiz: os do projeto (ou os do arquivo
   * pedido) e cada testbench.
   */
  targets: string[];
  /**
   * `Succeeded` se tudo elabora sem erro.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * O primeiro passo que falhou.
   */
  failed_step: Step | null;
  /**
   * Um `iverilog -t null` para o projeto, um por testbench e, com
   * `lint`, o `verilator --lint-only`.
   */
  steps: StepReport[];
  /**
   * Os erros e avisos, com arquivo e linha.
   */
  diagnostics: Diagnostic[];
  /**
   * Quanto a verificação levou, do começo ao fim, em milissegundos.
   */
  duration_ms: number;
}
/**
 * Um arquivo ou pasta que a conversão moveu.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ConvertedPath".
 */
export interface ConvertedPath {
  /**
   * De que processador.
   */
  processor: string;
  /**
   * Onde estava, absoluto.
   */
  from: string;
  /**
   * Onde ficou, absoluto.
   */
  to: string;
  /**
   * É algo que o YANC gera (foi para a pasta de saída).
   */
  generated: boolean;
}
/**
 * Um arquivo ou pasta que a conversão deixou onde estava.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SkippedPath".
 */
export interface SkippedPath {
  /**
   * De que processador.
   */
  processor: string;
  /**
   * Onde está, absoluto.
   */
  from: string;
  /**
   * Para onde iria.
   */
  to: string;
  /**
   * Por quê: o destino já existe, ou o erro do sistema.
   */
  reason: string;
}
/**
 * Um fonte que perdeu a linha do nome.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "RemovedNameLine".
 */
export interface RemovedNameLine {
  /**
   * De que processador.
   */
  processor: string;
  /**
   * O fonte, já no lugar novo.
   */
  file: string;
  /**
   * O nome que a linha dava, que era o do módulo Verilog até a 6.x. Se
   * difere do processador, o topo que instanciava o módulo pelo nome
   * antigo precisa do novo.
   */
  old_name: string;
}
/**
 * Um [`AuroraError`] pronto para atravessar uma fronteira: o JSON da CLI, a
 * mensagem do Studio para a interface, a resposta do `aurora serve`. É o que
 * o código, o texto e os campos da variante viram quando o enum não vai.
 *
 * ```json
 * { "code": "processor_not_found",
 *   "message": "No processor 'x' in the project (available: soma, filtro)",
 *   "data": { "name": "x", "available": ["soma", "filtro"] } }
 * ```
 *
 * A dica de cada interface (o comando `aurora` que resolve, o botão do
 * Studio) não mora aqui: a mensagem do Core é neutra, e quem mostra decide
 * pelo `code`. Um erro que não é do Core (o `busy` do Studio, o `cli` da
 * CLI) usa o mesmo tipo com [`ErrorInfo::new`] e um código próprio.
 *
 * ```
 * use aurora_core::{ErrorInfo, AuroraError};
 *
 * let info = ErrorInfo::from(AuroraError::ProcessorExists("soma".into()));
 * assert_eq!(info.code, "processor_exists");
 * assert_eq!(info.data.unwrap()["name"], "soma");
 * ```
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ErrorInfo".
 */
export interface ErrorInfo {
  /**
   * O identificador estável: [`AuroraError::code`], ou um código de quem
   * chamou (`busy`, `cli`), sempre em `snake_case`.
   */
  code: string;
  /**
   * O texto do erro, em inglês, pronto para mostrar.
   */
  message: string;
  /**
   * O que fazer, quando quem mostra sabe: o comando `aurora` que resolve,
   * na CLI. O core não o preenche; a mensagem dele é neutra. Omitido no
   * JSON quando não há.
   */
  hint?: string | null;
  /**
   * Os campos estruturados da variante, quando ela os tem
   * ([`AuroraError::data`]): `available`, `files`, `name`, `value`,
   * `reason`, `path`, `component`. Omitido no JSON quando não há.
   */
  data?: {
    [k: string]: unknown | undefined;
  };
}
/**
 * O que vai ser executado.
 */
export interface Invocation2 {
  /**
   * Caminho absoluto do executável. Nunca é procurado no `PATH`.
   */
  program: string;
  /**
   * Argumentos, na ordem. Caminhos já estão no formato que a ferramenta
   * aceita: o nativo do sistema, menos os do `iverilog` no Windows, que
   * vão com `/`.
   */
  args: string[];
  /**
   * Diretório de trabalho. Faz parte do contrato de várias ferramentas (o
   * testbench do YANC lê e grava os arquivos dele relativo a ele, e grava
   * a onda nele), por isso nunca é herdado.
   */
  cwd: string;
  /**
   * Variáveis definidas pela AURORA, somadas às de `INHERITED_ENV`.
   */
  env: [unknown, unknown][];
  /**
   * Nomes de variáveis copiadas do ambiente da AURORA, quando existirem.
   * Só os nomes aparecem no relatório.
   */
  inherit: string[];
}
/**
 * A última linha do `--events`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ResultLine".
 */
export interface ResultLine {
  /**
   * Sempre `"result"`.
   */
  event: 'result';
  /**
   * O mesmo objeto que o comando escreve com `--json` (veja o schema do
   * comando).
   */
  result: {
    [k: string]: unknown | undefined;
  };
}
/**
 * Um recurso e se está ligado.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FeatureState".
 */
export interface FeatureState {
  /**
   * O recurso.
   */
  feature: 'fpga' | 'assistant' | 'git';
  /**
   * Ligado.
   */
  enabled: boolean;
  /**
   * O que ele faz, em inglês ([`Feature::description`]).
   */
  description: string;
}
/**
 * Uma placa.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Board".
 */
export interface Board {
  /**
   * O identificador, usado no `fpga.json` e na CLI (`de2-115`).
   */
  id: string;
  /**
   * O nome para mostrar (`Terasic DE2-115`).
   */
  name: string;
  /**
   * De onde vieram os pinos: o manual, a versão e as tabelas.
   */
  source: string;
  device: Device;
  jtag: Jtag;
  /**
   * Os sinais da placa ligados ao FPGA.
   */
  signals: BoardSignal[];
  /**
   * O que cada sinal é na placa (chaves, LEDs, displays, tela), para a
   * placa virtual desenhá-los e simulá-los. Um sinal fora daqui também
   * vai para a placa; só não aparece na placa virtual.
   */
  peripherals?: Peripheral[];
}
/**
 * O FPGA.
 */
export interface Device {
  /**
   * A família, como o Quartus a escreve no `.qsf` (`Cyclone IV E`).
   */
  family: string;
  /**
   * O modelo, como no `.qsf` (`EP4CE115F29C7`).
   */
  part: string;
}
/**
 * A gravação pela USB da placa.
 */
export interface Jtag {
  /**
   * O cabo.
   */
  cable: 'usb-blaster' | 'usb-blasterII';
  /**
   * A posição do FPGA na cadeia JTAG, a partir de 1. Na DE10-Nano o FPGA
   * vem depois do processador ARM (HPS).
   */
  position: number;
  /**
   * O nome da placa no openFPGALoader (`-b`), se ele a conhece.
   */
  openfpgaloader?: string | null;
}
/**
 * Um sinal da placa: um fio (`CLOCK_50`) ou um barramento (`SW`), com um
 * pino por bit.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "BoardSignal".
 */
export interface BoardSignal {
  /**
   * O nome, como no manual (`KEY`, `LEDR`).
   */
  name: string;
  /**
   * A direção.
   */
  direction: 'input' | 'output';
  /**
   * Os pinos, do bit 0 em diante (`PIN_Y2`).
   */
  pins: string[];
  /**
   * O padrão de I/O, como no `.qsf` (`3.3-V LVTTL`): um para o sinal
   * inteiro ou um por bit, quando os bits ficam em bancos de tensões
   * diferentes.
   */
  io_standard: string | string[];
  /**
   * Ativo em nível baixo: o botão apertado lê 0, o segmento acende em 0.
   * Num sinal de saída, os bits que nada liga ficam no nível inativo.
   */
  active_low: boolean;
  /**
   * A frequência, se for um clock.
   */
  clock_mhz?: number | null;
  /**
   * O que é, como no manual.
   */
  description: string;
}
/**
 * O FPGA de uma placa.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Device".
 */
export interface Device1 {
  /**
   * A família, como o Quartus a escreve no `.qsf` (`Cyclone IV E`).
   */
  family: string;
  /**
   * O modelo, como no `.qsf` (`EP4CE115F29C7`).
   */
  part: string;
}
/**
 * Como gravar a placa pela USB dela.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Jtag".
 */
export interface Jtag1 {
  /**
   * O cabo.
   */
  cable: 'usb-blaster' | 'usb-blasterII';
  /**
   * A posição do FPGA na cadeia JTAG, a partir de 1. Na DE10-Nano o FPGA
   * vem depois do processador ARM (HPS).
   */
  position: number;
  /**
   * O nome da placa no openFPGALoader (`-b`), se ele a conhece.
   */
  openfpgaloader?: string | null;
}
/**
 * O resultado de [`build`].
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FpgaBuildResult".
 */
export interface FpgaBuildResult {
  /**
   * A placa (o `id`: `de2-115`).
   */
  board: string;
  /**
   * O topo do projeto, que o topo da placa instancia.
   */
  top: string;
  /**
   * A pasta do projeto do Quartus ([`build_dir`]).
   */
  dir: string;
  quartus: Quartus;
  /**
   * Como a compilação terminou. Um design que não alcança o clock
   * compila do mesmo jeito: quem diz é [`FpgaBuildResult::timing`].
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * O passo que falhou.
   */
  failed_step: Step | null;
  /**
   * Até quatro passos, todos do Quartus: `synthesize` (`quartus_map`),
   * `fit`, `bitstream` (`quartus_asm`) e `timing` (`quartus_sta`).
   */
  steps: StepReport[];
  /**
   * Os erros e avisos do Quartus, sem os `Info` e sem repetição (o aviso
   * de tempo vem uma vez por canto analisado).
   */
  diagnostics: Diagnostic[];
  /**
   * O topo da placa, o `.qsf` e os arquivos de gravação: o `.sof`,
   * obrigatório, e o `.rbf` e o `.svf`, para o openFPGALoader.
   */
  artifacts: Artifact[];
  /**
   * O `.sof`, quando a compilação terminou.
   */
  bitstream: string | null;
  /**
   * O que o design usa da FPGA, do resumo do Fitter, quando ele
   * terminou.
   */
  resources: ResourceUsage[];
  /**
   * A Fmax e as folgas de cada clock, quando a análise de tempo
   * terminou.
   */
  timing: TimingSummary | null;
  /**
   * Os ajustes das ligações do `fpga.json` (larguras completadas,
   * entradas soltas), como em `aurora fpga check`.
   */
  notes: string[];
  /**
   * Quanto a compilação levou, do começo ao fim, em milissegundos.
   */
  duration_ms: number;
}
/**
 * O Quartus que compilou.
 */
export interface Quartus {
  /**
   * A pasta `quartus` da instalação (`C:\intelFPGA_lite\22.1std\quartus`).
   */
  root: string;
  /**
   * A pasta dos programas: `bin64` no Windows, `bin` no Linux.
   */
  bin: string;
  /**
   * A versão, pelo nome da pasta da instalação (`22.1std`).
   */
  version?: string | null;
}
/**
 * Um recurso da FPGA no resumo do Fitter.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ResourceUsage".
 */
export interface ResourceUsage {
  /**
   * O nome, como o Quartus escreve (`Total logic elements`, `Total pins`,
   * `Logic utilization (in ALMs)`).
   */
  name: string;
  /**
   * Quanto o design usa.
   */
  used: number;
  /**
   * Quanto a FPGA tem, quando o Quartus diz.
   */
  available: number | null;
  /**
   * Detalhe do recurso de cima (`Dedicated logic registers`, dentro de
   * `Total logic elements`).
   */
  detail: boolean;
}
/**
 * O tempo do design.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "TimingSummary".
 */
export interface TimingSummary {
  /**
   * Um por clock, na ordem em que o relatório os cita.
   */
  clocks: ClockTiming[];
  /**
   * Nenhuma folga de setup ou de hold é negativa.
   */
  met: boolean;
}
/**
 * O tempo de um clock, no pior dos cantos de operação (tensão e
 * temperatura) que o Timing Analyzer analisa.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ClockTiming".
 */
export interface ClockTiming {
  /**
   * O clock: o sinal da placa, pelo nome do `create_clock`, ou o que o
   * Quartus achou que é clock (um registrador que dirige um `always`).
   */
  clock: string;
  /**
   * A frequência do oscilador da placa, em MHz: a que o `.sdc` pede.
   * `None` num clock que não é da placa.
   */
  target_mhz: number | null;
  /**
   * A maior frequência em que o design funciona com esse clock (a
   * `Restricted Fmax`), em MHz.
   */
  fmax_mhz: number | null;
  /**
   * A folga de setup, em ns. Negativa: o design não alcança a frequência
   * do clock.
   */
  setup_slack_ns: number | null;
  /**
   * A folga de hold, em ns.
   */
  hold_slack_ns: number | null;
}
/**
 * Uma instalação do Quartus Prime.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Quartus".
 */
export interface Quartus1 {
  /**
   * A pasta `quartus` da instalação (`C:\intelFPGA_lite\22.1std\quartus`).
   */
  root: string;
  /**
   * A pasta dos programas: `bin64` no Windows, `bin` no Linux.
   */
  bin: string;
  /**
   * A versão, pelo nome da pasta da instalação (`22.1std`).
   */
  version?: string | null;
}
/**
 * As ligações conferidas, bit a bit: o que o topo da placa precisa para ser
 * gerado.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Resolved".
 */
export interface Resolved {
  board: Board1;
  /**
   * O módulo do topo.
   */
  top: string;
  /**
   * As portas do topo, na ordem da declaração.
   */
  ports: TopPort[];
  /**
   * As entradas do topo, com a origem de cada bit, do bit 0 em diante.
   */
  inputs: PortDrive[];
  /**
   * Os sinais de saída da placa usados, com a porta que dirige cada bit.
   */
  board_outputs: SignalDrive[];
  /**
   * Os sinais de entrada da placa usados, na ordem em que aparecem.
   */
  board_inputs: string[];
  /**
   * As portas ligadas a um clock da placa.
   */
  clocks: Clock[];
  /**
   * Os ajustes feitos: larguras completadas ou cortadas, entradas soltas.
   */
  notes: string[];
}
/**
 * A placa.
 */
export interface Board1 {
  /**
   * O identificador, usado no `fpga.json` e na CLI (`de2-115`).
   */
  id: string;
  /**
   * O nome para mostrar (`Terasic DE2-115`).
   */
  name: string;
  /**
   * De onde vieram os pinos: o manual, a versão e as tabelas.
   */
  source: string;
  device: Device;
  jtag: Jtag;
  /**
   * Os sinais da placa ligados ao FPGA.
   */
  signals: BoardSignal[];
  /**
   * O que cada sinal é na placa (chaves, LEDs, displays, tela), para a
   * placa virtual desenhá-los e simulá-los. Um sinal fora daqui também
   * vai para a placa; só não aparece na placa virtual.
   */
  peripherals?: Peripheral[];
}
/**
 * Uma porta do topo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "TopPort".
 */
export interface TopPort {
  /**
   * O nome.
   */
  name: string;
  /**
   * `true` para entrada, `false` para saída.
   */
  input: boolean;
  /**
   * A largura.
   */
  width: number;
}
/**
 * A origem de cada bit de uma entrada do topo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "PortDrive".
 */
export interface PortDrive {
  /**
   * A porta.
   */
  port: string;
  /**
   * Um item por bit, do bit 0 em diante.
   */
  bits: BitSource[];
}
/**
 * O que dirige cada bit de um sinal de saída da placa.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SignalDrive".
 */
export interface SignalDrive {
  /**
   * O sinal.
   */
  signal: string;
  /**
   * Ativo em nível baixo: os bits sem ligação ficam em 1.
   */
  active_low: boolean;
  /**
   * Um item por bit, do bit 0 em diante; `None` fica no nível inativo.
   */
  bits: (PortBit | null)[];
}
/**
 * Um bit de uma saída do topo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "PortBit".
 */
export interface PortBit {
  /**
   * A porta.
   */
  port: string;
  /**
   * O bit.
   */
  bit: number;
  /**
   * Invertido (`!`).
   */
  invert: boolean;
}
/**
 * Uma porta ligada a um clock da placa: vai para o `.sdc`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Clock".
 */
export interface Clock {
  /**
   * O sinal da placa.
   */
  signal: string;
  /**
   * A porta do topo.
   */
  port: string;
  /**
   * A frequência.
   */
  mhz: number;
}
/**
 * Uma ligação como texto, para mostrar.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Connection".
 */
export interface Connection {
  /**
   * A entrada do topo (`in[15:0]`) ou o sinal de saída da placa
   * (`LEDR[17:0]`).
   */
  target: string;
  /**
   * O que a dirige (`~KEY[0]`, `{10'b0, out[7:0]}`).
   */
  source: string;
}
/**
 * Uma chamada do `iverilog` e a árvore que saiu dela.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Elaboration".
 */
export interface Elaboration {
  /**
   * O testbench elaborado; `None` no design.
   */
  testbench: string | null;
  /**
   * O processador, quando o testbench é o que o build dele gerou.
   */
  processor: string | null;
  /**
   * Como esta elaboração terminou. Uma que falha não impede as outras.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * Os módulos que nenhum outro instancia, com o que eles instanciam.
   * Vazio quando a elaboração falhou.
   */
  roots: ModuleInstance[];
  /**
   * Os erros e avisos desta elaboração (também em
   * [`HierarchyResult::diagnostics`]).
   */
  diagnostics: Diagnostic[];
}
/**
 * Uma instância de módulo na hierarquia elaborada.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ModuleInstance".
 */
export interface ModuleInstance {
  /**
   * O nome da instância (`dut`). Numa raiz, o nome do módulo. Dentro de
   * blocos `generate`, `begin` ou `fork`, com o nome deles na frente,
   * separado por `.` (`op_add.my_add`): os blocos não viram nós.
   */
  name: string;
  /**
   * O módulo instanciado.
   */
  module: string;
  /**
   * O arquivo que define o módulo, absoluto.
   */
  file: string | null;
  /**
   * A linha da definição (`module ...`).
   */
  line: number | null;
  /**
   * O arquivo da instância, no módulo de cima. `None` numa raiz.
   */
  instance_file: string | null;
  /**
   * A linha da instância.
   */
  instance_line: number | null;
  /**
   * O módulo vem da biblioteca SAPHO
   * ([`Toolchain::sapho_library`](crate::Toolchain::sapho_library)).
   */
  library: boolean;
  /**
   * As instâncias de dentro, na ordem do fonte.
   */
  children: ModuleInstance[];
}
/**
 * O resultado da correção.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Grade".
 */
export interface Grade {
  /**
   * O exercício.
   */
  exercise: string;
  /**
   * Como terminou.
   */
  verdict: 'solved' | 'compile_error' | 'mismatch' | 'incomplete' | 'timed_out' | 'cancelled';
  /**
   * Quantas amostras o testbench comparou.
   */
  samples: number;
  /**
   * Em quantas amostras alguma saída errou.
   */
  mismatched: number;
  /**
   * Cada saída, na ordem das portas.
   */
  outputs: OutputCheck[];
  /**
   * Com reset: quantas amostras erradas foram com ele ativo.
   */
  reset_mismatches: number | null;
  /**
   * A primeira amostra errada, com as entradas e as saídas.
   */
  counterexample: Counterexample | null;
  /**
   * O que se percebeu sobre a causa do erro, e o latch que a síntese
   * achou (este, também com o exercício resolvido).
   */
  findings: Finding[];
  /**
   * Os erros e avisos dos compiladores sobre o código do aluno, com os
   * avisos do lint do Verilator quando ele está instalado.
   */
  diagnostics: Diagnostic[];
  /**
   * O que o testbench escreveu, fora o resumo (um `tb.v` escrito à mão
   * pode explicar o erro aqui).
   */
  output: string[];
  /**
   * A onda da simulação.
   */
  waveform: string | null;
  /**
   * O arquivo de comandos do Surfer que mostra a onda com as entradas, as
   * saídas lado a lado com as da referência e o primeiro erro marcado.
   */
  layout: string | null;
  /**
   * Quanto a correção levou, em milissegundos.
   */
  duration_ms: number;
}
/**
 * O resultado de uma saída do módulo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "OutputCheck".
 */
export interface OutputCheck {
  /**
   * O nome da porta.
   */
  name: string;
  /**
   * Em quantas amostras ela diferiu da referência.
   */
  mismatches: number;
  /**
   * O instante do primeiro erro, em ns.
   */
  first_ns: number | null;
  /**
   * Das amostras erradas, quantas tinham bit em X ou Z.
   */
  unknown: number;
}
/**
 * A primeira amostra errada: o que entrou e o que saiu, para o aluno
 * refazer o caso de cabeça. Vem das linhas `first` do resumo; um `tb.v`
 * escrito à mão que não as escreve fica sem.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Counterexample".
 */
export interface Counterexample {
  /**
   * O instante da amostra, em ns.
   */
  time_ns: number;
  /**
   * No sequencial, o ciclo (a partir de 0).
   */
  cycle: number | null;
  /**
   * As entradas nessa amostra, sem o clock.
   */
  inputs: SignalValue[];
  /**
   * Cada saída nessa amostra: a esperada e a do aluno.
   */
  outputs: OutputValue[];
}
/**
 * O valor de uma entrada no contraexemplo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SignalValue".
 */
export interface SignalValue {
  /**
   * A porta.
   */
  name: string;
  /**
   * Os bits, do mais significativo ao menos, com `x` e `z`.
   */
  bits: string;
  /**
   * O valor para ler: `1`, `4'b0101 (5)`, `16'h00ff (255)`.
   */
  text: string;
}
/**
 * Uma saída no contraexemplo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "OutputValue".
 */
export interface OutputValue {
  /**
   * A porta.
   */
  name: string;
  /**
   * Os bits que a referência deu.
   */
  expected_bits: string;
  /**
   * Os bits que o módulo do aluno deu.
   */
  actual_bits: string;
  /**
   * O esperado, para ler.
   */
  expected: string;
  /**
   * O do aluno, para ler.
   */
  actual: string;
  /**
   * Se esta saída errou nessa amostra.
   */
  wrong: boolean;
}
/**
 * Um exercício conferido.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "DevExercise".
 */
export interface DevExercise {
  /**
   * O nome.
   */
  name: string;
  /**
   * O que está errado; vazio quando está tudo certo.
   */
  problems: string[];
  /**
   * Como o `start.v` se saiu (não pode ser `solved`).
   */
  start: Verdict | null;
  /**
   * Como a `solution.v` se saiu (tem que ser `solved`).
   */
  solution: Verdict | null;
}
/**
 * Um exercício na lista, com o que o aluno já fez.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ExerciseStatus".
 */
export interface ExerciseStatus {
  /**
   * O nome.
   */
  name: string;
  /**
   * O capítulo (a pasta dele).
   */
  chapter: string;
  /**
   * O título.
   */
  title: string;
  /**
   * Já resolvido.
   */
  solved: boolean;
  /**
   * É o exercício atual.
   */
  current: boolean;
  /**
   * O arquivo que o aluno edita.
   */
  file: string;
}
/**
 * O que [`Project::move_path`] fez.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "MovedPath".
 */
export interface MovedPath {
  /**
   * Onde estava, absoluto.
   */
  from: string;
  /**
   * Onde ficou, absoluto.
   */
  to: string;
  /**
   * Os arquivos registrados no `.spf` que mudaram de lugar, já com o
   * caminho novo: o próprio arquivo, ou os que estavam dentro da pasta.
   * Continuam com o mesmo papel e a mesma marca de topo ou de testbench
   * escolhido.
   */
  files: ProjectFile[];
}
/**
 * Um arquivo registrado no projeto.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ProjectFile".
 */
export interface ProjectFile {
  /**
   * Em qual lista ele está.
   */
  role: 'synthesizable' | 'testbench';
  /**
   * Caminho absoluto.
   */
  path: string;
  /**
   * Marcado como topo (sintetizável) ou como o testbench escolhido.
   */
  top_level: boolean;
}
/**
 * O que [`Project::remove_processor`] tirou do projeto.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "RemovedProcessor".
 */
export interface RemovedProcessor {
  /**
   * O nome do processador.
   */
  name: string;
  /**
   * A pasta dele (`<raiz>/<nome>`), que continua no disco.
   */
  dir: string;
  /**
   * Os arquivos de dentro da pasta que estavam registrados e saíram das
   * listas: o papel de cada um e se era o topo ou o testbench escolhido.
   */
  unregistered: ProjectFile[];
}
/**
 * Parâmetros do cabeçalho de um processador novo.
 *
 * Os padrões são os da AURORA (`js/project/processor_defaults.ts`): o float
 * estreito do SAPHO, 23 = 16 + 6 + 1 bits. No fluxo C só as portas vão para
 * o fonte; o resto fica com o que o `cppcomp` foi compilado para assumir.
 *
 * Em JSON, só `name` e `language` são obrigatórios: um campo ausente fica
 * com o padrão de [`NewProcessor::new`], e, sem `nubits`, com `nbmant` ou
 * `nbexpo` dados, `nubits` vira `nbmant + nbexpo + 1`
 * ([`NewProcessor::nubits_for`]), a única largura que o `asmcomp` aceita.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "NewProcessor".
 */
export interface NewProcessor {
  /**
   * Nome do processador: letras, dígitos e `_`, começando por letra ou `_`.
   */
  name: string;
  /**
   * Linguagem do fonte-modelo.
   */
  language: 'cmm' | 'cpp';
  /**
   * `#NUBITS`: largura da palavra de dados, em bits. Só C±.
   */
  nubits?: number;
  /**
   * `#NBMANT`: bits de mantissa do ponto flutuante do SAPHO (não é IEEE
   * 754). Só C±.
   */
  nbmant?: number;
  /**
   * `#NBEXPO`: bits de expoente do ponto flutuante. Só C±.
   */
  nbexpo?: number;
  /**
   * `#NUGAIN`: divisor de `norm(x)`. Precisa ser potência de dois, senão o
   * `cmmcomp` recusa (um divisor genérico viraria o caminho crítico da
   * ULA). Só C±; o fonte só traz a diretiva quando difere de 128, o
   * padrão do YANC.
   */
  nugain?: number;
  /**
   * `#NDSTAC`: profundidade da pilha de dados. Só C±. `None`, o padrão:
   * sem a diretiva, e o `asmcomp` calcula a profundidade pelo programa.
   */
  ndstac?: number | null;
  /**
   * `#SDEPTH`: profundidade da pilha de instruções (chamadas). Só C±.
   * `None`, o padrão: sem a diretiva, calculada pelo programa.
   */
  sdepth?: number | null;
  /**
   * `#NUIOIN` / `#pragma yanc nuioin`: número de portas de entrada.
   */
  input_ports?: number;
  /**
   * `#NUIOOU` / `#pragma yanc nuioou`: número de portas de saída.
   */
  output_ports?: number;
}
/**
 * Parâmetros de simulação de um processador, para
 * [`Project::configure_processor`]. `None` em um campo mantém o valor
 * atual.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ProcessorConfig".
 */
export interface ProcessorConfig {
  /**
   * Frequência de operação em MHz (`clk` no `.spf`).
   */
  frequency_mhz?: number | null;
  /**
   * Clocks que o testbench gerado simula (`numClocks` no `.spf`).
   */
  clocks?: number | null;
  /**
   * Exportar arrays para a simulação (`showArrays` no `.spf`).
   */
  show_arrays?: boolean | null;
}
/**
 * O que é da interface, não do fluxo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FlowOptions".
 */
export interface FlowOptions {
  /**
   * O texto do comando para o histórico e para o console (`aurora sim -p
   * soma`, `aurora-ide sim -p soma`). A CLI passa a linha digitada; um
   * cliente sem linha de comando monta com [`FlowRequest::command_line`].
   */
  command: string;
  /**
   * Como abrir a onda quando o pedido tem `open_wave`.
   */
  wave?: 'none' | 'window';
  /**
   * Quem abre o RTL viewer do `synth --open`: `None` abre no navegador do
   * sistema; `Some(nome)` só grava o HTML, para quem chamou abrir (o
   * `AURORA_OPENER=vscode` do terminal da extensão do VS Code).
   */
  viewer_opener?: string | null;
}
/**
 * O pedido de [`open_wave`].
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WaveRequest".
 */
export interface WaveRequest {
  /**
   * Qual onda.
   */
  source:
    | {
        /**
         * O arquivo.
         */
        path: string;
        kind: 'file';
      }
    | {
        /**
         * O processador; `None`, o projeto.
         */
        processor?: string | null;
        kind: 'project';
      };
  /**
   * O layout.
   */
  layout?:
    | {
        kind: 'generated';
      }
    | {
        kind: 'none';
      }
    | {
        kind: 'reset';
      }
    | {
        /**
         * O arquivo (`.surf.ron` ou `.sucl`).
         */
        path: string;
        kind: 'file';
      };
}
/**
 * O que o `fpga.json` guarda.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FpgaConfig".
 */
export interface FpgaConfig {
  /**
   * A placa ([`super::board()`]).
   */
  board: string;
  /**
   * O módulo que vai para a placa. Sem ele, o topo do projeto.
   */
  top?: string | null;
  /**
   * As ligações, na ordem do arquivo. Uma porta com vários sinais (uma
   * saída que vai a mais de um lugar da placa) tem uma [`Link`] para cada.
   */
  connect?: {
    [k: string]: LinkTargets | undefined;
  };
}
/**
 * Uma mudança de ligação feita na placa ([`FpgaConfig::reconnect`]): o sinal
 * da placa, um bit dele ou ele inteiro, e a porta do topo que passa a ir
 * nele.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Reconnect".
 */
export interface Reconnect {
  /**
   * O sinal da placa (`SW`, `HEX0`).
   */
  signal: string;
  /**
   * O bit do sinal; `None`, o sinal inteiro, do bit 0 em diante.
   */
  bit?: number | null;
  /**
   * A porta que vai nele; `None` deixa o bit (ou o sinal) solto.
   */
  port?: PortChoice | null;
}
/**
 * A porta do topo de uma [`Reconnect`].
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "PortChoice".
 */
export interface PortChoice {
  /**
   * A porta.
   */
  port: string;
  /**
   * O bit da porta que vai no bit do sinal (no sinal inteiro, o que vai no
   * bit 0 dele); `None`, o bit 0.
   */
  bit?: number | null;
  /**
   * Invertido (`!`): o botão apertado chega como 1, o LED acende com 0.
   */
  invert?: boolean;
}
/**
 * `initialize`.
 */
export interface ServerInfo {
  /**
   * A versão da AURORA.
   */
  aurora: string;
  /**
   * A versão do protocolo ([`PROTOCOL_VERSION`]).
   */
  protocol: number;
  /**
   * A pasta da instalação da AURORA, quando o servidor é o `aurora`
   * instalado.
   */
  prefix: string | null;
  /**
   * Cada recurso experimental e se está ligado.
   */
  experimental: {
    [k: string]: boolean | undefined;
  };
  /**
   * Os métodos que este servidor atende.
   */
  methods: string[];
}
/**
 * `project.create`, `project.status`, `project.testbench`.
 */
export interface ProjectStatus {
  /**
   * O nome do projeto (o do `.spf`).
   */
  name: string;
  /**
   * O `.spf`.
   */
  spf: string;
  /**
   * A pasta do projeto.
   */
  root: string;
  /**
   * Os módulos registrados (sintetizáveis), na ordem do `.spf`.
   */
  synthesizable: ProjectFile[];
  /**
   * Os testbenches registrados, na ordem do `.spf`.
   */
  testbenches: ProjectFile[];
  /**
   * O arquivo de topo, se escolhido.
   */
  top_level: string | null;
  /**
   * O módulo de topo, quando dá para saber: o `topLevelModule` do `.spf`,
   * o único módulo do arquivo de topo ou o que tem o nome dele.
   */
  top_module: string | null;
  /**
   * Por que o módulo de topo não dá para saber (arquivo com vários
   * módulos, nenhum com o nome dele).
   */
  top_module_error: ErrorInfo | null;
  /**
   * O testbench que a simulação do projeto roda.
   */
  selected_testbench: string | null;
  /**
   * O módulo desse testbench.
   */
  testbench_module: string | null;
  /**
   * A escolha de sinais da onda desse testbench (`wave/<testbench>.json`);
   * vazia, a onda grava todos os sinais.
   */
  wave_selection: string[];
  /**
   * Os `.v` e `.sv` da pasta que não estão registrados, e os `.py` com
   * `@cocotb.test`.
   */
  unregistered: string[];
  /**
   * Os processadores SAPHO, na ordem do `.spf`.
   */
  processors: ProcessorStatus[];
  /**
   * A onda da simulação do projeto, se já existe no disco.
   */
  waveform: string | null;
  /**
   * Os Verilog conhecidos que podem ser o topo: os registrados, o gerado
   * de cada processador e os de fora do `.spf`, menos os com nome de
   * testbench e os testbenches cocotb (a regra de
   * [`Project::set_top_level`]).
   */
  top_candidates: string[];
  /**
   * O que está estranho no `.spf` sem impedir de abrir
   * ([`Project::issues`]).
   */
  issues: ProjectIssue[];
}
/**
 * Um processador, com o que uma interface precisa saber dele além do
 * [`Processor`].
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ProcessorStatus".
 */
export interface ProcessorStatus {
  /**
   * O nome: a pasta, o nome do fonte, o módulo Verilog e os arquivos
   * gerados.
   */
  name: string;
  /**
   * A linguagem do fonte.
   */
  language: 'cmm' | 'cpp';
  /**
   * `<raiz>/<nome>`: o fonte e os arquivos que a pessoa escreve (os que
   * inicializam arrays, `input_<n>.txt`, `cade_<n>.txt`, os cabeçalhos
   * do C).
   */
  dir: string;
  /**
   * O programa-fonte, `<dir>/<nome>.cmm` ou `.cpp`.
   */
  source: string;
  /**
   * `<dir>/<nome>`, a pasta de saída do YANC (`-o`): o `.asm`, o Verilog,
   * as memórias, o testbench, as tabelas da onda e as saídas da
   * simulação, que roda nela.
   */
  output_dir: string;
  /**
   * `<raiz>/.aurora8/Temp/<nome>`: o que só a AURORA usa (o `.vvp`, o modelo do
   * Verilator, a cópia instrumentada do testbench, a trava de execução).
   */
  temp_dir: string;
  /**
   * Frequência de operação em MHz (`-f` do `asmcomp`).
   */
  frequency_mhz: number;
  /**
   * Clocks a simular (`-c` do `asmcomp`, vai para o testbench).
   */
  clocks: number;
  /**
   * Exporta cada elemento de array como um sinal da onda
   * (`--wave-arrays` do `cmmcomp`).
   */
  show_arrays: boolean;
  /**
   * O Verilog e o testbench gerados existem ([`Processor::is_built`]).
   */
  built: boolean;
  /**
   * Os arquivos de entrada (`<proc>/input_<n>.txt`, ao lado do fonte) que
   * existem, por porta.
   */
  inputs: PortFile[];
  /**
   * Os arquivos de saída (`<proc>/<proc>/output_<n>.txt`, na pasta de
   * saída) que existem, por porta.
   */
  outputs: PortFile[];
  /**
   * As entradas que o testbench lê e que não existem; só depois do build,
   * porque é o testbench gerado que diz quais são.
   */
  missing_inputs: string[];
  /**
   * A onda da simulação do processador, se já existe.
   */
  waveform: string | null;
  generated: GeneratedFiles;
}
/**
 * Um arquivo de porta de entrada ou de saída de um processador.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "PortFile".
 */
export interface PortFile {
  /**
   * A porta (o `n` de `in(n)` e `out(n)`).
   */
  port: number;
  /**
   * O arquivo.
   */
  path: string;
}
/**
 * O que o build gerou e existe no disco.
 */
export interface GeneratedFiles {
  /**
   * `<nome>.v`: o processador em Verilog, um módulo do design.
   */
  verilog: string | null;
  /**
   * `<nome>_tb.v`: o testbench que o `asmcomp` gera.
   */
  testbench: string | null;
  /**
   * `<nome>.asm`: o assembly que o compilador gera.
   */
  assembly: string | null;
  /**
   * `*.mif`: as memórias de dados e de instruções.
   */
  memories: string[];
  /**
   * As tabelas e os logs do YANC (`prog_log.txt`, `src_lines.txt`,
   * `trad_opcode.txt`, `app_log.txt`, `pc_<nome>_mem.txt`); as saídas da
   * simulação (`output_<n>.txt`, `toma_<n>.txt`) ficam de fora.
   */
  intermediates: string[];
}
/**
 * Um aviso de [`Project::issues`]: algo estranho no `.spf` que não impede
 * de abrir.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ProjectIssue".
 */
export interface ProjectIssue {
  /**
   * O tipo, para a interface traduzir.
   */
  kind:
    | 'rescued_path'
    | 'selection_not_registered'
    | 'testbench_as_top'
    | 'invalid_processor_name'
    | 'duplicate_file'
    | 'nested_project'
    | 'legacy_layout';
  /**
   * O arquivo ou a pasta envolvida, absoluta.
   */
  path: string | null;
  /**
   * O complemento do tipo: o caminho como estava gravado
   * (`rescued_path`), o campo do `.spf` (`selection_not_registered`,
   * `testbench_as_top`) ou o nome do processador
   * (`invalid_processor_name`, `legacy_layout`).
   */
  detail: string | null;
  /**
   * O aviso inteiro, em inglês, pronto para mostrar.
   */
  message: string;
}
/**
 * `project.remove`.
 */
export interface RemoveOutcome {
  /**
   * Tirados do projeto (continuam no disco).
   */
  removed: string[];
  /**
   * Pedidos, mas que não estavam no projeto.
   */
  not_registered: string[];
}
/**
 * `project.top`.
 */
export interface TopOutcome {
  /**
   * O arquivo de topo, se escolhido.
   */
  top_level: string | null;
  /**
   * O módulo de topo.
   */
  module: string | null;
  /**
   * Por que não dá para saber o módulo de um arquivo de topo escolhido.
   */
  module_error: ErrorInfo | null;
}
/**
 * `project.move`.
 */
export interface MovedPath1 {
  /**
   * Onde estava, absoluto.
   */
  from: string;
  /**
   * Onde ficou, absoluto.
   */
  to: string;
  /**
   * Os arquivos registrados no `.spf` que mudaram de lugar, já com o
   * caminho novo: o próprio arquivo, ou os que estavam dentro da pasta.
   * Continuam com o mesmo papel e a mesma marca de topo ou de testbench
   * escolhido.
   */
  files: ProjectFile[];
}
/**
 * `project.convert`.
 */
export interface LayoutConversion {
  /**
   * Os processadores que tinham a estrutura antiga, na ordem do `.spf`.
   */
  processors: string[];
  /**
   * O que mudou de lugar.
   */
  moved: ConvertedPath[];
  /**
   * O que ficou onde estava, com o motivo.
   */
  skipped: SkippedPath[];
  /**
   * Os fontes que perderam a linha do nome (`#PRNAME`, `#pragma yanc
   * prname`).
   */
  name_lines: RemovedNameLine[];
  /**
   * Os processadores cujo nome não serve mais: na 7.1 ele vira o módulo
   * Verilog, e o YANC recusa um fonte com esse nome.
   */
  invalid_names: string[];
  /**
   * As subpastas antigas que ficaram com algo que não foi movido (o que o
   * sistema deixa sozinho numa pasta, como o `.DS_Store`, não conta).
   */
  remaining_dirs: string[];
}
/**
 * `project.check_name`, `processor.check_name`.
 */
export interface NameCheck {
  /**
   * O nome serve.
   */
  ok: boolean;
  /**
   * Por que não serve (`invalid_name`).
   */
  error: ErrorInfo | null;
}
/**
 * `processor.add`, `processor.set`.
 */
export interface Processor {
  /**
   * O nome: a pasta, o nome do fonte, o módulo Verilog e os arquivos
   * gerados.
   */
  name: string;
  /**
   * A linguagem do fonte.
   */
  language: 'cmm' | 'cpp';
  /**
   * `<raiz>/<nome>`: o fonte e os arquivos que a pessoa escreve (os que
   * inicializam arrays, `input_<n>.txt`, `cade_<n>.txt`, os cabeçalhos
   * do C).
   */
  dir: string;
  /**
   * O programa-fonte, `<dir>/<nome>.cmm` ou `.cpp`.
   */
  source: string;
  /**
   * `<dir>/<nome>`, a pasta de saída do YANC (`-o`): o `.asm`, o Verilog,
   * as memórias, o testbench, as tabelas da onda e as saídas da
   * simulação, que roda nela.
   */
  output_dir: string;
  /**
   * `<raiz>/.aurora8/Temp/<nome>`: o que só a AURORA usa (o `.vvp`, o modelo do
   * Verilator, a cópia instrumentada do testbench, a trava de execução).
   */
  temp_dir: string;
  /**
   * Frequência de operação em MHz (`-f` do `asmcomp`).
   */
  frequency_mhz: number;
  /**
   * Clocks a simular (`-c` do `asmcomp`, vai para o testbench).
   */
  clocks: number;
  /**
   * Exporta cada elemento de array como um sinal da onda
   * (`--wave-arrays` do `cmmcomp`).
   */
  show_arrays: boolean;
}
/**
 * `processor.remove`.
 */
export interface ProcessorRemoval {
  removed: RemovedProcessor1;
  /**
   * A pasta foi para a lixeira do sistema.
   */
  trashed: boolean;
}
/**
 * O que saiu: o nome, a pasta e os arquivos que saíram das listas.
 */
export interface RemovedProcessor1 {
  /**
   * O nome do processador.
   */
  name: string;
  /**
   * A pasta dele (`<raiz>/<nome>`), que continua no disco.
   */
  dir: string;
  /**
   * Os arquivos de dentro da pasta que estavam registrados e saíram das
   * listas: o papel de cada um e se era o topo ou o testbench escolhido.
   */
  unregistered: ProjectFile[];
}
/**
 * `processor.defaults`.
 */
export interface NewProcessor1 {
  /**
   * Nome do processador: letras, dígitos e `_`, começando por letra ou `_`.
   */
  name: string;
  /**
   * Linguagem do fonte-modelo.
   */
  language: 'cmm' | 'cpp';
  /**
   * `#NUBITS`: largura da palavra de dados, em bits. Só C±.
   */
  nubits?: number;
  /**
   * `#NBMANT`: bits de mantissa do ponto flutuante do SAPHO (não é IEEE
   * 754). Só C±.
   */
  nbmant?: number;
  /**
   * `#NBEXPO`: bits de expoente do ponto flutuante. Só C±.
   */
  nbexpo?: number;
  /**
   * `#NUGAIN`: divisor de `norm(x)`. Precisa ser potência de dois, senão o
   * `cmmcomp` recusa (um divisor genérico viraria o caminho crítico da
   * ULA). Só C±; o fonte só traz a diretiva quando difere de 128, o
   * padrão do YANC.
   */
  nugain?: number;
  /**
   * `#NDSTAC`: profundidade da pilha de dados. Só C±. `None`, o padrão:
   * sem a diretiva, e o `asmcomp` calcula a profundidade pelo programa.
   */
  ndstac?: number | null;
  /**
   * `#SDEPTH`: profundidade da pilha de instruções (chamadas). Só C±.
   * `None`, o padrão: sem a diretiva, calculada pelo programa.
   */
  sdepth?: number | null;
  /**
   * `#NUIOIN` / `#pragma yanc nuioin`: número de portas de entrada.
   */
  input_ports?: number;
  /**
   * `#NUIOOU` / `#pragma yanc nuioou`: número de portas de saída.
   */
  output_ports?: number;
}
/**
 * `wave.layout`.
 */
export interface WaveLayoutInfo {
  /**
   * O layout dos processadores SAPHO, gerado da onda; `None` sem
   * processador.
   */
  layout: WaveLayout | null;
  /**
   * O layout salvo no projeto da onda (`wave/<testbench>.surf.ron`), onde
   * o Surfer salva (Ctrl+S): o do usuário, se ele salvou depois da última
   * geração, senão o gerado agora; `None` numa onda fora de projeto.
   */
  saved: SavedLayout | null;
  /**
   * Por que o layout não foi gerado (a onda abre sem ele).
   */
  layout_error: string | null;
  /**
   * Os comandos do Surfer de um exercício do `aurora learn` (o
   * `.aurora-learn/wave.sucl` ao lado da onda), quando a onda é de um.
   */
  exercise_layout: string | null;
}
/**
 * O layout de uma onda, em memória.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WaveLayout".
 */
export interface WaveLayout {
  /**
   * O conteúdo do `.surf.ron`.
   */
  state: string;
  /**
   * Os tradutores de valor que o estado usa, pelo nome.
   */
  mappings: MappingTranslator[];
  /**
   * Os processadores achados na onda, na ordem do cabeçalho.
   */
  processors: WaveProcessor[];
  /**
   * O testbench da onda (o escopo de cima dela): o nome dos arquivos dele
   * em `wave/`, a escolha de sinais e o layout salvo.
   */
  testbench: string | null;
  /**
   * A escolha de sinais que o layout seguiu (`wave/<testbench>.json`);
   * vazia sem escolha.
   */
  selection: string[];
}
/**
 * Um tradutor de valor do Surfer: um arquivo `Name = <nome>`, `Bits = <n>`
 * e uma linha `<valor> <texto>` por valor.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "MappingTranslator".
 */
export interface MappingTranslator {
  /**
   * O nome, que é também o nome do arquivo e o `format` dos sinais.
   */
  name: string;
  /**
   * O conteúdo do arquivo.
   */
  content: string;
}
/**
 * Um processador SAPHO achado na onda.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WaveProcessor".
 */
export interface WaveProcessor {
  /**
   * O escopo da instância na onda (`soma_tb.proc`).
   */
  instance: string;
  /**
   * O processador (o nome da pasta no projeto e do fonte).
   */
  processor: string;
  /**
   * Quantas variáveis do programa estão na onda (cada elemento de array
   * conta).
   */
  variables: number;
  /**
   * O PC aparece como instrução de assembly (havia `trad_opcode.txt`).
   */
  assembly: boolean;
  /**
   * A linha do fonte aparece como texto (havia `src_lines.txt`).
   */
  source: boolean;
  /**
   * As tabelas são mais novas que a onda: o processador foi compilado de
   * novo depois da simulação, e o assembly e a linha do C± sairiam
   * deslocados. O PC e a linha aparecem como números até simular de novo;
   * as interfaces avisam.
   */
  outdated: boolean;
}
/**
 * O layout salvo no projeto para a onda de um testbench:
 * `wave/<testbench>.surf.ron` ([`saved_layout`]).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SavedLayout".
 */
export interface SavedLayout {
  /**
   * O arquivo, onde o Surfer salva o estado (Ctrl+S).
   */
  path: string;
  /**
   * O estado que vai para o Surfer: o que o usuário salvou, ou o gerado.
   */
  state: string;
  /**
   * O usuário salvou o estado depois da última vez que a AURORA o gerou: a
   * AURORA o deixa como está.
   */
  customized: boolean;
}
/**
 * `fpga.top`.
 */
export interface ModuleInterface {
  /**
   * Nome do módulo.
   */
  name: string;
  /**
   * O arquivo que o declara.
   */
  file: string;
  /**
   * As portas.
   */
  ports: Port[];
}
/**
 * Uma porta de um módulo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Port".
 */
export interface Port {
  /**
   * Nome.
   */
  name: string;
  /**
   * Direção.
   */
  direction: 'input' | 'output' | 'inout';
  /**
   * Largura em bits, com os parâmetros nos valores padrão.
   */
  width: number;
  /**
   * `signed`.
   */
  signed: boolean;
}
/**
 * `vboard.layout`.
 */
export interface BoardLayout {
  /**
   * A placa (`de2-115`).
   */
  board: string;
  /**
   * O nome da placa (`Terasic DE2-115`).
   */
  board_name: string;
  /**
   * O módulo do projeto que vai na placa.
   */
  top: string;
  /**
   * Os periféricos, na ordem do JSON da placa.
   */
  peripherals: LayoutPeripheral[];
  /**
   * Os relógios da placa que chegam ao projeto.
   */
  clocks: LayoutClock[];
  /**
   * Os ajustes das ligações (larguras completadas ou cortadas, entradas
   * soltas), como no fluxo da placa.
   */
  notes: string[];
}
/**
 * Um periférico na placa virtual.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LayoutPeripheral".
 */
export interface LayoutPeripheral {
  /**
   * O identificador nas entradas e no estado: o nome do primeiro sinal do
   * periférico (`SW`, `HEX7`, `VGA_R`).
   */
  id: string;
  /**
   * Um periférico da placa: o tipo e os sinais dele. A placa virtual simula
   * cada um nos pinos, como a placa de verdade os liga ao FPGA.
   */
  peripheral:
    | {
        /**
         * O sinal de entrada.
         */
        signal: string;
        kind: 'switches';
      }
    | {
        /**
         * O sinal de entrada.
         */
        signal: string;
        kind: 'buttons';
      }
    | {
        /**
         * O sinal de saída.
         */
        signal: string;
        /**
         * A cor dos LEDs.
         */
        color: 'red' | 'green' | 'blue' | 'yellow';
        kind: 'leds';
      }
    | {
        /**
         * Os dígitos, da esquerda para a direita.
         */
        digits: string[];
        kind: 'seven_segment';
      }
    | {
        /**
         * O canal vermelho.
         */
        red: string;
        /**
         * O canal verde.
         */
        green: string;
        /**
         * O canal azul.
         */
        blue: string;
        /**
         * O sincronismo horizontal.
         */
        hsync: string;
        /**
         * O sincronismo vertical.
         */
        vsync: string;
        /**
         * O vídeo ativo (`VGA_BLANK_N`, `DE`), se a placa tem.
         */
        blank?: string | null;
        /**
         * O relógio de pixel que o projeto entrega ao conversor
         * (`VGA_CLK`), se a placa tem.
         */
        clock?: string | null;
        kind: 'vga';
      }
    | {
        /**
         * O relógio do teclado (entrada do FPGA).
         */
        clock: string;
        /**
         * O dado do teclado (entrada do FPGA).
         */
        data: string;
        kind: 'ps2';
      }
    | {
        /**
         * O que o projeto transmite (saída do FPGA).
         */
        tx: string;
        /**
         * O que o projeto recebe (entrada do FPGA), se a placa liga.
         */
        rx?: string | null;
        /**
         * A velocidade padrão, em bits por segundo.
         */
        baud: number;
        kind: 'uart';
      };
  /**
   * Algum sinal do periférico chega ao projeto. Um periférico solto
   * aparece apagado: o projeto não o usa.
   */
  connected: boolean;
  /**
   * Os sinais, com largura e polaridade.
   */
  signals: LayoutSignal[];
}
/**
 * Um sinal de um periférico.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LayoutSignal".
 */
export interface LayoutSignal {
  /**
   * O nome (`LEDR`).
   */
  name: string;
  /**
   * A largura em bits.
   */
  width: number;
  /**
   * Ativo em nível baixo na placa (os segmentos e os botões da Terasic):
   * o estado da placa virtual já vem convertido para aceso e apertado.
   */
  active_low: boolean;
  /**
   * O sinal chega ao projeto.
   */
  connected: boolean;
  /**
   * A porta do topo em cada bit, do bit 0 em diante (`b[3]`, `!rst`,
   * `a`); `None` no bit solto. Numa entrada da placa lida por mais de
   * uma porta, as portas separadas por vírgula.
   */
  ports: (string | null)[];
}
/**
 * Um relógio da placa que chega ao projeto.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LayoutClock".
 */
export interface LayoutClock {
  /**
   * O sinal (`CLOCK_50`).
   */
  signal: string;
  /**
   * A frequência, em MHz.
   */
  mhz: number;
}
/**
 * `vboard.run`.
 */
export interface BoardRun {
  build: BoardBuild;
  /**
   * Como a execução terminou; `None` se a compilação falhou.
   */
  outcome: SessionOutcome | null;
}
/**
 * A compilação.
 */
export interface BoardBuild {
  /**
   * Como terminou. `failed` é Verilog (ou ligação) que não compila.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * O passo que falhou, quando for o caso (`verilate`).
   */
  failed_step: Step | null;
  /**
   * Os passos rodados, com o comando e a saída.
   */
  steps: StepReport[];
  /**
   * As mensagens do YANC, do Verilator e do compilador.
   */
  diagnostics: Diagnostic[];
  /**
   * Os processadores SAPHO compilados antes da placa, os desatualizados
   * ([`crate::Processor::is_out_of_date`]); vazio quando todos estavam em
   * dia.
   */
  processors: BuildResult[];
  /**
   * A placa virtual do projeto; `None` quando um processador não compilou,
   * antes de o topo poder ser lido.
   */
  layout: BoardLayout1 | null;
  /**
   * O executável, se a compilação deu certo.
   */
  executable: string | null;
  /**
   * Quanto levou, em milissegundos.
   */
  duration_ms: number;
}
/**
 * A placa virtual de um projeto: a placa, o topo e cada periférico, ligado
 * ao projeto ou não, com as larguras. É o que uma interface desenha.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "BoardLayout".
 */
export interface BoardLayout1 {
  /**
   * A placa (`de2-115`).
   */
  board: string;
  /**
   * O nome da placa (`Terasic DE2-115`).
   */
  board_name: string;
  /**
   * O módulo do projeto que vai na placa.
   */
  top: string;
  /**
   * Os periféricos, na ordem do JSON da placa.
   */
  peripherals: LayoutPeripheral[];
  /**
   * Os relógios da placa que chegam ao projeto.
   */
  clocks: LayoutClock[];
  /**
   * Os ajustes das ligações (larguras completadas ou cortadas, entradas
   * soltas), como no fluxo da placa.
   */
  notes: string[];
}
/**
 * Como a placa virtual terminou.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SessionOutcome".
 */
export interface SessionOutcome {
  /**
   * Por que terminou.
   */
  reason: 'stopped' | 'finished' | 'crashed';
  /**
   * O tempo simulado no fim, em picossegundos.
   */
  time_ps: number;
  /**
   * Como o processo terminou.
   */
  termination:
    | {
        kind: 'exited';
        value: number;
      }
    | {
        kind: 'signaled';
        value: number;
      }
    | {
        kind: 'exception';
        value: number;
      }
    | {
        kind: 'cancelled';
      }
    | {
        kind: 'timed_out';
      }
    | {
        kind: 'unknown';
      };
  /**
   * Quanto rodou, em milissegundos.
   */
  duration_ms: number;
  /**
   * As últimas linhas do stderr, quando o processo quebrou.
   */
  tail: string[];
}
/**
 * `flow.run`.
 */
export interface FlowOutcome {
  /**
   * O fluxo ([`FlowRequest::name`]).
   */
  flow: string;
  /**
   * O comando equivalente ([`FlowOptions::command`]).
   */
  command: string;
  /**
   * Como terminou: o pior entre as fases que rodaram. `Cancelled` se o
   * cancelamento foi pedido; `TimedOut` se a simulação estourou o prazo;
   * `Failed` se alguma fase falhou (inclusive um build de `Build`, que
   * compila todos); senão `Succeeded`.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * `status == Succeeded`.
   */
  succeeded: boolean;
  /**
   * Os builds feitos antes (ou o próprio build).
   */
  builds: BuildResult[];
  /**
   * O que esses builds geraram e entrou no `.spf` agora.
   */
  registered: AddedFile[];
  /**
   * O resultado da verificação.
   */
  check: CheckResult | null;
  /**
   * Com `lint`, o componente do Verilator que falta quando nenhum passo
   * dele rodou: para a interface sugerir a instalação.
   */
  lint_missing: string | null;
  /**
   * A hierarquia.
   */
  hierarchy: HierarchyResult | null;
  /**
   * Na hierarquia sem processador: os processadores com fonte e sem
   * build, que ficaram de fora.
   */
  not_built: string[];
  /**
   * O resultado da simulação.
   */
  simulation: SimulationResult | null;
  /**
   * Os valores de cada porta de saída do processador simulado (só com
   * processador e sucesso).
   */
  outputs: FlowPortValues[];
  /**
   * O resultado da síntese.
   */
  synthesis: SynthesisResult | null;
  /**
   * O resultado do esquemático.
   */
  schematic: SchematicResult | null;
  /**
   * Por que o esquemático não foi desenhado depois da síntese (o módulo
   * pedido não está no netlist, `module_not_found`); a síntese vale.
   */
  schematic_error: ErrorInfo | null;
  /**
   * O HTML do RTL viewer, gravado ao lado do netlist.
   */
  rtl_viewer: string | null;
  /**
   * Por que o RTL viewer não saiu (a síntese vale).
   */
  rtl_viewer_error: ErrorInfo | null;
  /**
   * A onda aberta no surfer-aurora.
   */
  wave: WaveOpened | null;
  /**
   * Por que a onda não abriu (a simulação vale).
   */
  wave_error: ErrorInfo | null;
  /**
   * O relatório gravado no histórico (`run-000042`).
   */
  report: string | null;
  /**
   * Por que o relatório não foi gravado (o fluxo não falha por isso).
   */
  report_error: string | null;
  /**
   * A compilação para a placa.
   */
  fpga: FpgaBuildResult | null;
  /**
   * A gravação na placa.
   */
  fpga_program: FpgaProgramResult | null;
  /**
   * A correção de um exercício (`learn`): o `Grade` do aurora-learn, em
   * JSON (o core não conhece o tipo).
   */
  learn: {
    [k: string]: unknown | undefined;
  };
}
/**
 * O resultado de [`hierarchy`].
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "HierarchyResult".
 */
export interface HierarchyResult {
  /**
   * O design, sem testbench: os módulos que nenhum outro instancia (em
   * geral o topo), com tudo o que eles instanciam. `None` num projeto sem
   * Verilog de design.
   */
  design: Elaboration | null;
  /**
   * Cada testbench elaborado com o design: os registrados, na ordem do
   * `.spf`, e depois o testbench gerado de cada processador compilado.
   */
  testbenches: Elaboration[];
  /**
   * `Succeeded` se todas as elaborações deram certo; senão, como terminou
   * a primeira que não deu.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * `elaborate` quando uma elaboração falhou ou foi cancelada.
   */
  failed_step: Step | null;
  /**
   * Um `iverilog` para o design e um por testbench, na ordem.
   */
  steps: StepReport[];
  /**
   * Os erros e avisos de todas as elaborações, com arquivo e linha.
   */
  diagnostics: Diagnostic[];
  /**
   * Os `.vvp` de cada elaboração.
   */
  artifacts: Artifact[];
  /**
   * Quanto as elaborações levaram juntas, em milissegundos.
   */
  duration_ms: number;
}
/**
 * O resultado de [`simulate`] e de [`simulate_project`].
 *
 * Uma simulação que roda até o `$finish` e termina com código 0 é
 * `Succeeded`, mesmo que o circuito esteja errado: conferir o comportamento
 * é olhar `outputs` e a onda. O stdout do testbench (`$display`) está no
 * `stdout` do passo `simulate`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SimulationResult".
 */
export interface SimulationResult {
  /**
   * O módulo de topo simulado (o testbench).
   */
  top: string;
  /**
   * O simulador usado. Na simulação rápida de um testbench Verilog, o
   * Verilator, qualquer que fosse o pedido.
   */
  simulator: 'icarus' | 'verilator';
  /**
   * A simulação rápida ([`SimulationOptions::fast`]): rodou sem gravar
   * onda, e `waveform` é `null`.
   */
  fast: boolean;
  /**
   * Como a simulação terminou.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * O passo que falhou, quando for o caso: `elaborate` / `verilate`
   * (Verilog que não compila) ou `simulate` (a simulação em si).
   */
  failed_step: Step | null;
  /**
   * Icarus: `elaborate` e `simulate`. Verilator: `verilate` e `simulate`.
   */
  steps: StepReport[];
  /**
   * Mensagens de todos os passos. No passo `simulate`, só as linhas que o
   * parser reconhece: o resto do stdout é saída do testbench.
   */
  diagnostics: Diagnostic[];
  /**
   * O `.vvp` ou o executável do Verilator, a onda, e cada
   * `output_<n>.txt` escrito.
   */
  artifacts: Artifact[];
  /**
   * A onda gerada, se a simulação chegou ao fim; nunca na simulação
   * rápida.
   */
  waveform: Waveform | null;
  /**
   * `<proc>/<proc>/output_<n>.txt` escritos nesta simulação.
   */
  outputs: string[];
  /**
   * Arquivos que o testbench tenta ler e que não existem. Não impedem a
   * simulação (o testbench gerado pelo `asmcomp` confere o `$fopen` antes
   * de ler), mas a porta correspondente não recebe dado nenhum.
   */
  missing_inputs: string[];
  /**
   * Os testes de um testbench cocotb (`.py`), lidos do `results.xml`;
   * `null` num testbench Verilog, ou se o cocotb não chegou a gravar o
   * resultado.
   */
  tests: TestReport | null;
  /**
   * Quanto a simulação levou, do começo ao fim (preparar, compilar e
   * rodar), em milissegundos.
   */
  duration_ms: number;
}
/**
 * Uma onda gerada por simulação, pronta para
 * [`open_waveform`](crate::open_waveform).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Waveform".
 */
export interface Waveform {
  /**
   * Caminho absoluto, com o nome que o `$dumpfile` do testbench deu.
   */
  path: string;
  /**
   * O formato do conteúdo.
   */
  format: 'vcd' | 'fst';
}
/**
 * O resultado dos testes de um testbench cocotb, lido do `results.xml`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "TestReport".
 */
export interface TestReport {
  /**
   * O `results.xml` que o cocotb gravou.
   */
  results: string;
  /**
   * Os testes, na ordem em que rodaram.
   */
  cases: TestCase[];
  /**
   * Quantos passaram.
   */
  passed: number;
  /**
   * Quantos falharam.
   */
  failed: number;
  /**
   * Quantos não rodaram (`@cocotb.test(skip=True)`).
   */
  skipped: number;
}
/**
 * Um teste (`@cocotb.test()`).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "TestCase".
 */
export interface TestCase {
  /**
   * `<módulo>.<teste>`, como o cocotb mostra (`test_somador.basic_test`).
   */
  name: string;
  /**
   * Como terminou.
   */
  status: 'passed' | 'failed' | 'skipped';
  /**
   * Por que falhou (a mensagem da exceção) ou não rodou; `null` quando
   * passou.
   */
  message: string | null;
  /**
   * O `.py` do teste.
   */
  file: string | null;
  /**
   * Numa falha, a linha mais funda do traceback dentro do `.py`; senão, a
   * linha do teste.
   */
  line: number | null;
}
/**
 * Os valores de uma porta de saída de um processador simulado.
 *
 * No schema chama-se `FlowPortValues`: o `PortValues` do relatório `sim`
 * da CLI tem o erro em texto, e os dois convivem no mesmo conjunto de
 * tipos gerados.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FlowPortValues".
 */
export interface FlowPortValues {
  /**
   * O número da porta.
   */
  port: number;
  /**
   * `<proc>/<proc>/output_<porta>.txt`, na pasta de saída.
   */
  path: string;
  /**
   * Os valores, na ordem em que saíram. Vazio quando a porta não pôde
   * ser lida.
   */
  values: number[];
  /**
   * Por que a porta não pôde ser lida: uma linha que não é inteiro, como
   * o `x` que o simulador escreve numa divisão por zero. `None` quando
   * leu.
   */
  error: ErrorInfo | null;
}
/**
 * O resultado de [`synthesize`].
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SynthesisResult".
 */
export interface SynthesisResult {
  /**
   * O módulo de topo sintetizado.
   */
  top: string;
  /**
   * Como a síntese terminou.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * `synthesize` quando falhou.
   */
  failed_step: Step | null;
  /**
   * Um passo: o `yosys`. O log completo fica em `yosys.log`, ao lado do
   * netlist.
   */
  steps: StepReport[];
  /**
   * Os erros e avisos do Yosys, com arquivo e linha quando ele informa.
   */
  diagnostics: Diagnostic[];
  /**
   * O netlist (`hierarchy.json`).
   */
  artifacts: Artifact[];
  /**
   * `hierarchy.json`, quando a síntese terminou.
   */
  netlist: string | null;
  /**
   * Os módulos do netlist, na ordem do JSON, para escolher o que desenhar.
   */
  modules: string[];
  /**
   * O que o `stat` do Yosys contou no netlist, quando a síntese terminou
   * e o Yosys gravou o `stat.json`.
   */
  statistics: SynthesisStatistics | null;
  /**
   * Quanto a síntese levou, do começo ao fim, em milissegundos.
   */
  duration_ms: number;
}
/**
 * O que o `stat` do Yosys contou. Uma contagem que ele não informou fica
 * `null`, e é diferente de zero.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SynthesisStatistics".
 */
export interface SynthesisStatistics {
  /**
   * O Yosys que contou, como ele se identifica (`Yosys 0.69+156 (...)`).
   */
  tool: string;
  /**
   * O módulo de topo.
   */
  top: string;
  /**
   * Módulos.
   */
  modules: number | null;
  /**
   * Fios.
   */
  wires: number | null;
  /**
   * Bits de fio.
   */
  wire_bits: number | null;
  /**
   * Fios com nome do usuário.
   */
  public_wires: number | null;
  /**
   * Bits de fio com nome do usuário.
   */
  public_wire_bits: number | null;
  /**
   * Memórias.
   */
  memories: number | null;
  /**
   * Bits de memória.
   */
  memory_bits: number | null;
  /**
   * Processos.
   */
  processes: number | null;
  /**
   * Células, sem as instâncias de submódulo.
   */
  cells: number | null;
  /**
   * As células por tipo, da mais usada para a menos usada e, no empate,
   * pelo nome.
   */
  cell_types: CellUsage[];
}
/**
 * Quantas células de um tipo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "CellUsage".
 */
export interface CellUsage {
  /**
   * O tipo, como o Yosys o nomeia (`$add`, `$dff`).
   */
  cell_type: string;
  /**
   * Quantas.
   */
  count: number;
}
/**
 * O resultado de [`render_schematic`].
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SchematicResult".
 */
export interface SchematicResult {
  /**
   * O módulo desenhado.
   */
  module: string;
  /**
   * Como terminou. Uma mensagem de erro do `dot` conta como falha mesmo
   * que ele saia com código 0.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * `graph` (Yosys) ou `render` (`dot`) quando falhou.
   */
  failed_step: Step | null;
  /**
   * Dois passos: `graph` (`yosys show`) e `render` (`dot`).
   */
  steps: StepReport[];
  /**
   * Mensagens do Yosys e do Graphviz.
   */
  diagnostics: Diagnostic[];
  /**
   * O grafo (`.dot`) e o SVG.
   */
  artifacts: Artifact[];
  /**
   * O SVG, quando foi gerado. Fica ao lado do netlist, com o nome do
   * módulo (caracteres fora de `[A-Za-z0-9_.-]` viram `_`).
   */
  svg: string | null;
  /**
   * Quanto o desenho levou, do começo ao fim, em milissegundos.
   */
  duration_ms: number;
}
/**
 * A onda aberta no surfer-aurora, que continua aberto depois.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WaveOpened".
 */
export interface WaveOpened {
  /**
   * O arquivo da onda.
   */
  waveform: string;
  /**
   * O PID do surfer-aurora.
   */
  pid: number;
  /**
   * Onde vai o stdout e o stderr do surfer-aurora.
   */
  log: string;
  /**
   * O estado do Surfer com que a onda abriu (`.surf.ron` ou `.sucl`): o
   * layout salvo no projeto, o gerado, ou o de um exercício; `None` sem
   * layout.
   */
  layout: string | null;
  /**
   * O layout salvo no projeto (`wave/<testbench>.surf.ron`), onde o
   * Surfer salva (Ctrl+S); `None` numa onda fora de projeto ou sem layout
   * gerado.
   */
  saved_layout: SavedLayoutInfo | null;
  /**
   * Os processadores do layout gerado.
   */
  processors: WaveProcessor[];
  /**
   * A escolha de sinais que o layout gerado seguiu (`wave/<testbench>.json`);
   * vazia sem escolha ou sem layout gerado.
   */
  selection: string[];
  /**
   * Os processadores compilados de novo depois desta simulação: o PC e a
   * linha aparecem como números.
   */
  outdated: string[];
  /**
   * O layout gerado falhou e a onda abriu sem ele: o motivo.
   */
  layout_error: string | null;
  /**
   * Com [`LayoutPolicy::Reset`]: o testbench cujo layout salvo foi
   * apagado antes de abrir.
   */
  reset_deleted: string | null;
}
/**
 * O layout salvo no projeto para a onda aberta.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SavedLayoutInfo".
 */
export interface SavedLayoutInfo {
  /**
   * O arquivo.
   */
  path: string;
  /**
   * O usuário salvou depois que a AURORA o gerou: a AURORA não o refaz.
   */
  customized: boolean;
}
/**
 * O resultado de [`program`].
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FpgaProgramResult".
 */
export interface FpgaProgramResult {
  /**
   * A placa (o `id`).
   */
  board: string;
  /**
   * O `.sof` gravado.
   */
  bitstream: string;
  /**
   * O cabo usado, como o `quartus_pgm -l` o escreve
   * (`USB-Blaster [USB-0]`).
   */
  cable: string;
  /**
   * A posição da FPGA na cadeia JTAG (o `@n`).
   */
  position: number;
  quartus: Quartus2;
  /**
   * Como a gravação terminou.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * `program` quando falhou.
   */
  failed_step: Step | null;
  /**
   * Um passo: `program` (`quartus_pgm`).
   */
  steps: StepReport[];
  /**
   * Os erros e avisos do Programmer.
   */
  diagnostics: Diagnostic[];
  /**
   * Quanto a gravação levou, do começo ao fim, em milissegundos.
   */
  duration_ms: number;
}
/**
 * O Quartus que gravou.
 */
export interface Quartus2 {
  /**
   * A pasta `quartus` da instalação (`C:\intelFPGA_lite\22.1std\quartus`).
   */
  root: string;
  /**
   * A pasta dos programas: `bin64` no Windows, `bin` no Linux.
   */
  bin: string;
  /**
   * A versão, pelo nome da pasta da instalação (`22.1std`).
   */
  version?: string | null;
}
/**
 * A onda aberta no surfer-aurora, que continua aberto depois.
 */
export interface WaveOpened1 {
  /**
   * O arquivo da onda.
   */
  waveform: string;
  /**
   * O PID do surfer-aurora.
   */
  pid: number;
  /**
   * Onde vai o stdout e o stderr do surfer-aurora.
   */
  log: string;
  /**
   * O estado do Surfer com que a onda abriu (`.surf.ron` ou `.sucl`): o
   * layout salvo no projeto, o gerado, ou o de um exercício; `None` sem
   * layout.
   */
  layout: string | null;
  /**
   * O layout salvo no projeto (`wave/<testbench>.surf.ron`), onde o
   * Surfer salva (Ctrl+S); `None` numa onda fora de projeto ou sem layout
   * gerado.
   */
  saved_layout: SavedLayoutInfo | null;
  /**
   * Os processadores do layout gerado.
   */
  processors: WaveProcessor[];
  /**
   * A escolha de sinais que o layout gerado seguiu (`wave/<testbench>.json`);
   * vazia sem escolha ou sem layout gerado.
   */
  selection: string[];
  /**
   * Os processadores compilados de novo depois desta simulação: o PC e a
   * linha aparecem como números.
   */
  outdated: string[];
  /**
   * O layout gerado falhou e a onda abriu sem ele: o motivo.
   */
  layout_error: string | null;
  /**
   * Com [`LayoutPolicy::Reset`]: o testbench cujo layout salvo foi
   * apagado antes de abrir.
   */
  reset_deleted: string | null;
}
/**
 * `wave.signals`.
 */
export interface WaveSignals {
  /**
   * O testbench elaborado (o da simulação do projeto).
   */
  testbench: string;
  /**
   * O módulo do testbench: a raiz da árvore e o nome dos arquivos em
   * `wave/`.
   */
  module: string;
  /**
   * A árvore, a partir do módulo do testbench. `None` quando a elaboração
   * falhou (os erros em `diagnostics`).
   */
  root: SignalScope | null;
  /**
   * A escolha gravada; vazia: a simulação grava todos os sinais.
   */
  selection: string[];
  /**
   * Onde a escolha fica (`wave/<módulo>.json`), exista ou não.
   */
  selection_file: string;
  /**
   * Itens da escolha que a árvore não tem (um sinal renomeado, um módulo
   * que saiu): a simulação os deixa de fora.
   */
  unknown: string[];
  /**
   * Como a elaboração terminou.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * A elaboração (`iverilog`).
   */
  steps: StepReport[];
  /**
   * Os erros e avisos da elaboração.
   */
  diagnostics: Diagnostic[];
  /**
   * Quanto levou, em milissegundos.
   */
  duration_ms: number;
}
/**
 * Um escopo da árvore: uma instância de módulo, ou um bloco `generate` ou
 * `begin` com nome.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SignalScope".
 */
export interface SignalScope {
  /**
   * O nome (`dut`, `g[0]`).
   */
  name: string;
  /**
   * O caminho na hierarquia (`tb.dut`), como vai na escolha.
   */
  path: string;
  /**
   * Instância de módulo, bloco `generate` ou bloco com nome.
   */
  kind: 'module' | 'generate' | 'block';
  /**
   * O módulo, numa instância.
   */
  module: string | null;
  /**
   * É um processador SAPHO (tem `valr2` e `linetabs`): os sinais dele vão
   * para a onda com qualquer escolha.
   */
  processor: boolean;
  /**
   * Os sinais, na ordem do Icarus.
   */
  signals: WaveSignal[];
  /**
   * Os escopos de dentro.
   */
  scopes: SignalScope[];
}
/**
 * Um sinal da árvore.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WaveSignal".
 */
export interface WaveSignal {
  /**
   * O nome.
   */
  name: string;
  /**
   * O caminho na hierarquia (`tb.dut.q`).
   */
  path: string;
  /**
   * Bits (64 num `real`).
   */
  width: number;
  /**
   * `reg`, `wire`, `integer` ou `real`.
   */
  kind: 'reg' | 'wire' | 'integer' | 'real';
  /**
   * A direção, quando o sinal é uma porta do módulo.
   */
  direction: PortDirection | null;
}
/**
 * `wave.choose`.
 */
export interface WaveChoiceOutcome {
  /**
   * O módulo do testbench do projeto.
   */
  testbench: string;
  /**
   * O arquivo da escolha (`wave/<testbench>.json`); `None` quando ela
   * ficou vazia e o arquivo saiu: a onda grava todos os sinais.
   */
  file: string | null;
  /**
   * A escolha; vazia, todos os sinais.
   */
  selection: string[];
}
/**
 * Uma linha de [`list`].
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "RunSummary".
 */
export interface RunSummary {
  /**
   * O identificador.
   */
  id: string;
  /**
   * Quando terminou; `null` num relatório ilegível.
   */
  timestamp: string | null;
  /**
   * O comando.
   */
  command: string | null;
  /**
   * Como terminou.
   */
  status: Status | null;
  /**
   * O módulo de topo.
   */
  top: string | null;
  /**
   * Tem estatísticas de síntese.
   */
  synthesis: boolean;
  /**
   * Quanto dos tempos de simulação tem.
   */
  simulation: 'available' | 'partial' | 'unavailable';
  /**
   * O `record.json` pôde ser lido.
   */
  readable: boolean;
}
/**
 * `history.show`.
 */
export interface ReportView {
  /**
   * O identificador (`run-000042`).
   */
  id: string;
  /**
   * O `report.txt`.
   */
  path: string;
  record: RunRecord;
  /**
   * O `report.txt`, sem mudança.
   */
  text: string;
}
/**
 * O que a comparação usa.
 */
export interface RunRecord {
  /**
   * O formato ([`RECORD_SCHEMA`]).
   */
  schema: number;
  /**
   * O identificador (`run-000042`).
   */
  id: string;
  metadata: RunMetadata;
  /**
   * As estatísticas do Yosys, quando houve síntese e o Yosys as gravou.
   */
  synthesis: SynthesisStatistics | null;
  /**
   * Os tempos da simulação, quando houve simulação.
   */
  simulation: SimulationTimings | null;
}
/**
 * O contexto da operação.
 */
export interface RunMetadata {
  /**
   * Quando terminou, em UTC (`2026-10-03T21:04:05Z`).
   */
  timestamp: string;
  /**
   * O comando.
   */
  command: string;
  /**
   * Como uma operação terminou. Em JSON, em `snake_case` (`"succeeded"`).
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * Quanto levou, do começo ao fim, em milissegundos.
   */
  duration_ms: number;
  /**
   * A versão que gravou. `lace_version` nos relatórios gravados pelo
   * Lace, que a `.lace` levada para a `.aurora8` traz.
   */
  aurora_version: string;
  /**
   * O que identifica o projeto: um hash da raiz dele. Relatórios de
   * projetos diferentes não se comparam.
   */
  project: string;
  /**
   * O nome do projeto.
   */
  project_name: string;
  /**
   * O bundle de ferramentas (`2026.09.29`) e a plataforma.
   */
  bundle: string;
  /**
   * `linux-x64`, `darwin-arm64`, `windows-x64`.
   */
  platform: string;
  /**
   * O módulo de topo: o da síntese, ou o testbench simulado, ou o topo do
   * projeto.
   */
  top: string | null;
  /**
   * O contexto da síntese, quando houve.
   */
  synthesis: SynthesisContext | null;
  /**
   * O contexto da simulação, quando houve.
   */
  simulation: SimulationContext | null;
  environment: Environment;
}
/**
 * O que decide se duas sínteses se comparam, e o que vira aviso.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SynthesisContext".
 */
export interface SynthesisContext {
  /**
   * O módulo de topo.
   */
  top: string;
  /**
   * A versão do componente yosys do bundle.
   */
  component_version: string | null;
  /**
   * Hash dos fontes sintetizáveis e dos programas dos processadores.
   */
  sources: string;
}
/**
 * O que decide se duas simulações se comparam, e o que vira aviso.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SimulationContext".
 */
export interface SimulationContext {
  /**
   * O simulador.
   */
  simulator: 'icarus' | 'verilator';
  /**
   * O testbench (o módulo de topo simulado).
   */
  testbench: string;
  /**
   * A versão do componente do simulador no bundle.
   */
  component_version: string | null;
  /**
   * Hash dos fontes, com os testbenches, e dos programas dos processadores.
   */
  sources: string;
  /**
   * Hash das entradas dos processadores (`<proc>/input_*.txt`, ao lado
   * do fonte).
   */
  inputs: string;
  /**
   * A simulação gravou onda.
   */
  waveform: boolean;
}
/**
 * A máquina.
 */
export interface Environment {
  /**
   * O nome da máquina.
   */
  hostname: string | null;
  /**
   * `linux`, `macos`, `windows`.
   */
  os: string;
  /**
   * O nome da distribuição (Linux).
   */
  os_name: string | null;
  /**
   * A versão do kernel (Linux e macOS).
   */
  kernel: string | null;
  /**
   * `x86_64`, `aarch64`.
   */
  arch: string;
  /**
   * O modelo do processador (Linux).
   */
  cpu_model: string | null;
  /**
   * Processadores lógicos disponíveis.
   */
  cpus: number | null;
  /**
   * A memória total, em bytes (Linux).
   */
  memory_bytes: number | null;
  /**
   * Hash do sistema, da arquitetura, do processador, do número de
   * processadores e do nome da máquina: muda quando a máquina muda.
   */
  fingerprint: string;
}
/**
 * Os tempos de uma simulação, em milissegundos de relógio. O tempo simulado
 * é outra grandeza: o quanto o tempo andou dentro do modelo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SimulationTimings".
 */
export interface SimulationTimings {
  /**
   * A simulação terminou com sucesso.
   */
  succeeded: boolean;
  /**
   * O passo que compila o Verilog (`iverilog` ou `verilator --binary`).
   */
  compile_ms: number | null;
  /**
   * O passo que roda o testbench (`vvp` ou o modelo do Verilator).
   */
  execution_ms: number | null;
  /**
   * A simulação inteira, medida à parte: com a preparação da AURORA.
   */
  total_ms: number | null;
  /**
   * O tempo simulado no `$finish`, em femtossegundos, quando o simulador
   * o informa (o `vvp` informa; o Verilator, não).
   */
  simulated_fs: number | null;
}
/**
 * `history.compare`.
 */
export interface RunComparison {
  /**
   * O relatório atual.
   */
  current_id: string;
  current: RunMetadata1;
  /**
   * A referência.
   */
  baseline_id: string;
  baseline: RunMetadata2;
  /**
   * A síntese comparada; `null` se os dois não têm estatísticas do mesmo
   * topo.
   */
  synthesis: SynthesisComparison | null;
  /**
   * A simulação comparada; `null` se os dois não têm tempos do mesmo
   * simulador e testbench.
   */
  simulation: TimingComparison | null;
  /**
   * O que difere no contexto e pode explicar uma mudança (fontes,
   * versões, máquina), em inglês, sem ponto final.
   */
  warnings: string[];
}
/**
 * O contexto dele.
 */
export interface RunMetadata1 {
  /**
   * Quando terminou, em UTC (`2026-10-03T21:04:05Z`).
   */
  timestamp: string;
  /**
   * O comando.
   */
  command: string;
  /**
   * Como uma operação terminou. Em JSON, em `snake_case` (`"succeeded"`).
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * Quanto levou, do começo ao fim, em milissegundos.
   */
  duration_ms: number;
  /**
   * A versão que gravou. `lace_version` nos relatórios gravados pelo
   * Lace, que a `.lace` levada para a `.aurora8` traz.
   */
  aurora_version: string;
  /**
   * O que identifica o projeto: um hash da raiz dele. Relatórios de
   * projetos diferentes não se comparam.
   */
  project: string;
  /**
   * O nome do projeto.
   */
  project_name: string;
  /**
   * O bundle de ferramentas (`2026.09.29`) e a plataforma.
   */
  bundle: string;
  /**
   * `linux-x64`, `darwin-arm64`, `windows-x64`.
   */
  platform: string;
  /**
   * O módulo de topo: o da síntese, ou o testbench simulado, ou o topo do
   * projeto.
   */
  top: string | null;
  /**
   * O contexto da síntese, quando houve.
   */
  synthesis: SynthesisContext | null;
  /**
   * O contexto da simulação, quando houve.
   */
  simulation: SimulationContext | null;
  environment: Environment;
}
/**
 * O contexto dela.
 */
export interface RunMetadata2 {
  /**
   * Quando terminou, em UTC (`2026-10-03T21:04:05Z`).
   */
  timestamp: string;
  /**
   * O comando.
   */
  command: string;
  /**
   * Como uma operação terminou. Em JSON, em `snake_case` (`"succeeded"`).
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * Quanto levou, do começo ao fim, em milissegundos.
   */
  duration_ms: number;
  /**
   * A versão que gravou. `lace_version` nos relatórios gravados pelo
   * Lace, que a `.lace` levada para a `.aurora8` traz.
   */
  aurora_version: string;
  /**
   * O que identifica o projeto: um hash da raiz dele. Relatórios de
   * projetos diferentes não se comparam.
   */
  project: string;
  /**
   * O nome do projeto.
   */
  project_name: string;
  /**
   * O bundle de ferramentas (`2026.09.29`) e a plataforma.
   */
  bundle: string;
  /**
   * `linux-x64`, `darwin-arm64`, `windows-x64`.
   */
  platform: string;
  /**
   * O módulo de topo: o da síntese, ou o testbench simulado, ou o topo do
   * projeto.
   */
  top: string | null;
  /**
   * O contexto da síntese, quando houve.
   */
  synthesis: SynthesisContext | null;
  /**
   * O contexto da simulação, quando houve.
   */
  simulation: SimulationContext | null;
  environment: Environment;
}
/**
 * As estatísticas de síntese, comparadas.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SynthesisComparison".
 */
export interface SynthesisComparison {
  /**
   * As contagens do resumo, na ordem do relatório.
   */
  metrics: SynthesisMetricComparison[];
  /**
   * Os tipos de célula, da maior mudança para a menor e, no empate, pelo
   * nome.
   */
  cell_types: CellComparison[];
  /**
   * Tipos que só existem no atual.
   */
  added_cell_types: number;
  /**
   * Tipos que só existem na referência.
   */
  removed_cell_types: number;
  /**
   * Contagens do resumo que aumentaram.
   */
  increased: number;
  /**
   * Que diminuíram.
   */
  decreased: number;
  /**
   * Iguais.
   */
  unchanged: number;
  /**
   * Sem um dos lados.
   */
  not_comparable: number;
}
/**
 * Uma contagem do resumo, comparada.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SynthesisMetricComparison".
 */
export interface SynthesisMetricComparison {
  /**
   * Qual.
   */
  metric:
    | 'modules'
    | 'wires'
    | 'wire_bits'
    | 'public_wires'
    | 'public_wire_bits'
    | 'memories'
    | 'memory_bits'
    | 'processes'
    | 'cells';
  comparison: MetricComparison;
}
/**
 * A comparação.
 */
export interface MetricComparison {
  /**
   * Na referência.
   */
  baseline: number | null;
  /**
   * No atual.
   */
  current: number | null;
  /**
   * O que mudou.
   */
  change: 'unchanged' | 'increased' | 'decreased' | 'added' | 'removed' | 'not_comparable';
  /**
   * Atual menos referência.
   */
  delta: number | null;
  /**
   * A variação em porcentagem da referência. `null` quando não se
   * compara ou quando a referência é zero e o atual não (não há
   * porcentagem de zero; o texto mostra `new`).
   */
  percent: number | null;
}
/**
 * Um tipo de célula, comparado.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "CellComparison".
 */
export interface CellComparison {
  /**
   * O tipo (`$add`).
   */
  cell_type: string;
  usage: MetricComparison1;
}
/**
 * A comparação. Um tipo que só existe de um lado conta como 0 do outro,
 * com `added` ou `removed`.
 */
export interface MetricComparison1 {
  /**
   * Na referência.
   */
  baseline: number | null;
  /**
   * No atual.
   */
  current: number | null;
  /**
   * O que mudou.
   */
  change: 'unchanged' | 'increased' | 'decreased' | 'added' | 'removed' | 'not_comparable';
  /**
   * Atual menos referência.
   */
  delta: number | null;
  /**
   * A variação em porcentagem da referência. `null` quando não se
   * compara ou quando a referência é zero e o atual não (não há
   * porcentagem de zero; o texto mostra `new`).
   */
  percent: number | null;
}
/**
 * Os tempos de simulação, comparados.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "TimingComparison".
 */
export interface TimingComparison {
  compile: MetricComparison2;
  execution: MetricComparison3;
  total: MetricComparison4;
  simulated: MetricComparison5;
  /**
   * As duas rodaram em máquinas diferentes.
   */
  environment_changed: boolean;
}
/**
 * Compilação, em milissegundos.
 */
export interface MetricComparison2 {
  /**
   * Na referência.
   */
  baseline: number | null;
  /**
   * No atual.
   */
  current: number | null;
  /**
   * O que mudou.
   */
  change: 'unchanged' | 'increased' | 'decreased' | 'added' | 'removed' | 'not_comparable';
  /**
   * Atual menos referência.
   */
  delta: number | null;
  /**
   * A variação em porcentagem da referência. `null` quando não se
   * compara ou quando a referência é zero e o atual não (não há
   * porcentagem de zero; o texto mostra `new`).
   */
  percent: number | null;
}
/**
 * Execução, em milissegundos.
 */
export interface MetricComparison3 {
  /**
   * Na referência.
   */
  baseline: number | null;
  /**
   * No atual.
   */
  current: number | null;
  /**
   * O que mudou.
   */
  change: 'unchanged' | 'increased' | 'decreased' | 'added' | 'removed' | 'not_comparable';
  /**
   * Atual menos referência.
   */
  delta: number | null;
  /**
   * A variação em porcentagem da referência. `null` quando não se
   * compara ou quando a referência é zero e o atual não (não há
   * porcentagem de zero; o texto mostra `new`).
   */
  percent: number | null;
}
/**
 * A simulação inteira, em milissegundos.
 */
export interface MetricComparison4 {
  /**
   * Na referência.
   */
  baseline: number | null;
  /**
   * No atual.
   */
  current: number | null;
  /**
   * O que mudou.
   */
  change: 'unchanged' | 'increased' | 'decreased' | 'added' | 'removed' | 'not_comparable';
  /**
   * Atual menos referência.
   */
  delta: number | null;
  /**
   * A variação em porcentagem da referência. `null` quando não se
   * compara ou quando a referência é zero e o atual não (não há
   * porcentagem de zero; o texto mostra `new`).
   */
  percent: number | null;
}
/**
 * O tempo simulado, em femtossegundos.
 */
export interface MetricComparison5 {
  /**
   * Na referência.
   */
  baseline: number | null;
  /**
   * No atual.
   */
  current: number | null;
  /**
   * O que mudou.
   */
  change: 'unchanged' | 'increased' | 'decreased' | 'added' | 'removed' | 'not_comparable';
  /**
   * Atual menos referência.
   */
  delta: number | null;
  /**
   * A variação em porcentagem da referência. `null` quando não se
   * compara ou quando a referência é zero e o atual não (não há
   * porcentagem de zero; o texto mostra `new`).
   */
  percent: number | null;
}
/**
 * `history.clean`.
 */
export interface CleanOutcome {
  /**
   * Os apagados, do mais antigo para o mais novo.
   */
  removed: string[];
  /**
   * Quantos ficaram.
   */
  kept: number;
}
/**
 * `toolchain.info`.
 */
export interface ToolchainInfo {
  /**
   * O bundle abriu.
   */
  found: boolean;
  /**
   * Por que não abriu.
   */
  error: ErrorInfo | null;
  /**
   * Como foi achado.
   */
  origin: Origin | null;
  /**
   * A pasta do bundle.
   */
  root: string | null;
  /**
   * A versão do bundle.
   */
  bundle: string | null;
  /**
   * A plataforma do bundle (`linux-x64`, `darwin-arm64`, `windows-x64`).
   */
  platform: string | null;
  /**
   * Os componentes instalados.
   */
  components: BundleComponent[];
  /**
   * Os componentes que a AURORA conhece e que não estão instalados.
   */
  not_installed: string[];
  /**
   * Cada ferramenta que a AURORA roda.
   */
  tools: ToolStatus[];
  /**
   * O compilador do Verilator: do sistema no Linux e no macOS, do bundle
   * no Windows (`bundled`).
   */
  system_compiler: SystemCompiler | null;
  /**
   * O compilador declarado não serve.
   */
  compiler_error: ErrorInfo | null;
  /**
   * O Quartus Prime do sistema, se encontrado.
   */
  quartus: Quartus1 | null;
  /**
   * A pasta declarada do Quartus não serve.
   */
  quartus_error: ErrorInfo | null;
  /**
   * A CLI `aurora` da instalação.
   */
  aurora_cli: string | null;
  /**
   * O cliente web do Surfer (`surfer-aurora/web`), para a onda numa aba;
   * `None` num bundle que não o traz.
   */
  surfer_web: string | null;
}
/**
 * Um componente instalado (`components/<nome>.json`).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "BundleComponent".
 */
export interface BundleComponent {
  /**
   * Nome (ver o módulo [`component`]); é também o nome do arquivo.
   */
  name: string;
  /**
   * Versão exata: data da release do OSS CAD Suite, tag do YANC, do
   * surfer-aurora, do Graphviz.
   */
  version: string;
  /**
   * Diretório do componente, relativo ao bundle. Componentes do OSS CAD
   * Suite dividem o mesmo.
   */
  dir: string;
  /**
   * De onde veio: URL do pacote, ou `git+<repositório>@<commit>` quando o
   * empacotamento compilou do fonte.
   */
  source: string;
  /**
   * SHA-256 do pacote de origem, quando foi baixado pronto.
   */
  sha256: string | null;
  /**
   * SHA-256 dos executáveis do componente que a AURORA roda, por caminho
   * relativo ao bundle, conferido por [`Toolchain::verify`].
   */
  files: {
    [k: string]: string | undefined;
  };
}
/**
 * Uma ferramenta do bundle: onde está, ou por que não está.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ToolStatus".
 */
export interface ToolStatus {
  /**
   * O nome do executável (`iverilog`, `cmmcomp`).
   */
  name: string;
  /**
   * O componente que a traz; `None` para o Perl do sistema e o Quartus.
   */
  component: string | null;
  /**
   * O executável, quando disponível.
   */
  path: string | null;
  /**
   * Vem do sistema, não do bundle: o Perl do Verilator no Linux e no
   * macOS (no Windows ele vem no bundle) e o Quartus.
   */
  system: boolean;
  /**
   * Por que não está disponível.
   */
  error: ErrorInfo | null;
}
/**
 * O compilador C++, o `make` e o Perl que o Verilator usa.
 *
 * No Windows vêm do bundle, no componente verilator (`msys/ucrt64/bin` e
 * `msys/usr/bin`), e [`SystemCompiler::bundled`] é verdadeiro. No Linux e no
 * macOS vêm do sistema, a exceção à regra do bundle, procurados em locais
 * fixos ([`SystemCompiler::detect`]), nunca no `PATH`: `/usr/bin` no Linux;
 * `/usr/bin` com as Command Line Tools do Xcode no macOS. Ou num diretório
 * declarado ([`SystemCompiler::in_dir`], e [`SystemCompiler::in_msys2`]
 * para um MSYS2 no Windows), que substitui o padrão.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SystemCompiler".
 */
export interface SystemCompiler {
  /**
   * O Perl que executa o script `verilator`.
   */
  perl: string;
  /**
   * O `make` que o Verilator chama.
   */
  make: string;
  /**
   * O compilador C++ que o `make` chama.
   */
  cxx: string;
  /**
   * Os diretórios postos no `PATH` do Verilator para ele achar `make` e o
   * compilador.
   */
  path: string[];
  /**
   * Veio do bundle (Windows), e não do sistema.
   */
  bundled: boolean;
}
/**
 * Um executável do bundle cujo hash não bate com o manifesto.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FileMismatch".
 */
export interface FileMismatch {
  /**
   * O componente que lista o arquivo.
   */
  component: string;
  /**
   * Caminho relativo ao bundle.
   */
  path: string;
  /**
   * O hash do manifesto.
   */
  expected: string;
  /**
   * O hash do arquivo, ou `None` se ele não existe.
   */
  actual: string | null;
}
/**
 * `experimental.list`, `experimental.set`.
 */
export interface ExperimentalInfo {
  /**
   * O `experimental.json`; `None` sem pasta de configuração.
   */
  path: string | null;
  /**
   * Cada recurso e se está ligado.
   */
  components: FeatureState[];
}
/**
 * O que o `fpga.json` guarda.
 */
export interface FpgaConfig1 {
  /**
   * A placa ([`super::board()`]).
   */
  board: string;
  /**
   * O módulo que vai para a placa. Sem ele, o topo do projeto.
   */
  top?: string | null;
  /**
   * As ligações, na ordem do arquivo. Uma porta com vários sinais (uma
   * saída que vai a mais de um lugar da placa) tem uma [`Link`] para cada.
   */
  connect?: {
    [k: string]: LinkTargets | undefined;
  };
}
/**
 * `fpga.check`.
 */
export interface Prepared {
  /**
   * O `fpga.json`.
   */
  config: string;
  resolved: Resolved1;
  /**
   * O Verilog do topo da placa ([`wrapper::generate`]).
   */
  board_top: string;
  /**
   * Cada ligação como texto ([`wrapper::connections`]): cada entrada do
   * topo e cada sinal de saída da placa usado, com o que o dirige.
   */
  connections: Connection[];
}
/**
 * As ligações conferidas.
 */
export interface Resolved1 {
  board: Board1;
  /**
   * O módulo do topo.
   */
  top: string;
  /**
   * As portas do topo, na ordem da declaração.
   */
  ports: TopPort[];
  /**
   * As entradas do topo, com a origem de cada bit, do bit 0 em diante.
   */
  inputs: PortDrive[];
  /**
   * Os sinais de saída da placa usados, com a porta que dirige cada bit.
   */
  board_outputs: SignalDrive[];
  /**
   * Os sinais de entrada da placa usados, na ordem em que aparecem.
   */
  board_inputs: string[];
  /**
   * As portas ligadas a um clock da placa.
   */
  clocks: Clock[];
  /**
   * Os ajustes feitos: larguras completadas ou cortadas, entradas soltas.
   */
  notes: string[];
}
/**
 * `fpga.status`.
 */
export interface BitstreamState {
  /**
   * A placa (o `id`).
   */
  board: string;
  /**
   * O `.sof`, se a placa já foi compilada.
   */
  bitstream: string | null;
  /**
   * Quando o `.sof` foi gravado, em milissegundos desde 1970.
   */
  built_at_ms: number | null;
  /**
   * Por que o `.sof` não serve para gravar, uma razão por item: vazio
   * com `bitstream`, ele descreve o projeto de agora.
   */
  reasons: string[];
}
/**
 * `update.check`, `update.apply`.
 */
export interface UpdateReport {
  aurora: AuroraVersions;
  bundle: BundleVersions;
  /**
   * Cada componente instalado, na ordem do manifesto do bundle. Vazio sem
   * bundle.
   */
  components: ComponentVersions[];
  /**
   * A pasta da instalação; `null` se este `aurora` não foi instalado pelo
   * instalador (um build em `target/`, por exemplo).
   */
  prefix: string | null;
  /**
   * O que o comando fez.
   */
  action: 'checked' | 'up_to_date' | 'updated' | 'wizard_opened';
  /**
   * Como atualizou; `null` quando não atualizou.
   */
  method: UpdateMethod | null;
  /**
   * O que a atualização por componentes trocou, ou vai trocar com
   * `--check --from`; `null` quando ela não rodou.
   */
  changes: UpdateChanges | null;
}
/**
 * A AURORA: este e o da última release.
 */
export interface AuroraVersions {
  /**
   * A versão deste `aurora`.
   */
  installed: string;
  /**
   * A versão da última release no GitHub.
   */
  latest: string;
  /**
   * `latest` é mais nova que `installed`.
   */
  newer: boolean;
}
/**
 * O bundle de ferramentas: o instalado e o da última release.
 */
export interface BundleVersions {
  /**
   * O bundle instalado; `null` sem bundle.
   */
  installed: string | null;
  /**
   * O bundle da última release; `null` se o `bundle/versions.json` dela
   * não pôde ser lido.
   */
  latest: string | null;
  /**
   * `latest` é mais novo que `installed`.
   */
  newer: boolean;
}
/**
 * Um componente instalado em `aurora update`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ComponentVersions".
 */
export interface ComponentVersions {
  /**
   * O componente (`yanc`, `icarus`, `verilator`, `yosys`, `graphviz`,
   * `surfer-aurora`).
   */
  name: string;
  /**
   * O pacote de `bundle/versions.json` de onde ele sai (`oss-cad-suite`,
   * `yanc`, `surfer-aurora`, `graphviz`); `null` para um componente que
   * esta AURORA não conhece.
   */
  package: string | null;
  /**
   * A versão instalada.
   */
  installed: string;
  /**
   * A versão no bundle da última release; `null` se não pôde ser lida.
   */
  release: string | null;
  /**
   * `release` é mais nova que `installed`: chega atualizando a AURORA.
   */
  release_newer: boolean;
  /**
   * A última versão upstream; `null` se a consulta falhou.
   */
  upstream: string | null;
  /**
   * `upstream` é mais nova que `installed` e que `release`: só chega num
   * bundle novo, numa release nova da AURORA. Sem `release`, `false` quando
   * há AURORA mais nova, cujo bundle pode já trazê-la.
   */
  upstream_newer: boolean;
}
/**
 * O que a atualização por componentes trocou.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "UpdateChanges".
 */
export interface UpdateChanges {
  /**
   * Os componentes com arquivos que entraram ou saíram, na ordem do
   * bundle; `aurora` é a própria AURORA e o cabeçalho do bundle.
   */
  components: string[];
  /**
   * Bytes baixados.
   */
  download_bytes: number;
  /**
   * Arquivos que entraram (novos ou trocados).
   */
  files: number;
  /**
   * Arquivos que saíram.
   */
  removed: number;
}
/**
 * `install.add`.
 */
export interface InstallReport {
  /**
   * A pasta da instalação.
   */
  prefix: string;
  /**
   * Os aplicativos do bundle instalados agora.
   */
  components: string[];
  /**
   * Os que entraram nesta chamada (com os que eles exigem).
   */
  added: string[];
}
/**
 * `learn.check`.
 */
export interface LearnCheckOutcome {
  /**
   * A correção de cada exercício pedido, na ordem; termina no primeiro
   * cancelado.
   */
  results: Grade[];
  /**
   * Quantos exercícios da trilha estão resolvidos depois da rodada.
   */
  solved: number;
  /**
   * Quantos exercícios a trilha tem.
   */
  total: number;
}
/**
 * `learn.tracks`.
 */
export interface LearnTracks {
  /**
   * A pasta das trilhas.
   */
  dir: string;
  /**
   * As trilhas dela.
   */
  tracks: string[];
}
/**
 * `learn.status`, `learn.init`, `learn.set_current`.
 */
export interface WorkspaceSnapshot {
  /**
   * A pasta de exercícios.
   */
  root: string;
  /**
   * A trilha (`verilog`).
   */
  track: string;
  /**
   * O título da trilha.
   */
  title: string;
  /**
   * As boas-vindas, em markdown.
   */
  welcome: string;
  /**
   * A mensagem do fim, em markdown.
   */
  farewell: string | null;
  /**
   * Os capítulos, na ordem.
   */
  chapters: ChapterSnapshot[];
  /**
   * O exercício atual.
   */
  current: string;
  /**
   * Quantos resolvidos.
   */
  solved: number;
  /**
   * Quantos exercícios.
   */
  total: number;
}
/**
 * Um capítulo de [`WorkspaceSnapshot`].
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ChapterSnapshot".
 */
export interface ChapterSnapshot {
  /**
   * A pasta do capítulo (`01_primeiros_passos`).
   */
  id: string;
  /**
   * O título.
   */
  title: string;
  /**
   * A introdução, em markdown.
   */
  intro: string;
  /**
   * Os exercícios, na ordem.
   */
  exercises: ExerciseSnapshot[];
}
/**
 * Um exercício de [`WorkspaceSnapshot`], com o que o aluno já fez.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ExerciseSnapshot".
 */
export interface ExerciseSnapshot {
  /**
   * O nome.
   */
  name: string;
  /**
   * O título.
   */
  title: string;
  /**
   * O enunciado, em markdown.
   */
  prompt: string;
  /**
   * As dicas, em markdown.
   */
  hints: string[];
  /**
   * O módulo que o aluno escreve.
   */
  module: string;
  /**
   * O arquivo que o aluno edita.
   */
  file: string;
  /**
   * O `.spf` do projeto do exercício.
   */
  spf: string;
  /**
   * A onda da última correção.
   */
  waveform: string;
  /**
   * Resolvido.
   */
  solved: boolean;
  /**
   * A solução liberada, depois de resolvido.
   */
  solution: string | null;
}
/**
 * `git.status`.
 */
export interface GitStatus {
  /**
   * A versão do git (`2.54.0.windows.1`).
   */
  version: string;
  /**
   * O repositório que contém o projeto; `None` fora de repositório.
   */
  repository: RepositoryStatus | null;
}
/**
 * O estado de um repositório.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "RepositoryStatus".
 */
export interface RepositoryStatus {
  /**
   * A pasta de topo do repositório, com `/`.
   */
  top: string;
  /**
   * A pasta do git (`.git`, ou a de um worktree), com `/`: é ela que muda
   * num commit ou numa troca de ramo feitos fora da AURORA.
   */
  git_dir: string;
  /**
   * A pasta do projeto relativa ao topo, com `/` e sem barra no fim;
   * vazia quando o projeto é o topo.
   */
  prefix: string;
  branch: BranchState;
  /**
   * Os arquivos com mudança em relação ao último commit, primeiro os de
   * dentro da pasta do projeto.
   */
  files: ChangedFile[];
  /**
   * A lista parou em [`MAX_FILES`](crate::MAX_FILES): há mais arquivos
   * mudados do que os listados (falta um `.gitignore`, em geral).
   */
  truncated: boolean;
  /**
   * Quem assina os commits, pela configuração do git; `None` sem nome ou
   * sem e-mail.
   */
  identity: Identity | null;
  /**
   * Os remotos.
   */
  remotes: Remote[];
  /**
   * Uma operação pela metade (um merge com conflito, por exemplo).
   */
  operation: Operation | null;
}
/**
 * O ramo atual.
 */
export interface BranchState {
  /**
   * O nome; `None` com o HEAD fora de ramo.
   */
  name: string | null;
  /**
   * O commit atual; `None` num repositório ainda sem commit.
   */
  commit: string | null;
  /**
   * O ramo remoto que ele acompanha (`origin/main`); `None` se ainda não
   * foi publicado.
   */
  upstream: string | null;
  /**
   * Commits locais que o remoto não tem.
   */
  ahead: number;
  /**
   * Commits do remoto que o local não tem (pelo último fetch).
   */
  behind: number;
  /**
   * O remoto para onde o ramo vai ao publicar: o configurado no ramo, o
   * `origin` ou o único que houver.
   */
  remote: string | null;
}
/**
 * Um arquivo com mudança.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ChangedFile".
 */
export interface ChangedFile {
  /**
   * O caminho relativo ao topo do repositório, com `/`.
   */
  path: string;
  /**
   * O caminho anterior, num arquivo renomeado.
   */
  original: string | null;
  /**
   * A mudança em relação ao último commit, juntando o que já foi
   * preparado (o índice) e o que não foi.
   */
  change:
    | 'added'
    | 'untracked'
    | 'modified'
    | 'deleted'
    | 'renamed'
    | 'copied'
    | 'type_changed'
    | 'conflicted';
  /**
   * Tem mudança preparada no índice (por um `git add` no terminal).
   */
  staged: boolean;
  /**
   * Tem mudança ainda não preparada.
   */
  unstaged: boolean;
  /**
   * Está dentro da pasta do projeto.
   */
  in_project: boolean;
}
/**
 * Nome e e-mail de quem faz o commit.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Identity".
 */
export interface Identity {
  /**
   * O nome.
   */
  name: string;
  /**
   * O e-mail.
   */
  email: string;
}
/**
 * Um remoto.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Remote".
 */
export interface Remote {
  /**
   * O nome (`origin`).
   */
  name: string;
  /**
   * O endereço, sem usuário nem senha embutidos.
   */
  url: string;
  /**
   * O repositório do GitHub, quando o remoto é do GitHub.
   */
  github: GitHubRepository | null;
}
/**
 * Um repositório do GitHub.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "GitHubRepository".
 */
export interface GitHubRepository {
  /**
   * O dono (usuário ou organização).
   */
  owner: string;
  /**
   * O nome.
   */
  name: string;
}
/**
 * `git.read_file`.
 */
export interface FileVersion {
  /**
   * O arquivo existe nessa versão.
   */
  exists: boolean;
  /**
   * O tamanho, em bytes.
   */
  size: number;
  /**
   * O conteúdo é binário (tem byte nulo no começo).
   */
  binary: boolean;
  /**
   * Passou de [`MAX_FILE_SIZE`](crate::MAX_FILE_SIZE) e não foi lido.
   */
  too_large: boolean;
  /**
   * O texto, quando existe, não é binário e não é grande demais.
   */
  text: string | null;
}
/**
 * `git.commit`.
 */
export interface CommitResult {
  /**
   * O identificador completo.
   */
  oid: string;
  /**
   * O identificador curto.
   */
  short: string;
  /**
   * A primeira linha da mensagem.
   */
  summary: string;
  /**
   * O ramo; `None` com o HEAD fora de ramo.
   */
  branch: string | null;
}
/**
 * `git.discard`.
 */
export interface DiscardResult {
  /**
   * Os arquivos que voltaram ao último commit ou saíram, relativos ao topo.
   */
  discarded: string[];
  /**
   * As versões descartadas que foram para a lixeira do sistema, de onde
   * voltam: o caminho absoluto de cada uma, antes de ir.
   */
  trashed: string[];
}
/**
 * `git.init`.
 */
export interface InitResult {
  /**
   * A pasta do repositório (a do projeto).
   */
  top: string;
  /**
   * O ramo inicial.
   */
  branch: string;
  /**
   * As linhas acrescentadas ao `.gitignore`.
   */
  ignored: string[];
}
/**
 * `git.log`.
 */
export interface GitLog {
  /**
   * Os commits, do mais novo para o mais velho.
   */
  commits: CommitSummary[];
  /**
   * Há mais commits depois desta página.
   */
  more: boolean;
}
/**
 * Um commit do histórico.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "CommitSummary".
 */
export interface CommitSummary {
  /**
   * O identificador completo.
   */
  oid: string;
  /**
   * O identificador curto.
   */
  short: string;
  /**
   * Os pais (dois num merge, nenhum no primeiro commit).
   */
  parents: string[];
  /**
   * O autor.
   */
  author: string;
  /**
   * O e-mail do autor.
   */
  email: string;
  /**
   * A data do autor, em segundos desde 1970 (UTC).
   */
  time: number;
  /**
   * A primeira linha da mensagem.
   */
  summary: string;
  /**
   * O resto da mensagem.
   */
  body: string;
  /**
   * Os ramos e tags que apontam para ele (`origin/main`, `tag: v1.0`).
   */
  refs: string[];
  /**
   * Ainda não está em nenhum ramo remoto: só existe nesta máquina.
   */
  local: boolean;
}
/**
 * `git.commit_files`.
 */
export interface CommitFiles {
  /**
   * O commit.
   */
  oid: string;
  /**
   * O pai com que ele é comparado (o primeiro, num merge); `None` no
   * primeiro commit.
   */
  parent: string | null;
  /**
   * Os arquivos.
   */
  files: CommitFile[];
}
/**
 * Um arquivo mudado por um commit.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "CommitFile".
 */
export interface CommitFile {
  /**
   * O caminho, relativo ao topo, com `/`.
   */
  path: string;
  /**
   * O caminho anterior, num arquivo renomeado.
   */
  original: string | null;
  /**
   * A mudança.
   */
  change:
    | 'added'
    | 'untracked'
    | 'modified'
    | 'deleted'
    | 'renamed'
    | 'copied'
    | 'type_changed'
    | 'conflicted';
}
/**
 * `git.branches`.
 */
export interface Branches {
  /**
   * O atual; `None` com o HEAD fora de ramo.
   */
  current: string | null;
  /**
   * Os locais, do usado mais recentemente para o mais antigo.
   */
  local: LocalBranch[];
  /**
   * Os remotos sem ramo local com o mesmo nome.
   */
  remote: RemoteBranch[];
}
/**
 * Um ramo local.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LocalBranch".
 */
export interface LocalBranch {
  /**
   * O nome.
   */
  name: string;
  /**
   * O ramo remoto que ele acompanha.
   */
  upstream: string | null;
  /**
   * Commits que o remoto não tem.
   */
  ahead: number;
  /**
   * Commits do remoto que ele não tem.
   */
  behind: number;
  /**
   * O ramo remoto foi apagado.
   */
  gone: boolean;
  /**
   * A data do último commit, em segundos desde 1970.
   */
  time: number;
}
/**
 * Um ramo remoto.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "RemoteBranch".
 */
export interface RemoteBranch {
  /**
   * O nome completo (`origin/feature`).
   */
  name: string;
  /**
   * O remoto (`origin`).
   */
  remote: string;
  /**
   * O ramo no remoto (`feature`).
   */
  branch: string;
  /**
   * A data do último commit, em segundos desde 1970.
   */
  time: number;
}
/**
 * `git.fetch`, `git.pull`, `git.push`.
 */
export interface SyncResult {
  /**
   * O remoto.
   */
  remote: string;
  /**
   * O ramo.
   */
  branch: string | null;
  /**
   * O ramo remoto que ele acompanha, depois da operação.
   */
  upstream: string | null;
  /**
   * O ramo foi publicado agora (não existia no remoto).
   */
  published: boolean;
}
/**
 * A conta do GitHub conectada.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Account".
 */
export interface Account {
  /**
   * O nome de usuário (`octocat`).
   */
  login: string;
  /**
   * O identificador numérico.
   */
  id: number;
  /**
   * O nome, se a pessoa preencheu.
   */
  name: string | null;
  /**
   * A foto.
   */
  avatar_url: string;
  /**
   * A página do perfil.
   */
  html_url: string;
  /**
   * O e-mail para os commits que o GitHub liga à conta sem expor o
   * e-mail real (`123+octocat@users.noreply.github.com`).
   */
  commit_email: string;
  /**
   * O gerenciador de credenciais do git guardou o token: o login vale
   * depois de fechar a AURORA, e o `git push` do terminal também o usa.
   * Falso quando o git desta máquina não tem gerenciador de credenciais.
   */
  saved: boolean;
}
/**
 * `github.login_start`.
 */
export interface DeviceCode {
  /**
   * O código que identifica este login em [`login_poll`]. Não é para
   * mostrar.
   */
  device_code: string;
  /**
   * O código que a pessoa digita no GitHub (`WDJB-MJHT`).
   */
  user_code: string;
  /**
   * Onde ela digita (`https://github.com/login/device`).
   */
  verification_uri: string;
  /**
   * Em quantos segundos o código vence.
   */
  expires_in: number;
  /**
   * Quantos segundos esperar entre duas chamadas de [`login_poll`].
   */
  interval: number;
}
/**
 * A conta do GitHub conectada.
 */
export interface Account1 {
  /**
   * O nome de usuário (`octocat`).
   */
  login: string;
  /**
   * O identificador numérico.
   */
  id: number;
  /**
   * O nome, se a pessoa preencheu.
   */
  name: string | null;
  /**
   * A foto.
   */
  avatar_url: string;
  /**
   * A página do perfil.
   */
  html_url: string;
  /**
   * O e-mail para os commits que o GitHub liga à conta sem expor o
   * e-mail real (`123+octocat@users.noreply.github.com`).
   */
  commit_email: string;
  /**
   * O gerenciador de credenciais do git guardou o token: o login vale
   * depois de fechar a AURORA, e o `git push` do terminal também o usa.
   * Falso quando o git desta máquina não tem gerenciador de credenciais.
   */
  saved: boolean;
}
/**
 * O que `initialize` devolve.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ServerInfo".
 */
export interface ServerInfo1 {
  /**
   * A versão da AURORA.
   */
  aurora: string;
  /**
   * A versão do protocolo ([`PROTOCOL_VERSION`]).
   */
  protocol: number;
  /**
   * A pasta da instalação da AURORA, quando o servidor é o `aurora`
   * instalado.
   */
  prefix: string | null;
  /**
   * Cada recurso experimental e se está ligado.
   */
  experimental: {
    [k: string]: boolean | undefined;
  };
  /**
   * Os métodos que este servidor atende.
   */
  methods: string[];
}
/**
 * O projeto inteiro, como uma interface o mostra. Só lê o disco.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ProjectStatus".
 */
export interface ProjectStatus1 {
  /**
   * O nome do projeto (o do `.spf`).
   */
  name: string;
  /**
   * O `.spf`.
   */
  spf: string;
  /**
   * A pasta do projeto.
   */
  root: string;
  /**
   * Os módulos registrados (sintetizáveis), na ordem do `.spf`.
   */
  synthesizable: ProjectFile[];
  /**
   * Os testbenches registrados, na ordem do `.spf`.
   */
  testbenches: ProjectFile[];
  /**
   * O arquivo de topo, se escolhido.
   */
  top_level: string | null;
  /**
   * O módulo de topo, quando dá para saber: o `topLevelModule` do `.spf`,
   * o único módulo do arquivo de topo ou o que tem o nome dele.
   */
  top_module: string | null;
  /**
   * Por que o módulo de topo não dá para saber (arquivo com vários
   * módulos, nenhum com o nome dele).
   */
  top_module_error: ErrorInfo | null;
  /**
   * O testbench que a simulação do projeto roda.
   */
  selected_testbench: string | null;
  /**
   * O módulo desse testbench.
   */
  testbench_module: string | null;
  /**
   * A escolha de sinais da onda desse testbench (`wave/<testbench>.json`);
   * vazia, a onda grava todos os sinais.
   */
  wave_selection: string[];
  /**
   * Os `.v` e `.sv` da pasta que não estão registrados, e os `.py` com
   * `@cocotb.test`.
   */
  unregistered: string[];
  /**
   * Os processadores SAPHO, na ordem do `.spf`.
   */
  processors: ProcessorStatus[];
  /**
   * A onda da simulação do projeto, se já existe no disco.
   */
  waveform: string | null;
  /**
   * Os Verilog conhecidos que podem ser o topo: os registrados, o gerado
   * de cada processador e os de fora do `.spf`, menos os com nome de
   * testbench e os testbenches cocotb (a regra de
   * [`Project::set_top_level`]).
   */
  top_candidates: string[];
  /**
   * O que está estranho no `.spf` sem impedir de abrir
   * ([`Project::issues`]).
   */
  issues: ProjectIssue[];
}
/**
 * Os arquivos que o build de um processador gera, pelo papel. Só entram os
 * que existem.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "GeneratedFiles".
 */
export interface GeneratedFiles1 {
  /**
   * `<nome>.v`: o processador em Verilog, um módulo do design.
   */
  verilog: string | null;
  /**
   * `<nome>_tb.v`: o testbench que o `asmcomp` gera.
   */
  testbench: string | null;
  /**
   * `<nome>.asm`: o assembly que o compilador gera.
   */
  assembly: string | null;
  /**
   * `*.mif`: as memórias de dados e de instruções.
   */
  memories: string[];
  /**
   * As tabelas e os logs do YANC (`prog_log.txt`, `src_lines.txt`,
   * `trad_opcode.txt`, `app_log.txt`, `pc_<nome>_mem.txt`); as saídas da
   * simulação (`output_<n>.txt`, `toma_<n>.txt`) ficam de fora.
   */
  intermediates: string[];
}
/**
 * `project.remove`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "RemoveOutcome".
 */
export interface RemoveOutcome1 {
  /**
   * Tirados do projeto (continuam no disco).
   */
  removed: string[];
  /**
   * Pedidos, mas que não estavam no projeto.
   */
  not_registered: string[];
}
/**
 * `project.top`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "TopOutcome".
 */
export interface TopOutcome1 {
  /**
   * O arquivo de topo, se escolhido.
   */
  top_level: string | null;
  /**
   * O módulo de topo.
   */
  module: string | null;
  /**
   * Por que não dá para saber o módulo de um arquivo de topo escolhido.
   */
  module_error: ErrorInfo | null;
}
/**
 * O que [`Project::convert_layout`] fez.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LayoutConversion".
 */
export interface LayoutConversion1 {
  /**
   * Os processadores que tinham a estrutura antiga, na ordem do `.spf`.
   */
  processors: string[];
  /**
   * O que mudou de lugar.
   */
  moved: ConvertedPath[];
  /**
   * O que ficou onde estava, com o motivo.
   */
  skipped: SkippedPath[];
  /**
   * Os fontes que perderam a linha do nome (`#PRNAME`, `#pragma yanc
   * prname`).
   */
  name_lines: RemovedNameLine[];
  /**
   * Os processadores cujo nome não serve mais: na 7.1 ele vira o módulo
   * Verilog, e o YANC recusa um fonte com esse nome.
   */
  invalid_names: string[];
  /**
   * As subpastas antigas que ficaram com algo que não foi movido (o que o
   * sistema deixa sozinho numa pasta, como o `.DS_Store`, não conta).
   */
  remaining_dirs: string[];
}
/**
 * `project.check_name` e `processor.check_name`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "NameCheck".
 */
export interface NameCheck1 {
  /**
   * O nome serve.
   */
  ok: boolean;
  /**
   * Por que não serve (`invalid_name`).
   */
  error: ErrorInfo | null;
}
/**
 * Um processador do projeto, com todos os caminhos já absolutos.
 *
 * Os valores de simulação (`frequency_mhz`, `clocks`, `show_arrays`) vêm do
 * `.spf` (`clk`, `numClocks`, `showArrays`) ou dos padrões da AURORA
 * ([`DEFAULT_FREQUENCY_MHZ`], [`DEFAULT_CLOCKS`], `false`). Para mudá-los
 * só num build, use [`BuildOptions`](crate::BuildOptions).
 *
 * A linguagem é resolvida como na AURORA: `language` declarada no `.spf`;
 * senão a extensão de `sourceFile`/`cmmFile`; senão o arquivo que existir na
 * pasta do processador (`<nome>.cmm` antes de `<nome>.cpp`); senão C±.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Processor".
 */
export interface Processor1 {
  /**
   * O nome: a pasta, o nome do fonte, o módulo Verilog e os arquivos
   * gerados.
   */
  name: string;
  /**
   * A linguagem do fonte.
   */
  language: 'cmm' | 'cpp';
  /**
   * `<raiz>/<nome>`: o fonte e os arquivos que a pessoa escreve (os que
   * inicializam arrays, `input_<n>.txt`, `cade_<n>.txt`, os cabeçalhos
   * do C).
   */
  dir: string;
  /**
   * O programa-fonte, `<dir>/<nome>.cmm` ou `.cpp`.
   */
  source: string;
  /**
   * `<dir>/<nome>`, a pasta de saída do YANC (`-o`): o `.asm`, o Verilog,
   * as memórias, o testbench, as tabelas da onda e as saídas da
   * simulação, que roda nela.
   */
  output_dir: string;
  /**
   * `<raiz>/.aurora8/Temp/<nome>`: o que só a AURORA usa (o `.vvp`, o modelo do
   * Verilator, a cópia instrumentada do testbench, a trava de execução).
   */
  temp_dir: string;
  /**
   * Frequência de operação em MHz (`-f` do `asmcomp`).
   */
  frequency_mhz: number;
  /**
   * Clocks a simular (`-c` do `asmcomp`, vai para o testbench).
   */
  clocks: number;
  /**
   * Exporta cada elemento de array como um sinal da onda
   * (`--wave-arrays` do `cmmcomp`).
   */
  show_arrays: boolean;
}
/**
 * `processor.remove`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ProcessorRemoval".
 */
export interface ProcessorRemoval1 {
  removed: RemovedProcessor1;
  /**
   * A pasta foi para a lixeira do sistema.
   */
  trashed: boolean;
}
/**
 * `wave.layout`: o que um visualizador próprio precisa para mostrar a
 * onda com o layout da AURORA.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WaveLayoutInfo".
 */
export interface WaveLayoutInfo1 {
  /**
   * O layout dos processadores SAPHO, gerado da onda; `None` sem
   * processador.
   */
  layout: WaveLayout | null;
  /**
   * O layout salvo no projeto da onda (`wave/<testbench>.surf.ron`), onde
   * o Surfer salva (Ctrl+S): o do usuário, se ele salvou depois da última
   * geração, senão o gerado agora; `None` numa onda fora de projeto.
   */
  saved: SavedLayout | null;
  /**
   * Por que o layout não foi gerado (a onda abre sem ele).
   */
  layout_error: string | null;
  /**
   * Os comandos do Surfer de um exercício do `aurora learn` (o
   * `.aurora-learn/wave.sucl` ao lado da onda), quando a onda é de um.
   */
  exercise_layout: string | null;
}
/**
 * Um módulo e as portas dele, na ordem da declaração.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ModuleInterface".
 */
export interface ModuleInterface1 {
  /**
   * Nome do módulo.
   */
  name: string;
  /**
   * O arquivo que o declara.
   */
  file: string;
  /**
   * As portas.
   */
  ports: Port[];
}
/**
 * A placa virtual compilada e rodada: a compilação e, se ela deu certo,
 * como a execução terminou.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "BoardRun".
 */
export interface BoardRun1 {
  build: BoardBuild;
  /**
   * Como a execução terminou; `None` se a compilação falhou.
   */
  outcome: SessionOutcome | null;
}
/**
 * O resultado de [`build`]: a placa e o executável, ou os erros do
 * Verilator e do compilador C++.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "BoardBuild".
 */
export interface BoardBuild1 {
  /**
   * Como terminou. `failed` é Verilog (ou ligação) que não compila.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * O passo que falhou, quando for o caso (`verilate`).
   */
  failed_step: Step | null;
  /**
   * Os passos rodados, com o comando e a saída.
   */
  steps: StepReport[];
  /**
   * As mensagens do YANC, do Verilator e do compilador.
   */
  diagnostics: Diagnostic[];
  /**
   * Os processadores SAPHO compilados antes da placa, os desatualizados
   * ([`crate::Processor::is_out_of_date`]); vazio quando todos estavam em
   * dia.
   */
  processors: BuildResult[];
  /**
   * A placa virtual do projeto; `None` quando um processador não compilou,
   * antes de o topo poder ser lido.
   */
  layout: BoardLayout1 | null;
  /**
   * O executável, se a compilação deu certo.
   */
  executable: string | null;
  /**
   * Quanto levou, em milissegundos.
   */
  duration_ms: number;
}
/**
 * O resultado de um fluxo. Os campos de uma fase que não rodou ficam `None`
 * ou vazios.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FlowOutcome".
 */
export interface FlowOutcome1 {
  /**
   * O fluxo ([`FlowRequest::name`]).
   */
  flow: string;
  /**
   * O comando equivalente ([`FlowOptions::command`]).
   */
  command: string;
  /**
   * Como terminou: o pior entre as fases que rodaram. `Cancelled` se o
   * cancelamento foi pedido; `TimedOut` se a simulação estourou o prazo;
   * `Failed` se alguma fase falhou (inclusive um build de `Build`, que
   * compila todos); senão `Succeeded`.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * `status == Succeeded`.
   */
  succeeded: boolean;
  /**
   * Os builds feitos antes (ou o próprio build).
   */
  builds: BuildResult[];
  /**
   * O que esses builds geraram e entrou no `.spf` agora.
   */
  registered: AddedFile[];
  /**
   * O resultado da verificação.
   */
  check: CheckResult | null;
  /**
   * Com `lint`, o componente do Verilator que falta quando nenhum passo
   * dele rodou: para a interface sugerir a instalação.
   */
  lint_missing: string | null;
  /**
   * A hierarquia.
   */
  hierarchy: HierarchyResult | null;
  /**
   * Na hierarquia sem processador: os processadores com fonte e sem
   * build, que ficaram de fora.
   */
  not_built: string[];
  /**
   * O resultado da simulação.
   */
  simulation: SimulationResult | null;
  /**
   * Os valores de cada porta de saída do processador simulado (só com
   * processador e sucesso).
   */
  outputs: FlowPortValues[];
  /**
   * O resultado da síntese.
   */
  synthesis: SynthesisResult | null;
  /**
   * O resultado do esquemático.
   */
  schematic: SchematicResult | null;
  /**
   * Por que o esquemático não foi desenhado depois da síntese (o módulo
   * pedido não está no netlist, `module_not_found`); a síntese vale.
   */
  schematic_error: ErrorInfo | null;
  /**
   * O HTML do RTL viewer, gravado ao lado do netlist.
   */
  rtl_viewer: string | null;
  /**
   * Por que o RTL viewer não saiu (a síntese vale).
   */
  rtl_viewer_error: ErrorInfo | null;
  /**
   * A onda aberta no surfer-aurora.
   */
  wave: WaveOpened | null;
  /**
   * Por que a onda não abriu (a simulação vale).
   */
  wave_error: ErrorInfo | null;
  /**
   * O relatório gravado no histórico (`run-000042`).
   */
  report: string | null;
  /**
   * Por que o relatório não foi gravado (o fluxo não falha por isso).
   */
  report_error: string | null;
  /**
   * A compilação para a placa.
   */
  fpga: FpgaBuildResult | null;
  /**
   * A gravação na placa.
   */
  fpga_program: FpgaProgramResult | null;
  /**
   * A correção de um exercício (`learn`): o `Grade` do aurora-learn, em
   * JSON (o core não conhece o tipo).
   */
  learn: {
    [k: string]: unknown | undefined;
  };
}
/**
 * A árvore de sinais do testbench do projeto, para escolher o que a onda
 * grava ([`wave_signals`]).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WaveSignals".
 */
export interface WaveSignals1 {
  /**
   * O testbench elaborado (o da simulação do projeto).
   */
  testbench: string;
  /**
   * O módulo do testbench: a raiz da árvore e o nome dos arquivos em
   * `wave/`.
   */
  module: string;
  /**
   * A árvore, a partir do módulo do testbench. `None` quando a elaboração
   * falhou (os erros em `diagnostics`).
   */
  root: SignalScope | null;
  /**
   * A escolha gravada; vazia: a simulação grava todos os sinais.
   */
  selection: string[];
  /**
   * Onde a escolha fica (`wave/<módulo>.json`), exista ou não.
   */
  selection_file: string;
  /**
   * Itens da escolha que a árvore não tem (um sinal renomeado, um módulo
   * que saiu): a simulação os deixa de fora.
   */
  unknown: string[];
  /**
   * Como a elaboração terminou.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * A elaboração (`iverilog`).
   */
  steps: StepReport[];
  /**
   * Os erros e avisos da elaboração.
   */
  diagnostics: Diagnostic[];
  /**
   * Quanto levou, em milissegundos.
   */
  duration_ms: number;
}
/**
 * A escolha de sinais depois da mudança.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WaveChoiceOutcome".
 */
export interface WaveChoiceOutcome1 {
  /**
   * O módulo do testbench do projeto.
   */
  testbench: string;
  /**
   * O arquivo da escolha (`wave/<testbench>.json`); `None` quando ela
   * ficou vazia e o arquivo saiu: a onda grava todos os sinais.
   */
  file: string | null;
  /**
   * A escolha; vazia, todos os sinais.
   */
  selection: string[];
}
/**
 * `history.show`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ReportView".
 */
export interface ReportView1 {
  /**
   * O identificador (`run-000042`).
   */
  id: string;
  /**
   * O `report.txt`.
   */
  path: string;
  record: RunRecord;
  /**
   * O `report.txt`, sem mudança.
   */
  text: string;
}
/**
 * Um relatório guardado: o que a comparação e a lista usam. O texto fica ao
 * lado, em `report.txt` ([`report_text`]).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "RunRecord".
 */
export interface RunRecord1 {
  /**
   * O formato ([`RECORD_SCHEMA`]).
   */
  schema: number;
  /**
   * O identificador (`run-000042`).
   */
  id: string;
  metadata: RunMetadata;
  /**
   * As estatísticas do Yosys, quando houve síntese e o Yosys as gravou.
   */
  synthesis: SynthesisStatistics | null;
  /**
   * Os tempos da simulação, quando houve simulação.
   */
  simulation: SimulationTimings | null;
}
/**
 * O contexto de uma operação guardada.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "RunMetadata".
 */
export interface RunMetadata3 {
  /**
   * Quando terminou, em UTC (`2026-10-03T21:04:05Z`).
   */
  timestamp: string;
  /**
   * O comando.
   */
  command: string;
  /**
   * Como uma operação terminou. Em JSON, em `snake_case` (`"succeeded"`).
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * Quanto levou, do começo ao fim, em milissegundos.
   */
  duration_ms: number;
  /**
   * A versão que gravou. `lace_version` nos relatórios gravados pelo
   * Lace, que a `.lace` levada para a `.aurora8` traz.
   */
  aurora_version: string;
  /**
   * O que identifica o projeto: um hash da raiz dele. Relatórios de
   * projetos diferentes não se comparam.
   */
  project: string;
  /**
   * O nome do projeto.
   */
  project_name: string;
  /**
   * O bundle de ferramentas (`2026.09.29`) e a plataforma.
   */
  bundle: string;
  /**
   * `linux-x64`, `darwin-arm64`, `windows-x64`.
   */
  platform: string;
  /**
   * O módulo de topo: o da síntese, ou o testbench simulado, ou o topo do
   * projeto.
   */
  top: string | null;
  /**
   * O contexto da síntese, quando houve.
   */
  synthesis: SynthesisContext | null;
  /**
   * O contexto da simulação, quando houve.
   */
  simulation: SimulationContext | null;
  environment: Environment;
}
/**
 * A máquina onde a operação rodou. O que o sistema não informa sem rodar
 * outro programa fica `null`: a AURORA só lê arquivos do sistema (no Linux,
 * `/proc` e `/etc/os-release`) e o `uname`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Environment".
 */
export interface Environment1 {
  /**
   * O nome da máquina.
   */
  hostname: string | null;
  /**
   * `linux`, `macos`, `windows`.
   */
  os: string;
  /**
   * O nome da distribuição (Linux).
   */
  os_name: string | null;
  /**
   * A versão do kernel (Linux e macOS).
   */
  kernel: string | null;
  /**
   * `x86_64`, `aarch64`.
   */
  arch: string;
  /**
   * O modelo do processador (Linux).
   */
  cpu_model: string | null;
  /**
   * Processadores lógicos disponíveis.
   */
  cpus: number | null;
  /**
   * A memória total, em bytes (Linux).
   */
  memory_bytes: number | null;
  /**
   * Hash do sistema, da arquitetura, do processador, do número de
   * processadores e do nome da máquina: muda quando a máquina muda.
   */
  fingerprint: string;
}
/**
 * Dois relatórios comparados: o atual contra a referência.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "RunComparison".
 */
export interface RunComparison1 {
  /**
   * O relatório atual.
   */
  current_id: string;
  current: RunMetadata1;
  /**
   * A referência.
   */
  baseline_id: string;
  baseline: RunMetadata2;
  /**
   * A síntese comparada; `null` se os dois não têm estatísticas do mesmo
   * topo.
   */
  synthesis: SynthesisComparison | null;
  /**
   * A simulação comparada; `null` se os dois não têm tempos do mesmo
   * simulador e testbench.
   */
  simulation: TimingComparison | null;
  /**
   * O que difere no contexto e pode explicar uma mudança (fontes,
   * versões, máquina), em inglês, sem ponto final.
   */
  warnings: string[];
}
/**
 * Um número comparado.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "MetricComparison".
 */
export interface MetricComparison6 {
  /**
   * Na referência.
   */
  baseline: number | null;
  /**
   * No atual.
   */
  current: number | null;
  /**
   * O que mudou.
   */
  change: 'unchanged' | 'increased' | 'decreased' | 'added' | 'removed' | 'not_comparable';
  /**
   * Atual menos referência.
   */
  delta: number | null;
  /**
   * A variação em porcentagem da referência. `null` quando não se
   * compara ou quando a referência é zero e o atual não (não há
   * porcentagem de zero; o texto mostra `new`).
   */
  percent: number | null;
}
/**
 * `history.clean`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "CleanOutcome".
 */
export interface CleanOutcome1 {
  /**
   * Os apagados, do mais antigo para o mais novo.
   */
  removed: string[];
  /**
   * Quantos ficaram.
   */
  kept: number;
}
/**
 * O que uma tela de ferramentas mostra: o `aurora tools` em dados. Nunca
 * falha: um bundle que não abriu vira `found: false` com o erro
 * ([`describe_error`]).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ToolchainInfo".
 */
export interface ToolchainInfo1 {
  /**
   * O bundle abriu.
   */
  found: boolean;
  /**
   * Por que não abriu.
   */
  error: ErrorInfo | null;
  /**
   * Como foi achado.
   */
  origin: Origin | null;
  /**
   * A pasta do bundle.
   */
  root: string | null;
  /**
   * A versão do bundle.
   */
  bundle: string | null;
  /**
   * A plataforma do bundle (`linux-x64`, `darwin-arm64`, `windows-x64`).
   */
  platform: string | null;
  /**
   * Os componentes instalados.
   */
  components: BundleComponent[];
  /**
   * Os componentes que a AURORA conhece e que não estão instalados.
   */
  not_installed: string[];
  /**
   * Cada ferramenta que a AURORA roda.
   */
  tools: ToolStatus[];
  /**
   * O compilador do Verilator: do sistema no Linux e no macOS, do bundle
   * no Windows (`bundled`).
   */
  system_compiler: SystemCompiler | null;
  /**
   * O compilador declarado não serve.
   */
  compiler_error: ErrorInfo | null;
  /**
   * O Quartus Prime do sistema, se encontrado.
   */
  quartus: Quartus1 | null;
  /**
   * A pasta declarada do Quartus não serve.
   */
  quartus_error: ErrorInfo | null;
  /**
   * A CLI `aurora` da instalação.
   */
  aurora_cli: string | null;
  /**
   * O cliente web do Surfer (`surfer-aurora/web`), para a onda numa aba;
   * `None` num bundle que não o traz.
   */
  surfer_web: string | null;
}
/**
 * `experimental.list` e `experimental.set`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ExperimentalInfo".
 */
export interface ExperimentalInfo1 {
  /**
   * O `experimental.json`; `None` sem pasta de configuração.
   */
  path: string | null;
  /**
   * Cada recurso e se está ligado.
   */
  components: FeatureState[];
}
/**
 * O `fpga.json` conferido: a placa, as ligações bit a bit e o topo da
 * placa gerado.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Prepared".
 */
export interface Prepared1 {
  /**
   * O `fpga.json`.
   */
  config: string;
  resolved: Resolved1;
  /**
   * O Verilog do topo da placa ([`wrapper::generate`]).
   */
  board_top: string;
  /**
   * Cada ligação como texto ([`wrapper::connections`]): cada entrada do
   * topo e cada sinal de saída da placa usado, com o que o dirige.
   */
  connections: Connection[];
}
/**
 * O arquivo de gravação da placa do `fpga.json`: se existe e, se existe,
 * por que não descreve o projeto de agora ([`bitstream_state`]).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "BitstreamState".
 */
export interface BitstreamState1 {
  /**
   * A placa (o `id`).
   */
  board: string;
  /**
   * O `.sof`, se a placa já foi compilada.
   */
  bitstream: string | null;
  /**
   * Quando o `.sof` foi gravado, em milissegundos desde 1970.
   */
  built_at_ms: number | null;
  /**
   * Por que o `.sof` não serve para gravar, uma razão por item: vazio
   * com `bitstream`, ele descreve o projeto de agora.
   */
  reasons: string[];
}
/**
 * O relatório do `aurora update` (`docs/schema/update.json`).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "UpdateReport".
 */
export interface UpdateReport1 {
  aurora: AuroraVersions;
  bundle: BundleVersions;
  /**
   * Cada componente instalado, na ordem do manifesto do bundle. Vazio sem
   * bundle.
   */
  components: ComponentVersions[];
  /**
   * A pasta da instalação; `null` se este `aurora` não foi instalado pelo
   * instalador (um build em `target/`, por exemplo).
   */
  prefix: string | null;
  /**
   * O que o comando fez.
   */
  action: 'checked' | 'up_to_date' | 'updated' | 'wizard_opened';
  /**
   * Como atualizou; `null` quando não atualizou.
   */
  method: UpdateMethod | null;
  /**
   * O que a atualização por componentes trocou, ou vai trocar com
   * `--check --from`; `null` quando ela não rodou.
   */
  changes: UpdateChanges | null;
}
/**
 * As versões da AURORA em `aurora update`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "AuroraVersions".
 */
export interface AuroraVersions1 {
  /**
   * A versão deste `aurora`.
   */
  installed: string;
  /**
   * A versão da última release no GitHub.
   */
  latest: string;
  /**
   * `latest` é mais nova que `installed`.
   */
  newer: boolean;
}
/**
 * As versões do bundle em `aurora update`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "BundleVersions".
 */
export interface BundleVersions1 {
  /**
   * O bundle instalado; `null` sem bundle.
   */
  installed: string | null;
  /**
   * O bundle da última release; `null` se o `bundle/versions.json` dela
   * não pôde ser lido.
   */
  latest: string | null;
  /**
   * `latest` é mais novo que `installed`.
   */
  newer: boolean;
}
/**
 * `aurora install` (`docs/schema/install.json`).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "InstallReport".
 */
export interface InstallReport1 {
  /**
   * A pasta da instalação.
   */
  prefix: string;
  /**
   * Os aplicativos do bundle instalados agora.
   */
  components: string[];
  /**
   * Os que entraram nesta chamada (com os que eles exigem).
   */
  added: string[];
}
/**
 * O resultado de uma rodada de correção (e do `learn.check` do protocolo).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LearnCheckOutcome".
 */
export interface LearnCheckOutcome1 {
  /**
   * A correção de cada exercício pedido, na ordem; termina no primeiro
   * cancelado.
   */
  results: Grade[];
  /**
   * Quantos exercícios da trilha estão resolvidos depois da rodada.
   */
  solved: number;
  /**
   * Quantos exercícios a trilha tem.
   */
  total: number;
}
/**
 * De onde vêm as trilhas e quais há (o resultado do `learn.tracks` do
 * protocolo).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LearnTracks".
 */
export interface LearnTracks1 {
  /**
   * A pasta das trilhas.
   */
  dir: string;
  /**
   * As trilhas dela.
   */
  tracks: string[];
}
/**
 * A pasta de exercícios como uma interface a mostra: a trilha, os
 * capítulos com os enunciados e o que o aluno já fez
 * ([`Workspace::snapshot`]). É o resultado do `learn.status` do protocolo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WorkspaceSnapshot".
 */
export interface WorkspaceSnapshot1 {
  /**
   * A pasta de exercícios.
   */
  root: string;
  /**
   * A trilha (`verilog`).
   */
  track: string;
  /**
   * O título da trilha.
   */
  title: string;
  /**
   * As boas-vindas, em markdown.
   */
  welcome: string;
  /**
   * A mensagem do fim, em markdown.
   */
  farewell: string | null;
  /**
   * Os capítulos, na ordem.
   */
  chapters: ChapterSnapshot[];
  /**
   * O exercício atual.
   */
  current: string;
  /**
   * Quantos resolvidos.
   */
  solved: number;
  /**
   * Quantos exercícios.
   */
  total: number;
}
/**
 * `git.status`: o git da máquina e o repositório do projeto.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "GitStatus".
 */
export interface GitStatus1 {
  /**
   * A versão do git (`2.54.0.windows.1`).
   */
  version: string;
  /**
   * O repositório que contém o projeto; `None` fora de repositório.
   */
  repository: RepositoryStatus | null;
}
/**
 * O ramo atual e a relação com o remoto.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "BranchState".
 */
export interface BranchState1 {
  /**
   * O nome; `None` com o HEAD fora de ramo.
   */
  name: string | null;
  /**
   * O commit atual; `None` num repositório ainda sem commit.
   */
  commit: string | null;
  /**
   * O ramo remoto que ele acompanha (`origin/main`); `None` se ainda não
   * foi publicado.
   */
  upstream: string | null;
  /**
   * Commits locais que o remoto não tem.
   */
  ahead: number;
  /**
   * Commits do remoto que o local não tem (pelo último fetch).
   */
  behind: number;
  /**
   * O remoto para onde o ramo vai ao publicar: o configurado no ramo, o
   * `origin` ou o único que houver.
   */
  remote: string | null;
}
/**
 * `git.read_file`: o conteúdo de um arquivo numa versão.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FileVersion".
 */
export interface FileVersion1 {
  /**
   * O arquivo existe nessa versão.
   */
  exists: boolean;
  /**
   * O tamanho, em bytes.
   */
  size: number;
  /**
   * O conteúdo é binário (tem byte nulo no começo).
   */
  binary: boolean;
  /**
   * Passou de [`MAX_FILE_SIZE`](crate::MAX_FILE_SIZE) e não foi lido.
   */
  too_large: boolean;
  /**
   * O texto, quando existe, não é binário e não é grande demais.
   */
  text: string | null;
}
/**
 * `git.commit`: o commit criado.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "CommitResult".
 */
export interface CommitResult1 {
  /**
   * O identificador completo.
   */
  oid: string;
  /**
   * O identificador curto.
   */
  short: string;
  /**
   * A primeira linha da mensagem.
   */
  summary: string;
  /**
   * O ramo; `None` com o HEAD fora de ramo.
   */
  branch: string | null;
}
/**
 * `git.discard`: o que foi descartado.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "DiscardResult".
 */
export interface DiscardResult1 {
  /**
   * Os arquivos que voltaram ao último commit ou saíram, relativos ao topo.
   */
  discarded: string[];
  /**
   * As versões descartadas que foram para a lixeira do sistema, de onde
   * voltam: o caminho absoluto de cada uma, antes de ir.
   */
  trashed: string[];
}
/**
 * `git.init`: o repositório criado.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "InitResult".
 */
export interface InitResult1 {
  /**
   * A pasta do repositório (a do projeto).
   */
  top: string;
  /**
   * O ramo inicial.
   */
  branch: string;
  /**
   * As linhas acrescentadas ao `.gitignore`.
   */
  ignored: string[];
}
/**
 * `git.log`: uma página do histórico do ramo atual.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "GitLog".
 */
export interface GitLog1 {
  /**
   * Os commits, do mais novo para o mais velho.
   */
  commits: CommitSummary[];
  /**
   * Há mais commits depois desta página.
   */
  more: boolean;
}
/**
 * `git.commit_files`: os arquivos que um commit mudou.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "CommitFiles".
 */
export interface CommitFiles1 {
  /**
   * O commit.
   */
  oid: string;
  /**
   * O pai com que ele é comparado (o primeiro, num merge); `None` no
   * primeiro commit.
   */
  parent: string | null;
  /**
   * Os arquivos.
   */
  files: CommitFile[];
}
/**
 * `git.branches`: os ramos.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Branches".
 */
export interface Branches1 {
  /**
   * O atual; `None` com o HEAD fora de ramo.
   */
  current: string | null;
  /**
   * Os locais, do usado mais recentemente para o mais antigo.
   */
  local: LocalBranch[];
  /**
   * Os remotos sem ramo local com o mesmo nome.
   */
  remote: RemoteBranch[];
}
/**
 * `git.fetch`, `git.pull` e `git.push`: o que foi feito com o remoto.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SyncResult".
 */
export interface SyncResult1 {
  /**
   * O remoto.
   */
  remote: string;
  /**
   * O ramo.
   */
  branch: string | null;
  /**
   * O ramo remoto que ele acompanha, depois da operação.
   */
  upstream: string | null;
  /**
   * O ramo foi publicado agora (não existia no remoto).
   */
  published: boolean;
}
/**
 * O código do fluxo de dispositivo, de [`login_start`].
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "DeviceCode".
 */
export interface DeviceCode1 {
  /**
   * O código que identifica este login em [`login_poll`]. Não é para
   * mostrar.
   */
  device_code: string;
  /**
   * O código que a pessoa digita no GitHub (`WDJB-MJHT`).
   */
  user_code: string;
  /**
   * Onde ela digita (`https://github.com/login/device`).
   */
  verification_uri: string;
  /**
   * Em quantos segundos o código vence.
   */
  expires_in: number;
  /**
   * Quantos segundos esperar entre duas chamadas de [`login_poll`].
   */
  interval: number;
}
/**
 * O resultado de [`build`]. Ver o módulo `pipeline` para os campos comuns a
 * toda operação.
 *
 * Em JSON (resumido):
 *
 * ```json
 * { "processor": "soma", "language": "cmm", "status": "succeeded",
 *   "failed_step": null, "frequency_mhz": 100, "clocks": 2000,
 *   "steps": [ { "step": "compile", "tool": "cmmcomp", "...": "..." } ],
 *   "diagnostics": [],
 *   "artifacts": [ { "kind": "verilog", "path": "/p/soma/soma/soma.v",
 *                    "required": true, "fresh": true } ] }
 * ```
 */
export interface BuildResult1 {
  /**
   * O processador compilado.
   */
  processor: string;
  /**
   * A linguagem do fonte, que decidiu o pipeline.
   */
  language: 'cmm' | 'cpp';
  /**
   * Como o build terminou.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * O passo que falhou ou quebrou, quando for o caso.
   */
  failed_step: Step | null;
  /**
   * A frequência usada (do `.spf` ou de [`BuildOptions`]).
   */
  frequency_mhz: number;
  /**
   * Os clocks usados (do `.spf` ou de [`BuildOptions`]).
   */
  clocks: number;
  /**
   * Um relatório por compilador executado: `compile` e `assemble`
   * ([`build_steps`]).
   */
  steps: StepReport[];
  /**
   * Todas as mensagens de todos os passos, na ordem em que saíram. Hoje
   * os compiladores param no primeiro erro, mas a API já é uma lista.
   */
  diagnostics: Diagnostic[];
  /**
   * Obrigatórios, na pasta de saída: `<nome>.asm`, `<nome>.v`,
   * `<nome>_data.mif`, `<nome>_inst.mif` e o testbench `<nome>_tb.v`.
   * Intermediários (`pc_<nome>_mem.txt`, `src_lines.txt`,
   * `trad_opcode.txt`, `prog_log.txt`, `app_log.txt`), quando existem.
   */
  artifacts: Artifact[];
  /**
   * Quanto o build levou, do começo ao fim, em milissegundos.
   */
  duration_ms: number;
}
/**
 * A placa virtual de um projeto: a placa, o topo e cada periférico, ligado
 * ao projeto ou não, com as larguras. É o que uma interface desenha.
 */
export interface BoardLayout2 {
  /**
   * A placa (`de2-115`).
   */
  board: string;
  /**
   * O nome da placa (`Terasic DE2-115`).
   */
  board_name: string;
  /**
   * O módulo do projeto que vai na placa.
   */
  top: string;
  /**
   * Os periféricos, na ordem do JSON da placa.
   */
  peripherals: LayoutPeripheral[];
  /**
   * Os relógios da placa que chegam ao projeto.
   */
  clocks: LayoutClock[];
  /**
   * Os ajustes das ligações (larguras completadas ou cortadas, entradas
   * soltas), como no fluxo da placa.
   */
  notes: string[];
}
/**
 * O estado.
 */
export interface BoardState {
  /**
   * O tempo simulado, em picossegundos.
   */
  time_ps: number;
  /**
   * Segundos simulados por segundo de relógio desde o estado anterior: 1
   * é o tempo real.
   */
  speed: number;
  /**
   * A simulação está parada ([`BoardInput::Pause`]).
   */
  paused: boolean;
  /**
   * Quanto cada LED e cada segmento ficou aceso desde o estado anterior,
   * de 0 a 255, por sinal, do bit 0 em diante: o PWM aparece como brilho,
   * e o display multiplexado como persistência.
   */
  lights: {
    [k: string]: number[] | undefined;
  };
  /**
   * Os bytes que o projeto transmitiu em cada UART desde o estado
   * anterior.
   */
  uart: {
    [k: string]: number[] | undefined;
  };
  /**
   * A velocidade de cada UART que não está na da placa: o projeto
   * transmitia em outra, e a placa passou a ler na padrão mais próxima do
   * bit medido (os primeiros bytes se perdem). Vazio na velocidade da
   * placa.
   */
  uart_baud?: {
    [k: string]: number | undefined;
  };
}
/**
 * O quadro, em PNG.
 */
export interface Frame {
  /**
   * O periférico de vídeo.
   */
  device: string;
  /**
   * A largura visível, em pixels.
   */
  width: number;
  /**
   * A altura visível, em linhas.
   */
  height: number;
  /**
   * A imagem em PNG, em base64.
   */
  png: string;
}
/**
 * O estado da placa virtual, cerca de 30 vezes por segundo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "BoardState".
 */
export interface BoardState1 {
  /**
   * O tempo simulado, em picossegundos.
   */
  time_ps: number;
  /**
   * Segundos simulados por segundo de relógio desde o estado anterior: 1
   * é o tempo real.
   */
  speed: number;
  /**
   * A simulação está parada ([`BoardInput::Pause`]).
   */
  paused: boolean;
  /**
   * Quanto cada LED e cada segmento ficou aceso desde o estado anterior,
   * de 0 a 255, por sinal, do bit 0 em diante: o PWM aparece como brilho,
   * e o display multiplexado como persistência.
   */
  lights: {
    [k: string]: number[] | undefined;
  };
  /**
   * Os bytes que o projeto transmitiu em cada UART desde o estado
   * anterior.
   */
  uart: {
    [k: string]: number[] | undefined;
  };
  /**
   * A velocidade de cada UART que não está na da placa: o projeto
   * transmitia em outra, e a placa passou a ler na padrão mais próxima do
   * bit medido (os primeiros bytes se perdem). Vazio na velocidade da
   * placa.
   */
  uart_baud?: {
    [k: string]: number | undefined;
  };
}
/**
 * Um quadro de vídeo remontado pela placa virtual. Só sai quando a imagem
 * muda.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Frame".
 */
export interface Frame1 {
  /**
   * O periférico de vídeo.
   */
  device: string;
  /**
   * A largura visível, em pixels.
   */
  width: number;
  /**
   * A altura visível, em linhas.
   */
  height: number;
  /**
   * A imagem em PNG, em base64.
   */
  png: string;
}
/**
 * O que a simulação de um processador escreveu numa porta de saída.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "PortValues".
 */
export interface PortValues {
  /**
   * O número da porta.
   */
  port: number;
  /**
   * `<proc>/<proc>/output_<porta>.txt`, na pasta de saída.
   */
  path: string;
  /**
   * Os valores, na ordem em que saíram. Vazio quando a porta não pôde ser
   * lida.
   */
  values: number[];
  /**
   * Por que a porta não pôde ser lida: uma linha que não é inteiro, como o
   * `x` que o simulador escreve numa divisão por zero. `null` quando leu.
   */
  error: string | null;
}
/**
 * Um [`AuroraError`] pronto para atravessar uma fronteira: o JSON da CLI, a
 * mensagem do Studio para a interface, a resposta do `aurora serve`. É o que
 * o código, o texto e os campos da variante viram quando o enum não vai.
 *
 * ```json
 * { "code": "processor_not_found",
 *   "message": "No processor 'x' in the project (available: soma, filtro)",
 *   "data": { "name": "x", "available": ["soma", "filtro"] } }
 * ```
 *
 * A dica de cada interface (o comando `aurora` que resolve, o botão do
 * Studio) não mora aqui: a mensagem do Core é neutra, e quem mostra decide
 * pelo `code`. Um erro que não é do Core (o `busy` do Studio, o `cli` da
 * CLI) usa o mesmo tipo com [`ErrorInfo::new`] e um código próprio.
 *
 * ```
 * use aurora_core::{ErrorInfo, AuroraError};
 *
 * let info = ErrorInfo::from(AuroraError::ProcessorExists("soma".into()));
 * assert_eq!(info.code, "processor_exists");
 * assert_eq!(info.data.unwrap()["name"], "soma");
 * ```
 */
export interface ErrorInfo1 {
  /**
   * O identificador estável: [`AuroraError::code`], ou um código de quem
   * chamou (`busy`, `cli`), sempre em `snake_case`.
   */
  code: string;
  /**
   * O texto do erro, em inglês, pronto para mostrar.
   */
  message: string;
  /**
   * O que fazer, quando quem mostra sabe: o comando `aurora` que resolve,
   * na CLI. O core não o preenche; a mensagem dele é neutra. Omitido no
   * JSON quando não há.
   */
  hint?: string | null;
  /**
   * Os campos estruturados da variante, quando ela os tem
   * ([`AuroraError::data`]): `available`, `files`, `name`, `value`,
   * `reason`, `path`, `component`. Omitido no JSON quando não há.
   */
  data?: {
    [k: string]: unknown | undefined;
  };
}
/**
 * O layout salvo no projeto para a onda aberta.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SavedLayoutReport".
 */
export interface SavedLayoutReport {
  /**
   * O arquivo.
   */
  path: string;
  /**
   * O usuário salvou depois que a AURORA o gerou: a AURORA não o refaz
   * (`aurora wave --reset-layout` volta ao gerado).
   */
  customized: boolean;
}
/**
 * `aurora add`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "AddReport".
 */
export interface AddReport {
  /**
   * O `.spf`.
   */
  project: string;
  /**
   * Um por arquivo, na ordem da linha de comando.
   */
  files: AddedFile[];
}
/**
 * `aurora build`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "BuildReport".
 */
export interface BuildReport {
  /**
   * O `.spf`.
   */
  project: string;
  /**
   * Um por processador compilado. Vazio num projeto sem processadores.
   */
  results: BuildResult[];
  /**
   * Os arquivos que o build gerou e que entraram no `.spf` agora
   * (`Project::register_generated`): o `<nome>/<nome>/<nome>.v` entre os
   * sintetizáveis e o `<nome>/<nome>/<nome>_tb.v` entre os testbenches.
   * Vazio se já estavam no projeto.
   */
  registered: AddedFile[];
  /**
   * O relatório gravado no histórico do projeto (`run-000042`), para
   * `aurora report show`; `null` se nada rodou ou se não pôde ser gravado.
   */
  report: string | null;
}
/**
 * `aurora check`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "CheckReport".
 */
export interface CheckReport {
  check: CheckResult1;
  /**
   * O relatório gravado no histórico do projeto (`run-000042`), para
   * `aurora report show`; `null` se nada rodou ou se não pôde ser gravado.
   */
  report: string | null;
}
/**
 * O resultado de [`check`]. Não há artefatos: a verificação só diz se o
 * Verilog elabora, e o que os compiladores acharam.
 */
export interface CheckResult1 {
  /**
   * Os módulos elaborados como raiz: os do projeto (ou os do arquivo
   * pedido) e cada testbench.
   */
  targets: string[];
  /**
   * `Succeeded` se tudo elabora sem erro.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * O primeiro passo que falhou.
   */
  failed_step: Step | null;
  /**
   * Um `iverilog -t null` para o projeto, um por testbench e, com
   * `lint`, o `verilator --lint-only`.
   */
  steps: StepReport[];
  /**
   * Os erros e avisos, com arquivo e linha.
   */
  diagnostics: Diagnostic[];
  /**
   * Quanto a verificação levou, do começo ao fim, em milissegundos.
   */
  duration_ms: number;
}
/**
 * `aurora convert`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ConvertReport".
 */
export interface ConvertReport {
  /**
   * O `.spf`.
   */
  project: string;
  /**
   * Os processadores que tinham a estrutura antiga, na ordem do `.spf`.
   */
  processors: string[];
  /**
   * O que mudou de lugar.
   */
  moved: ConvertedPath[];
  /**
   * O que ficou onde estava, com o motivo.
   */
  skipped: SkippedPath[];
  /**
   * Os fontes que perderam a linha do nome (`#PRNAME`, `#pragma yanc
   * prname`).
   */
  name_lines: RemovedNameLine[];
  /**
   * Os processadores cujo nome não serve mais: na 7.1 ele vira o módulo
   * Verilog, e o YANC recusa um fonte com esse nome.
   */
  invalid_names: string[];
  /**
   * As subpastas antigas que ficaram com algo que não foi movido (o que o
   * sistema deixa sozinho numa pasta, como o `.DS_Store`, não conta).
   */
  remaining_dirs: string[];
}
/**
 * Qualquer comando que não conseguiu rodar (código de saída 2).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ErrorReport".
 */
export interface ErrorReport {
  error: ErrorInfo2;
}
/**
 * Um [`AuroraError`] pronto para atravessar uma fronteira: o JSON da CLI, a
 * mensagem do Studio para a interface, a resposta do `aurora serve`. É o que
 * o código, o texto e os campos da variante viram quando o enum não vai.
 *
 * ```json
 * { "code": "processor_not_found",
 *   "message": "No processor 'x' in the project (available: soma, filtro)",
 *   "data": { "name": "x", "available": ["soma", "filtro"] } }
 * ```
 *
 * A dica de cada interface (o comando `aurora` que resolve, o botão do
 * Studio) não mora aqui: a mensagem do Core é neutra, e quem mostra decide
 * pelo `code`. Um erro que não é do Core (o `busy` do Studio, o `cli` da
 * CLI) usa o mesmo tipo com [`ErrorInfo::new`] e um código próprio.
 *
 * ```
 * use aurora_core::{ErrorInfo, AuroraError};
 *
 * let info = ErrorInfo::from(AuroraError::ProcessorExists("soma".into()));
 * assert_eq!(info.code, "processor_exists");
 * assert_eq!(info.data.unwrap()["name"], "soma");
 * ```
 */
export interface ErrorInfo2 {
  /**
   * O identificador estável: [`AuroraError::code`], ou um código de quem
   * chamou (`busy`, `cli`), sempre em `snake_case`.
   */
  code: string;
  /**
   * O texto do erro, em inglês, pronto para mostrar.
   */
  message: string;
  /**
   * O que fazer, quando quem mostra sabe: o comando `aurora` que resolve,
   * na CLI. O core não o preenche; a mensagem dele é neutra. Omitido no
   * JSON quando não há.
   */
  hint?: string | null;
  /**
   * Os campos estruturados da variante, quando ela os tem
   * ([`AuroraError::data`]): `available`, `files`, `name`, `value`,
   * `reason`, `path`, `component`. Omitido no JSON quando não há.
   */
  data?: {
    [k: string]: unknown | undefined;
  };
}
/**
 * `aurora experimental`: os recursos experimentais e se cada um está ligado,
 * já com a mudança de `enable` ou `disable`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ExperimentalReport".
 */
export interface ExperimentalReport {
  /**
   * O arquivo do que está ligado (`experimental.json`, na pasta de
   * configuração da AURORA); `null` se o sistema não diz onde fica essa
   * pasta.
   */
  path: string | null;
  /**
   * Os recursos, na ordem da lista.
   */
  components: FeatureState[];
}
/**
 * `aurora fpga boards`: as placas conhecidas, ou a pedida.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FpgaBoardsReport".
 */
export interface FpgaBoardsReport {
  /**
   * As placas, com o FPGA, o cabo e os sinais com os pinos.
   */
  boards: Board[];
}
/**
 * `aurora fpga build`: os processadores, compilados antes, e a compilação
 * para a placa pelo Quartus.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FpgaBuildReport".
 */
export interface FpgaBuildReport {
  /**
   * Os processadores. Se um falha, `fpga` é `null`.
   */
  builds: BuildResult[];
  /**
   * Os arquivos que o build gerou e que entraram no `.spf` agora
   * (`Project::register_generated`): o `<nome>/<nome>/<nome>.v` entre os
   * sintetizáveis e o `<nome>/<nome>/<nome>_tb.v` entre os testbenches.
   * Vazio se já estavam no projeto.
   */
  registered: AddedFile[];
  /**
   * A compilação para a placa.
   */
  fpga: FpgaBuildResult | null;
}
/**
 * `aurora fpga program --list`: os cabos de gravação ligados.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FpgaCablesReport".
 */
export interface FpgaCablesReport {
  /**
   * Como o `quartus_pgm -l` os escreve (`USB-Blaster [USB-0]`); vazio,
   * nenhum.
   */
  cables: string[];
}
/**
 * `aurora fpga check`: o `fpga.json` conferido.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "FpgaCheckReport".
 */
export interface FpgaCheckReport {
  /**
   * O `fpga.json`.
   */
  config: string;
  resolved: Resolved2;
  /**
   * O Verilog do topo da placa que a AURORA gera.
   */
  board_top: string;
  /**
   * Cada ligação como texto: o que vai em cada entrada do topo e em cada
   * sinal de saída da placa.
   */
  connections: Connection[];
}
/**
 * As ligações, bit a bit.
 */
export interface Resolved2 {
  board: Board1;
  /**
   * O módulo do topo.
   */
  top: string;
  /**
   * As portas do topo, na ordem da declaração.
   */
  ports: TopPort[];
  /**
   * As entradas do topo, com a origem de cada bit, do bit 0 em diante.
   */
  inputs: PortDrive[];
  /**
   * Os sinais de saída da placa usados, com a porta que dirige cada bit.
   */
  board_outputs: SignalDrive[];
  /**
   * Os sinais de entrada da placa usados, na ordem em que aparecem.
   */
  board_inputs: string[];
  /**
   * As portas ligadas a um clock da placa.
   */
  clocks: Clock[];
  /**
   * Os ajustes feitos: larguras completadas ou cortadas, entradas soltas.
   */
  notes: string[];
}
/**
 * `aurora learn check`: a correção de cada exercício pedido, que também
 * grava se ele está resolvido.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LearnCheckReport".
 */
export interface LearnCheckReport {
  /**
   * Uma por exercício, na ordem da trilha.
   */
  results: Grade[];
  /**
   * Quantos exercícios da trilha estão resolvidos agora.
   */
  solved: number;
  /**
   * Quantos a trilha tem.
   */
  total: number;
}
/**
 * O resultado da conferência.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "DevReport".
 */
export interface DevReport {
  /**
   * A trilha.
   */
  track: string;
  /**
   * Cada exercício, na ordem.
   */
  exercises: DevExercise[];
}
/**
 * `aurora learn hint`: as dicas de um exercício.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LearnHintReport".
 */
export interface LearnHintReport {
  /**
   * O exercício.
   */
  exercise: string;
  /**
   * O título.
   */
  title: string;
  /**
   * As dicas, em markdown, na ordem.
   */
  hints: string[];
}
/**
 * `aurora learn init`: a pasta de exercícios criada.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LearnInitReport".
 */
export interface LearnInitReport {
  /**
   * A pasta de exercícios.
   */
  root: string;
  /**
   * A trilha (`verilog`).
   */
  track: string;
  /**
   * Quantos exercícios a trilha tem.
   */
  exercises: number;
  /**
   * O exercício atual (o primeiro).
   */
  current: string;
}
/**
 * `aurora learn list`: os exercícios e o que já foi resolvido.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LearnListReport".
 */
export interface LearnListReport {
  /**
   * A pasta de exercícios.
   */
  root: string;
  /**
   * A trilha.
   */
  track: string;
  /**
   * O título dela.
   */
  title: string;
  /**
   * Quantos resolvidos.
   */
  solved: number;
  /**
   * Quantos exercícios.
   */
  total: number;
  /**
   * Cada exercício, na ordem.
   */
  exercises: ExerciseStatus[];
}
/**
 * `aurora learn reset`: o arquivo que voltou ao começo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LearnResetReport".
 */
export interface LearnResetReport {
  /**
   * O exercício.
   */
  exercise: string;
  /**
   * O arquivo do aluno.
   */
  file: string;
}
/**
 * `aurora learn wave`: a onda aberta no Surfer.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "LearnWaveReport".
 */
export interface LearnWaveReport {
  /**
   * O exercício.
   */
  exercise: string;
  /**
   * A onda da última correção.
   */
  waveform: string;
  /**
   * O arquivo de comandos com que ela abriu.
   */
  layout: string | null;
  /**
   * O processo do Surfer.
   */
  pid: number;
}
/**
 * `aurora move`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "MoveReport".
 */
export interface MoveReport {
  /**
   * O `.spf`.
   */
  project: string;
  /**
   * Um por origem, na ordem da linha de comando.
   */
  moved: MovedPath[];
}
/**
 * `aurora new`: o projeto criado.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "NewReport".
 */
export interface NewReport {
  /**
   * Sempre `"Project created"`.
   */
  message: string;
  /**
   * O `.spf` criado.
   */
  path: string;
  /**
   * A pasta do projeto.
   */
  root: string;
}
/**
 * `aurora order`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "OrderReport".
 */
export interface OrderReport {
  /**
   * O `.spf`.
   */
  project: string;
  /**
   * A lista em que o arquivo está, na ordem nova.
   */
  files: ProjectFile[];
}
/**
 * `aurora proc remove`: o processador que saiu do projeto e se a pasta dele
 * foi para a lixeira (`--delete`).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ProcRemoveReport".
 */
export interface ProcRemoveReport {
  removed: RemovedProcessor2;
  /**
   * A pasta foi para a lixeira do sistema; `false`, ela continua no disco
   * (ou não existia).
   */
  trashed: boolean;
}
/**
 * O que saiu: o nome, a pasta e os arquivos que saíram das listas.
 */
export interface RemovedProcessor2 {
  /**
   * O nome do processador.
   */
  name: string;
  /**
   * A pasta dele (`<raiz>/<nome>`), que continua no disco.
   */
  dir: string;
  /**
   * Os arquivos de dentro da pasta que estavam registrados e saíram das
   * listas: o papel de cada um e se era o topo ou o testbench escolhido.
   */
  unregistered: ProjectFile[];
}
/**
 * O processador; só `name` e `language` são obrigatórios.
 */
export interface NewProcessor2 {
  /**
   * Nome do processador: letras, dígitos e `_`, começando por letra ou `_`.
   */
  name: string;
  /**
   * Linguagem do fonte-modelo.
   */
  language: 'cmm' | 'cpp';
  /**
   * `#NUBITS`: largura da palavra de dados, em bits. Só C±.
   */
  nubits?: number;
  /**
   * `#NBMANT`: bits de mantissa do ponto flutuante do SAPHO (não é IEEE
   * 754). Só C±.
   */
  nbmant?: number;
  /**
   * `#NBEXPO`: bits de expoente do ponto flutuante. Só C±.
   */
  nbexpo?: number;
  /**
   * `#NUGAIN`: divisor de `norm(x)`. Precisa ser potência de dois, senão o
   * `cmmcomp` recusa (um divisor genérico viraria o caminho crítico da
   * ULA). Só C±; o fonte só traz a diretiva quando difere de 128, o
   * padrão do YANC.
   */
  nugain?: number;
  /**
   * `#NDSTAC`: profundidade da pilha de dados. Só C±. `None`, o padrão:
   * sem a diretiva, e o `asmcomp` calcula a profundidade pelo programa.
   */
  ndstac?: number | null;
  /**
   * `#SDEPTH`: profundidade da pilha de instruções (chamadas). Só C±.
   * `None`, o padrão: sem a diretiva, calculada pelo programa.
   */
  sdepth?: number | null;
  /**
   * `#NUIOIN` / `#pragma yanc nuioin`: número de portas de entrada.
   */
  input_ports?: number;
  /**
   * `#NUIOOU` / `#pragma yanc nuioou`: número de portas de saída.
   */
  output_ports?: number;
}
/**
 * O que muda (`None` mantém).
 */
export interface ProcessorConfig1 {
  /**
   * Frequência de operação em MHz (`clk` no `.spf`).
   */
  frequency_mhz?: number | null;
  /**
   * Clocks que o testbench gerado simula (`numClocks` no `.spf`).
   */
  clocks?: number | null;
  /**
   * Exportar arrays para a simulação (`showArrays` no `.spf`).
   */
  show_arrays?: boolean | null;
}
/**
 * Qual onda e com que layout.
 */
export interface WaveRequest1 {
  /**
   * Qual onda.
   */
  source:
    | {
        /**
         * O arquivo.
         */
        path: string;
        kind: 'file';
      }
    | {
        /**
         * O processador; `None`, o projeto.
         */
        processor?: string | null;
        kind: 'project';
      };
  /**
   * O layout.
   */
  layout?:
    | {
        kind: 'generated';
      }
    | {
        kind: 'none';
      }
    | {
        kind: 'reset';
      }
    | {
        /**
         * O arquivo (`.surf.ron` ou `.sucl`).
         */
        path: string;
        kind: 'file';
      };
}
/**
 * O que o `fpga.json` guarda.
 */
export interface FpgaConfig2 {
  /**
   * A placa ([`super::board()`]).
   */
  board: string;
  /**
   * O módulo que vai para a placa. Sem ele, o topo do projeto.
   */
  top?: string | null;
  /**
   * As ligações, na ordem do arquivo. Uma porta com vários sinais (uma
   * saída que vai a mais de um lugar da placa) tem uma [`Link`] para cada.
   */
  connect?: {
    [k: string]: LinkTargets | undefined;
  };
}
/**
 * A mudança.
 */
export interface Reconnect1 {
  /**
   * O sinal da placa (`SW`, `HEX0`).
   */
  signal: string;
  /**
   * O bit do sinal; `None`, o sinal inteiro, do bit 0 em diante.
   */
  bit?: number | null;
  /**
   * A porta que vai nele; `None` deixa o bit (ou o sinal) solto.
   */
  port?: PortChoice | null;
}
/**
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "Responses".
 */
export interface Responses {
  response: Response;
  notification: Notification;
  error: ErrorInfo;
  shutdown: ShutdownReason;
}
/**
 * `aurora remove`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "RemoveReport".
 */
export interface RemoveReport {
  /**
   * O `.spf`.
   */
  project: string;
  /**
   * Tirados do projeto (continuam no disco).
   */
  removed: string[];
  /**
   * Pedidos, mas que não estavam no projeto.
   */
  not_registered: string[];
}
/**
 * `aurora report clean`: os relatórios apagados.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ReportCleanReport".
 */
export interface ReportCleanReport {
  /**
   * Os apagados, do mais antigo para o mais novo. Vazio se não havia o
   * que apagar.
   */
  removed: string[];
  /**
   * Quantos ficaram no histórico.
   */
  kept: number;
}
/**
 * `aurora report list`: os relatórios, do mais novo para o mais antigo.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ReportListReport".
 */
export interface ReportListReport {
  /**
   * Os relatórios, até o `--limit`.
   */
  reports: RunSummary[];
}
/**
 * `aurora report` e `aurora report show`: um relatório guardado.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ReportShowReport".
 */
export interface ReportShowReport {
  /**
   * O identificador (`run-000042`).
   */
  id: string;
  /**
   * O `report.txt`.
   */
  path: string;
  record: RunRecord2;
  /**
   * O `report.txt`, sem mudança.
   */
  text: string;
}
/**
 * O que a comparação usa: contexto, estatísticas e tempos.
 */
export interface RunRecord2 {
  /**
   * O formato ([`RECORD_SCHEMA`]).
   */
  schema: number;
  /**
   * O identificador (`run-000042`).
   */
  id: string;
  metadata: RunMetadata;
  /**
   * As estatísticas do Yosys, quando houve síntese e o Yosys as gravou.
   */
  synthesis: SynthesisStatistics | null;
  /**
   * Os tempos da simulação, quando houve simulação.
   */
  simulation: SimulationTimings | null;
}
/**
 * `aurora sim`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SimReport".
 */
export interface SimReport {
  /**
   * Os processadores, compilados antes. Se um falha, `simulation` é
   * `null`.
   */
  builds: BuildResult[];
  /**
   * Os arquivos que o build gerou e que entraram no `.spf` agora
   * (`Project::register_generated`): o `<nome>/<nome>/<nome>.v` entre os
   * sintetizáveis e o `<nome>/<nome>/<nome>_tb.v` entre os testbenches.
   * Vazio se já estavam no projeto.
   */
  registered: AddedFile[];
  /**
   * A simulação.
   */
  simulation: SimulationResult | null;
  /**
   * Com `-p`, o que a simulação escreveu em cada porta de saída.
   */
  outputs: PortValues[];
  /**
   * Com `--open`, o processo do surfer-aurora.
   */
  surfer_pid: number | null;
  /**
   * O relatório gravado no histórico do projeto (`run-000042`), para
   * `aurora report show`; `null` se nada rodou ou se não pôde ser gravado.
   */
  report: string | null;
}
/**
 * `aurora status`: o projeto inteiro.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "StatusReport".
 */
export interface StatusReport {
  /**
   * Nome do projeto (o do `.spf`).
   */
  name: string;
  /**
   * O `.spf`.
   */
  spf: string;
  /**
   * A pasta do projeto.
   */
  root: string;
  /**
   * Os módulos registrados.
   */
  synthesizable: ProjectFile[];
  /**
   * Os testbenches registrados.
   */
  testbench: ProjectFile[];
  /**
   * O arquivo de topo, se escolhido.
   */
  top_level: string | null;
  /**
   * O módulo de topo: o `topLevelModule` do `.spf`, o único módulo do
   * arquivo de topo ou o que tem o nome dele. `null` se não dá para saber.
   */
  top_module: string | null;
  /**
   * O testbench que `aurora sim` simula.
   */
  selected_testbench: string | null;
  /**
   * O módulo desse testbench.
   */
  testbench_module: string | null;
  /**
   * Arquivos `.v` e `.sv` da pasta que não estão no projeto.
   */
  unregistered: string[];
  /**
   * O processador em cuja pasta o comando rodou: o que `build`, `check`,
   * `sim`, `wave`, `synth` e `proc set` usam quando não recebem nome.
   * `null` fora da pasta de um processador.
   */
  here: string | null;
  /**
   * Os processadores SAPHO.
   */
  processors: ProcessorStatus[];
  /**
   * O que está estranho no `.spf` sem impedir de abrir
   * (`Project::issues`): caminho de outra máquina achado pela cauda,
   * topo fora da lista, processador com nome que não compila.
   */
  issues: ProjectIssue[];
}
/**
 * `aurora synth`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "SynthReport".
 */
export interface SynthReport {
  /**
   * Os processadores, compilados antes. Se um falha, o resto é `null`.
   */
  builds: BuildResult[];
  /**
   * Os arquivos que o build gerou e que entraram no `.spf` agora
   * (`Project::register_generated`): o `<nome>/<nome>/<nome>.v` entre os
   * sintetizáveis e o `<nome>/<nome>/<nome>_tb.v` entre os testbenches.
   * Vazio se já estavam no projeto.
   */
  registered: AddedFile[];
  /**
   * A síntese.
   */
  synthesis: SynthesisResult | null;
  /**
   * Com `--svg`, o esquemático.
   */
  schematic: SchematicResult | null;
  /**
   * Com `--open`, o HTML do RTL viewer, gravado ao lado do netlist; `null`
   * sem `--open` ou numa síntese que não terminou.
   */
  rtl_viewer: string | null;
  /**
   * O relatório gravado no histórico do projeto (`run-000042`), para
   * `aurora report show`; `null` se nada rodou ou se não pôde ser gravado.
   */
  report: string | null;
}
/**
 * `aurora tools`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "ToolsReport".
 */
export interface ToolsReport {
  /**
   * A pasta do bundle.
   */
  root: string;
  /**
   * A versão do bundle.
   */
  bundle: string;
  /**
   * A plataforma do bundle (`linux-x64`, `darwin-arm64`, `windows-x64`).
   */
  platform: string;
  /**
   * Os componentes instalados.
   */
  components: BundleComponent[];
  /**
   * Os componentes que esta AURORA conhece e que não estão instalados.
   */
  not_installed: string[];
  /**
   * Cada ferramenta, pelo nome do executável.
   */
  tools: {
    [k: string]: ToolEntry | undefined;
  };
  /**
   * O compilador do Verilator, se encontrado: do sistema no Linux e no
   * macOS, do bundle no Windows (`bundled`).
   */
  system_compiler: SystemCompiler | null;
  /**
   * O Quartus Prime do sistema, que compila para as placas Intel, se
   * encontrado.
   */
  quartus: Quartus1 | null;
  /**
   * Com `--verify`, os executáveis cujo SHA-256 não confere (vazio: todos
   * conferem).
   */
  verify: FileMismatch[] | null;
}
/**
 * `aurora top`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "TopReport".
 */
export interface TopReport {
  /**
   * O arquivo de topo, se escolhido.
   */
  top_level: string | null;
  /**
   * O módulo de topo.
   */
  module: string | null;
  /**
   * Por que não dá para saber o módulo de um arquivo de topo escolhido
   * (vários módulos, nenhum com o nome do arquivo).
   */
  module_error: ErrorInfo | null;
}
/**
 * `aurora uninstall` (`docs/schema/uninstall.json`).
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "UninstallReport".
 */
export interface UninstallReport {
  /**
   * A pasta da instalação.
   */
  prefix: string;
  /**
   * `true`: a pasta e o atalho já saíram (Linux, macOS). `false`: o
   * desinstalador do Windows foi aberto e termina depois que quem chamou
   * sai.
   */
  removed: boolean;
}
/**
 * `aurora vboard run`: a placa virtual compilada e rodada sem janela.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "VboardRunReport".
 */
export interface VboardRunReport {
  build: BoardBuild2;
  /**
   * Como a execução terminou.
   */
  outcome: SessionOutcome | null;
  /**
   * O último estado: LEDs e segmentos acesos (0 a 255) e os bytes da UART
   * do último intervalo.
   */
  state: BoardState1 | null;
  /**
   * O PNG do último quadro de vídeo, se pedido e se houve vídeo.
   */
  png: string | null;
  /**
   * O que o projeto escreveu (`$display`), até 100 linhas por estado.
   */
  output: string[];
  /**
   * As linhas que passaram desse limite e ficaram de fora de `output`.
   */
  output_omitted: number;
}
/**
 * A compilação da placa virtual. Se falha, os outros campos são
 * `null`.
 */
export interface BoardBuild2 {
  /**
   * Como terminou. `failed` é Verilog (ou ligação) que não compila.
   */
  status: 'succeeded' | 'failed' | 'crashed' | 'incomplete' | 'cancelled' | 'timed_out';
  /**
   * O passo que falhou, quando for o caso (`verilate`).
   */
  failed_step: Step | null;
  /**
   * Os passos rodados, com o comando e a saída.
   */
  steps: StepReport[];
  /**
   * As mensagens do YANC, do Verilator e do compilador.
   */
  diagnostics: Diagnostic[];
  /**
   * Os processadores SAPHO compilados antes da placa, os desatualizados
   * ([`crate::Processor::is_out_of_date`]); vazio quando todos estavam em
   * dia.
   */
  processors: BuildResult[];
  /**
   * A placa virtual do projeto; `None` quando um processador não compilou,
   * antes de o topo poder ser lido.
   */
  layout: BoardLayout1 | null;
  /**
   * O executável, se a compilação deu certo.
   */
  executable: string | null;
  /**
   * Quanto levou, em milissegundos.
   */
  duration_ms: number;
}
/**
 * `aurora wave select` e `aurora wave unselect`: a escolha de sinais depois da
 * mudança.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WaveChoiceReport".
 */
export interface WaveChoiceReport {
  /**
   * O módulo do testbench do projeto.
   */
  testbench: string;
  /**
   * O arquivo da escolha (`wave/<testbench>.json`); `null` quando ela
   * ficou vazia e o arquivo saiu: a onda grava todos os sinais.
   */
  file: string | null;
  /**
   * A escolha; vazia: todos os sinais.
   */
  selection: string[];
}
/**
 * `aurora wave`.
 *
 * This interface was referenced by `AuroraSchemas`'s JSON-Schema
 * via the `definition` "WaveReport".
 */
export interface WaveReport {
  /**
   * A onda aberta.
   */
  waveform: string;
  /**
   * O processo do surfer-aurora, que continua aberto.
   */
  pid: number;
  /**
   * Onde vai o stdout e o stderr do surfer-aurora.
   */
  log: string;
  /**
   * O estado do Surfer com que a onda abriu (`.surf.ron`): o layout salvo
   * no projeto ou o gerado; `null` com `--no-layout`, numa onda sem sinal
   * para mostrar ou que não é VCD nem FST.
   */
  layout: string | null;
  /**
   * O layout salvo no projeto (`wave/<testbench>.surf.ron`), onde o
   * Surfer salva (Ctrl+S); `null` numa onda fora de projeto.
   */
  saved_layout: SavedLayoutReport | null;
  /**
   * Os processadores do layout.
   */
  processors: WaveProcessor[];
}
