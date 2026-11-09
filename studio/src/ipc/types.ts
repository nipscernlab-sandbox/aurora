// Os tipos do próprio Studio, espelhando os structs Rust de src-tauri/src.
// Os tipos do Core da AURORA (BuildResult, SimulationResult, Event...) vêm
// gerados em ./aurora-types.ts. A referência de cada comando está em
// docs/IPC.md.
//
// Ao mudar um struct em Rust, mude aqui também: não há geração automática
// destes (ver docs/ARCHITECTURE.md, "Contrato entre backend e interface").

import type {
  AddedFile,
  BoardLayout,
  BoardRun,
  BoardState,
  BuildResult,
  BundleComponent,
  ChapterSnapshot,
  CheckResult,
  Connection,
  Elaboration,
  Event,
  ExerciseSnapshot,
  FileRole,
  FpgaBuildResult,
  FpgaProgramResult,
  Grade,
  HierarchyResult,
  Language,
  LearnTracks,
  ModuleInstance,
  MovedPath,
  Processor,
  ProjectIssue,
  Quartus,
  Resolved,
  SchematicResult,
  SimulationResult,
  Simulator,
  Status,
  Step,
  SynthesisResult,
  SystemCompiler,
  WaveProcessor,
  WorkspaceSnapshot,
} from './aurora-types';

export type {
  AddedFile,
  Elaboration,
  HierarchyResult,
  Language,
  LearnTracks,
  ModuleInstance,
  MovedPath,
  Processor,
  ProjectIssue,
  Simulator,
  WaveProcessor,
};

/** `error.rs`: o erro de todo comando. */
export interface IpcError {
  code: string;
  message: string;
  /** Os campos do erro que o servidor da AURORA mandou, quando há: os
   * arquivos de `git_switch_blocked`, o stderr de `git_failed`. */
  data?: Record<string, unknown>;
}

/** `commands/app.rs`: AppInfo. */
export interface AppInfo {
  name: string;
  version: string;
  os: string;
  aurora_platform: string | null;
  config_dir: string | null;
  tauri_version: string;
  webview_version: string | null;
  /** Build de desenvolvimento. */
  debug: boolean;
}

/** `settings.rs`: Settings. */
export interface Settings {
  language: 'system' | 'pt' | 'en';
  /** A paleta: o id do tema escuro dela em themes/catalog.ts. */
  theme: string;
  /** O modo: um esquema fixo, ou o do sistema. */
  theme_mode: 'dark' | 'light' | 'system';
  /** A aparência: a clássica, com as regiões separadas por linhas, ou as
   * ilhas (app.css, `data-look`). */
  look: 'classic' | 'islands';
  toolchain_dir: string | null;
  compiler_dir: string | null;
  /** O Quartus Prime, como o `--quartus` da CLI; null: os locais padrão. */
  quartus_dir: string | null;
  /** As trilhas do `aurora learn` de outra pasta, como o AURORA_LEARN_DIR. */
  learn_dir: string | null;
  /** O programa `aurora` que o Studio abre como servidor; null: o da instalação. */
  aurora_path: string | null;
  simulator: Simulator;
  open_wave_after_sim: boolean;
  /** Onde a onda abre: numa aba (cliente web do Surfer) ou em janela. */
  wave_viewer: 'tab' | 'window';
  sim_timeout_s: number | null;
  verbose: boolean;
  restore_last_project: boolean;
  /** Procurar AURORA mais nova ao abrir e a cada 12 horas (state/updates.ts). */
  check_updates: boolean;
  /** Windows: `powershell` ou `cmd`. */
  terminal_shell: 'powershell' | 'cmd';
  editor: EditorSettings;
  zen: ZenSettings;
  layouts: LayoutSettings;
  /** O tour pela interface já apareceu (state/tour.ts). Sem `settings.json`,
   * começa `false`, e o tour abre sozinho. */
  tour_done: boolean;
  /** O executável do Claude Code para o assistente (ou a pasta dele); null
   * procura no PATH e nas pastas dos instaladores. */
  assistant_claude: string | null;
  /** O `aurora` que serve as ferramentas ao assistente; null, o da instalação. */
  assistant_aurora: string | null;
  /** O modelo do assistente (`--model` do Claude Code); null, o padrão. */
  assistant_model: string | null;
  /** O assistente pode editar e criar arquivos na pasta do projeto. */
  assistant_allow_edits: boolean;
  recent_projects: RecentProject[];
}

/** `settings.rs`: LayoutSettings. Cada layout gravado vai como a interface o
 * escreveu (state/layoutModel.ts); o backend não o lê. */
export interface LayoutSettings {
  /** O id do layout em uso: um dos gravados ou um pronto (`default`). */
  active: string;
  saved: unknown[];
}

/** `settings.rs`: ZenSettings. */
export interface ZenSettings {
  fullscreen: boolean;
  center_layout: boolean;
  show_tabs: boolean;
  hide_line_numbers: boolean;
}

export interface EditorSettings {
  font_size: number;
  tab_size: number;
  word_wrap: boolean;
  minimap: boolean;
  auto_save: boolean;
  vim_mode: boolean;
}

export interface RecentProject {
  spf: string;
  name: string;
  opened_at_ms: number;
}

/** `commands/app.rs`: RecentStatus. */
export interface RecentStatus extends RecentProject {
  exists: boolean;
}

/** `commands/project.rs`: ProjectFile do Core. */
export interface ProjectFile {
  role: FileRole;
  path: string;
  top_level: boolean;
}

/** `commands/project.rs`: ProjectSnapshot. */
export interface ProjectSnapshot {
  name: string;
  spf: string;
  root: string;
  synthesizable: ProjectFile[];
  testbenches: ProjectFile[];
  top_level: string | null;
  top_module: string | null;
  top_module_error: IpcError | null;
  selected_testbench: string | null;
  testbench_module: string | null;
  /** A escolha de sinais da onda do testbench (`wave/<testbench>.json`):
   * vazia, a onda grava todos os sinais. */
  wave_selection: string[];
  unregistered: string[];
  processors: ProcessorStatus[];
  waveform: string | null;
  /** Os Verilog que podem ser o topo (a regra do Core: qualquer um, menos
   * nome de testbench), registrados, gerados e de fora do `.spf`. */
  top_candidates: string[];
  /** O que está estranho no `.spf` sem impedir de abrir (`Project::issues`). */
  issues: ProjectIssue[];
}

export interface PortFile {
  port: number;
  path: string;
}

/** `commands/project.rs`: ProcessorStatus (o Processor do Core achatado). */
export interface ProcessorStatus extends Processor {
  built: boolean;
  inputs: PortFile[];
  outputs: PortFile[];
  missing_inputs: string[];
  waveform: string | null;
  generated: GeneratedFiles;
}

/** `commands/project.rs`: GeneratedFiles. Só os que existem. */
export interface GeneratedFiles {
  verilog: string | null;
  testbench: string | null;
  assembly: string | null;
  memories: string[];
  intermediates: string[];
}

/** `commands/history.rs`: CleanupRequest. */
export type CleanupRequest = { kind: 'all' } | { kind: 'keep_latest'; keep: number } | { kind: 'reports'; ids: string[] };

/** `commands/history.rs`: CleanReport. */
export interface CleanReport {
  removed: string[];
  kept: number;
}

/** `commands/project.rs`: NewProcessorRequest. */
export interface NewProcessorRequest {
  name: string;
  language: Language;
  input_ports?: number;
  output_ports?: number;
  nubits?: number;
  nbmant?: number;
  nbexpo?: number;
  nugain?: number;
  ndstac?: number;
  sdepth?: number;
}

/** O NewProcessor do Core, com os padrões da AURORA. */
export interface NewProcessorDefaults {
  name: string;
  language: Language;
  nubits: number;
  nbmant: number;
  nbexpo: number;
  nugain: number;
  /** `null`: sem a diretiva, e o YANC calcula a pilha pelo programa. */
  ndstac: number | null;
  sdepth: number | null;
  input_ports: number;
  output_ports: number;
}

/** `Project::convert_layout` do Core (`aurora_core::LayoutConversion`). */
export interface LayoutConversion {
  processors: string[];
  moved: { processor: string; from: string; to: string; generated: boolean }[];
  skipped: { processor: string; from: string; to: string; reason: string }[];
  name_lines: { processor: string; file: string; old_name: string }[];
  invalid_names: string[];
  remaining_dirs: string[];
}

/** `commands/files.rs`. */
export interface DirEntry {
  name: string;
  path: string;
  is_dir: boolean;
  hidden: boolean;
  /** Nome de testbench (`tb_<nome>.v`, `<nome>_tb.v`): não pode ser o topo. */
  testbench_name: boolean;
}

export interface TextFile {
  path: string;
  content: string;
  modified_ms: number;
  binary: boolean;
  too_large: boolean;
}

export interface FileStat {
  is_dir: boolean;
  modified_ms: number;
  size: number;
}

export interface SearchMatch {
  path: string;
  line: number;
  column: number;
  length: number;
  text: string;
}

/** `flows.rs`: FlowRequest. */
export type FlowRequest =
  | { flow: 'build'; processors?: string[] }
  | { flow: 'check'; file?: string | null; processor?: string | null; lint?: boolean }
  | {
      flow: 'simulate';
      processor?: string | null;
      testbench?: string | null;
      simulator: Simulator;
      /** A simulação rápida: sem onda, com o testbench Verilog no
       * Verilator; `simulator` só vale para um testbench cocotb. */
      fast?: boolean;
      timeout_s?: number | null;
      open_wave?: boolean;
    }
  | { flow: 'synthesize'; processor?: string | null; schematic?: boolean; module?: string | null }
  | { flow: 'schematic'; netlist: string; module: string; bus_widths?: boolean }
  /** `aurora learn check`: corrige um exercício e grava se está resolvido. */
  | { flow: 'learn'; root: string; exercise: string }
  /** `aurora fpga build`: os processadores e depois o projeto para a placa, pelo Quartus. */
  | { flow: 'fpga_build' }
  /** `aurora fpga program`: grava o `.sof` da última compilação para a placa. */
  | { flow: 'fpga_program'; cable?: string | null };

export type FlowName = FlowRequest['flow'];

/** `flows.rs`: PortValues. */
export interface PortValues {
  port: number;
  path: string;
  values: number[];
  error: IpcError | null;
}

/** `aurora_core::flows::WaveOpened`. */
export interface WaveOpened {
  waveform: string;
  pid: number;
  log: string;
  /** O estado do Surfer com que a onda abriu (`.surf.ron` ou `.sucl`). */
  layout?: string | null;
  saved_layout?: { path: string; customized: boolean } | null;
  processors?: WaveProcessor[];
  selection?: string[];
  /** Processadores compilados de novo depois da simulação: PC e linha em
   * números até simular de novo. */
  outdated: string[];
  /** O layout gerado falhou e a onda abriu sem ele: o motivo. */
  layout_error?: string | null;
  reset_deleted?: string | null;
}

/** `aurora_core::flows::FlowOutcome`, com `wave_tab` e `learn` do Studio
 * (`flows.rs`). */
export interface FlowOutcome {
  flow: FlowName;
  command: string;
  /** Como terminou: o pior entre as fases que rodaram. */
  status?: Status;
  succeeded: boolean;
  builds: BuildResult[];
  /** O `.v` e o testbench que os builds geraram e entraram no `.spf` agora. */
  registered: AddedFile[];
  check: CheckResult | null;
  /** Com `lint`, o componente do Verilator que falta. */
  lint_missing?: string | null;
  hierarchy?: HierarchyResult | null;
  not_built?: string[];
  simulation: SimulationResult | null;
  outputs: PortValues[];
  synthesis: SynthesisResult | null;
  schematic: SchematicResult | null;
  rtl_viewer?: string | null;
  rtl_viewer_error?: IpcError | null;
  wave: WaveOpened | null;
  /** A onda a abrir numa aba, no lugar da janela. */
  wave_tab: string | null;
  wave_error: IpcError | null;
  /** Por que o esquemático não saiu depois da síntese (grande demais). */
  schematic_error: IpcError | null;
  report: string | null;
  report_error: string | null;
  /** A correção de um exercício (fluxo `learn`). */
  learn: Grade | null;
  /** A compilação para a placa (fluxo `fpga_build`). */
  fpga: FpgaBuildResult | null;
  /** A gravação na placa (fluxo `fpga_program`). */
  fpga_program: FpgaProgramResult | null;
}

/** As operações que rodam a CLI (`aurora install`, `aurora update`). */
export type CliFlow = 'install' | 'update';

/** O resultado de uma operação da CLI, embrulhado pelo Studio. `result` é o
 * JSON que ela escreveu: o relatório do comando, `{error}` quando falhou, ou
 * `null` quando não escreveu nada. */
export interface CliOutcome {
  flow: CliFlow;
  succeeded: boolean;
  cancelled: boolean;
  exit_code: number | null;
  result: unknown;
}

/** `aurora_core::flows::Phase`. */
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

/** O `fpga.json` (`aurora_core::fpga::FpgaConfig`): a placa, o topo e as
 * ligações, porta (com faixa) para sinal da placa (`[!]SINAL[faixa]`, `0`,
 * `1`), na ordem do arquivo; uma saída que vai a vários sinais tem a lista
 * deles. */
export interface FpgaConfig {
  board: string;
  top?: string | null;
  connect: Record<string, string | string[]>;
}

/** `fpga::Prepared`: o `fpga.json` conferido (`aurora fpga check`). */
export interface FpgaPrepared {
  config: string;
  resolved: Resolved;
  board_top: string;
  /** Cada ligação como texto: o que vai em cada entrada do topo e em cada
   * sinal de saída da placa. */
  connections: Connection[];
}

/** `fpga::BitstreamState`: o arquivo de gravação da placa e por que ele não
 * serve para gravar (vazio com `bitstream`: pronto). */
export interface BitstreamState {
  board: string;
  bitstream: string | null;
  built_at_ms: number | null;
  reasons: string[];
}

/** `verilog::ModuleInterface`: o topo que vai para a placa e as portas dele. */
export interface ModuleInterface {
  name: string;
  file: string;
  ports: { name: string; direction: 'input' | 'output' | 'inout'; width: number; signed: boolean }[];
}

/** `jobs.rs`: JobMessage. */
export type JobMessage =
  | { type: 'started'; job: number; command: string }
  | { type: 'phase'; phase: Phase }
  | { type: 'events'; events: Event[] }
  | { type: 'output_omitted'; step: Step; tool: string; lines: number }
  | { type: 'build'; result: BuildResult }
  | { type: 'install'; event: unknown }
  | { type: 'cli_output'; stream: 'stdout' | 'stderr'; line: string }
  | { type: 'finished'; outcome: FlowOutcome | CliOutcome }
  | { type: 'failed'; error: IpcError };

/** `vboard.rs`: o que a placa virtual manda pelo canal do `vboard_start`,
 * com o `type` igual ao método da notificação do `vboard.run`. */
export type VboardMessage =
  | { type: 'started'; run: number }
  | { type: 'phase'; phase: Phase }
  | { type: 'events'; events: Event[] }
  | { type: 'output_omitted'; step: Step; tool: string; lines: number }
  | { type: 'vboard_ready'; layout: BoardLayout }
  | { type: 'vboard_state'; state: BoardState }
  | { type: 'vboard_frame'; frame: { device: string; width: number; height: number; png: string } }
  | { type: 'vboard_output'; lines: string[]; omitted: number }
  | { type: 'finished'; result: BoardRun }
  | { type: 'failed'; error: IpcError };

/** `toolchain.rs`. */
export type ToolchainOrigin = 'settings' | 'environment' | 'bundled' | 'installation' | 'path' | 'beside';

export interface ToolStatus {
  name: string;
  component: string | null;
  path: string | null;
  system: boolean;
  error: IpcError | null;
}

export interface ToolchainInfo {
  found: boolean;
  error: IpcError | null;
  origin: ToolchainOrigin | null;
  root: string | null;
  bundle: string | null;
  platform: string | null;
  components: BundleComponent[];
  not_installed: string[];
  tools: ToolStatus[];
  system_compiler: SystemCompiler | null;
  compiler_error: IpcError | null;
  /** O Quartus Prime do sistema, que compila para as placas Intel. */
  quartus: Quartus | null;
  quartus_error: IpcError | null;
  aurora_cli: string | null;
  /** O cliente web do Surfer do bundle (`surfer-aurora/web`), para a onda
   * numa aba; `null` num bundle que não o traz. */
  surfer_web: string | null;
}

/** `ListPosition` do Core (`Project::reorder_file`, `aurora order`). */
export type ListPosition =
  | { kind: 'first' }
  | { kind: 'last' }
  | { kind: 'before'; path: string }
  | { kind: 'after'; path: string };

/** `wave_tab.rs`: WaveTab. */
export interface WaveTab {
  id: string;
  url: string;
  processors: WaveProcessor[];
  /** O layout salvo no projeto (`wave/<testbench>.surf.ron`), onde o Ctrl+S
   * da aba salva; `null` numa onda fora de projeto. */
  saved_layout: string | null;
  /** O usuário salvou o layout depois que a AURORA o gerou. */
  customized: boolean;
  /** A escolha de sinais que o layout segue; vazia: todos. */
  selection: string[];
}

/** `terminal.rs`: TerminalMessage. */
export type TerminalMessage = { type: 'data'; data: string } | { type: 'exit'; code: number | null };

/** `aurora_agent::AgentEvent` (assistant.rs): o que acontece numa sessão do
 * assistente, na ordem. */
export type AgentEvent =
  | { type: 'ready'; session_id: string; model: string | null; tools: string[]; aurora_connected: boolean | null }
  | { type: 'text'; text: string }
  | { type: 'tool_use'; id: string; name: string; input: unknown }
  | { type: 'tool_result'; id: string; content: string; is_error: boolean }
  | {
      type: 'done';
      is_error: boolean;
      text: string | null;
      cost_usd: number | null;
      duration_ms: number | null;
      turns: number | null;
      session_id: string | null;
      input_tokens: number | null;
      output_tokens: number | null;
    }
  | { type: 'log'; line: string }
  | { type: 'exited'; code: number | null };

/** `assistant.rs`: AssistantInfo. */
export interface AssistantInfo {
  claude: string | null;
  version: string | null;
  aurora: string | null;
  project: string | null;
}

/** `watcher.rs`: o evento `studio://fs-changed`. */
export interface FsChanged {
  paths: string[];
}

/** O retrato da pasta de exercícios, que `learn.status`, `learn.init` e
 * `learn.set_current` devolvem: o `WorkspaceSnapshot` do aurora-learn, gerado
 * do protocolo. */
export type LearnSnapshot = WorkspaceSnapshot;
/** Um capítulo de {@link LearnSnapshot}. */
export type LearnChapter = ChapterSnapshot;
/** Um exercício de {@link LearnChapter}. */
export type LearnExercise = ExerciseSnapshot;
