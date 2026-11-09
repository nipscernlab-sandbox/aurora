// As chamadas ao backend, com os tipos de entrada e saída. O resto da
// interface só fala com o backend por aqui. Cada função chama um comando
// Tauri do Studio ou, por `aurora()`, um método do protocolo da AURORA, que o
// backend repassa ao servidor (`aurora serve`). Referência: docs/IPC.md.

import { Channel, invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type {
  Account,
  AddedFile,
  Board,
  Branches,
  CommitFiles,
  CommitResult,
  DeviceCode,
  DiscardResult,
  FileVersion,
  GitLog,
  GitStatus,
  InitResult,
  LoginPoll,
  Revision,
  SyncResult,
  BoardInput,
  BoardLayout,
  Reconnect,
  Cleanup,
  ExperimentalReport,
  Feature,
  FileMismatch,
  NameCheck,
  ProcRemoveReport,
  RemoveOutcome,
  TopOutcome,
  WaveChoiceOutcome,
  ProjectFile,
  RunComparison,
  RunRecord,
  RunSummary,
  WaveSignals,
} from './aurora-types';
import type {
  AppInfo,
  CleanReport,
  CleanupRequest,
  DirEntry,
  FileStat,
  FlowRequest,
  BitstreamState,
  FpgaConfig,
  FpgaPrepared,
  FsChanged,
  HierarchyResult,
  IpcError,
  JobMessage,
  LearnSnapshot,
  LearnTracks,
  ListPosition,
  ModuleInterface,
  MovedPath,
  LayoutConversion,
  NewProcessorDefaults,
  NewProcessorRequest,
  Processor,
  ProjectSnapshot,
  RecentStatus,
  SearchMatch,
  Settings,
  AgentEvent,
  AssistantInfo,
  TerminalMessage,
  TextFile,
  ToolchainInfo,
  VboardMessage,
  WaveOpened,
  WaveTab,
} from './types';

/** Normaliza o que um `invoke` rejeitou para um `IpcError`. */
export function toIpcError(error: unknown): IpcError {
  if (error && typeof error === 'object' && 'code' in error && 'message' in error) {
    return error as IpcError;
  }
  return { code: 'internal', message: String(error) };
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toIpcError(error);
  }
}

/** Um método do protocolo da AURORA, pelo servidor do Studio (`server.rs`):
 * o mesmo pedido e a mesma resposta do `aurora serve` (docs/PROTOCOL.md). Sem
 * `project` nos parâmetros, vale o projeto aberto. */
function aurora<T>(method: string, params?: Record<string, unknown>): Promise<T> {
  return call<T>('aurora_call', { method, params: params ?? null });
}

/** O `NameCheck` do protocolo como a interface o usa: `null` se o nome
 * serve, senão o erro. */
function nameError(check: NameCheck): IpcError | null {
  if (check.ok) return null;
  return { code: check.error?.code ?? 'invalid_name', message: check.error?.message ?? '' };
}

/** A limpeza do histórico como a interface a escreve, no formato do protocolo. */
function cleanupOf(request: CleanupRequest): Cleanup {
  switch (request.kind) {
    case 'all':
      return { kind: 'all' };
    case 'keep_latest':
      return { kind: 'keep_latest', value: request.keep };
    case 'reports':
      return { kind: 'reports', value: request.ids };
  }
}

export interface ReportView {
  id: string;
  path: string;
  record: RunRecord;
  text: string;
}

export const api = {
  app: {
    info: () => call<AppInfo>('app_info'),
    settings: () => call<Settings>('settings_get'),
    saveSettings: (settings: Settings) => call<Settings>('settings_set', { settings }),
    recent: () => call<RecentStatus[]>('recent_projects'),
    forgetRecent: (spf: string) => call<void>('recent_forget', { spf }),
    /** Abre uma onda no surfer-aurora: com `path`, aquele arquivo (a onda de
     * um exercício do `aurora learn` vem com o layout da correção); sem, a da
     * simulação do processador ou, sem processador, a do projeto. Um arquivo
     * vai com `project: null`, para não esperar na fila do projeto aberto. */
    openWave: (processor?: string | null, path?: string | null) =>
      aurora<WaveOpened>(
        'wave.open',
        path
          ? { project: null, request: { source: { kind: 'file', path } } }
          : { request: { source: { kind: 'project', processor: processor ?? null } } },
      ),
    /** Fecha o Studio e, com `restart`, abre ele de novo (depois do `aurora
     * update`). Quem chama grava os arquivos antes. */
    exit: (restart: boolean) => call<void>('app_exit', { restart }),
    /** Fecha o splash, depois de a janela principal aparecer. */
    closeSplash: () => call<void>('splash_close'),
  },

  /** A onda numa aba (`wave_tab.rs`). */
  waveTab: {
    open: (path: string) => call<WaveTab>('wave_tab_open', { path }),
    close: (id: string) => call<void>('wave_tab_close', { id }),
  },

  wave: {
    /** A árvore de sinais do testbench do projeto, com a escolha gravada. */
    signals: () => aurora<WaveSignals>('wave.signals'),
    /** Grava a escolha (vazia: todos os sinais) e devolve como ficou. Os
     * caminhos são conferidos na árvore do testbench elaborado. */
    setSelection: (signals: string[]) =>
      aurora<WaveChoiceOutcome>('wave.choose', { choice: { kind: 'set', paths: signals } }).then(
        (outcome) => outcome.selection,
      ),
    /** Apaga o layout salvo da onda; `false` se não havia. */
    resetLayout: (waveform: string) => aurora<boolean>('wave.reset_layout', { waveform }),
  },

  project: {
    defaultParent: () => call<string>('project_default_parent'),
    open: (path: string) => call<ProjectSnapshot>('project_open', { path }),
    create: (parent: string, name: string) =>
      call<ProjectSnapshot>('project_create', { parent, name }),
    close: () => call<void>('project_close'),
    snapshot: () => aurora<ProjectSnapshot>('project.status'),
    addVerilog: (path: string, testbench: boolean) =>
      aurora<AddedFile[]>('project.add', { files: [path], testbench }).then((added) => added[0]),
    /** `false` se o arquivo não estava no projeto. */
    removeVerilog: (path: string) =>
      aurora<RemoveOutcome>('project.remove', { files: [path] }).then((r) => r.removed.length > 0),
    /** Escolhe o topo por arquivo ou por módulo; devolve o arquivo de topo. */
    setTop: (target: string) =>
      aurora<TopOutcome>('project.top', { target }).then((r) => r.top_level ?? ''),
    setTestbench: (path: string) => aurora<ProjectSnapshot>('project.testbench', { path }),
    processorDefaults: () => aurora<NewProcessorDefaults>('processor.defaults'),
    /** Campo ausente fica com o padrão da AURORA; sem `nubits`, com
     * `nbmant` ou `nbexpo` dados, ele vira `nbmant + nbexpo + 1`. */
    addProcessor: (request: NewProcessorRequest) => aurora<Processor>('processor.add', { spec: request }),
    /** Tira o processador do projeto (`aurora proc remove`); com `toTrash`, a
     * pasta dele vai para a lixeira, e sem, fica no disco. */
    removeProcessor: (name: string, toTrash: boolean) =>
      aurora<ProcRemoveReport>('processor.remove', { name, to_trash: toTrash }),
    configureProcessor: (
      name: string,
      config: { frequency_mhz?: number | null; clocks?: number | null; show_arrays?: boolean | null },
    ) =>
      aurora<Processor>('processor.set', {
        name,
        config: {
          frequency_mhz: config.frequency_mhz ?? null,
          clocks: config.clocks ?? null,
          show_arrays: config.show_arrays ?? null,
        },
      }),
    outputValues: (processor: string, port: number) =>
      aurora<number[]>('processor.outputs', { processor, port }),
    verilogModules: (text: string) => aurora<string[]>('verilog.modules', { text }),
    /** Move ou renomeia (`Project::move_path` do Core): `to` é o caminho
     * final. Erro `path_exists` se ele já existe, sem `overwrite`; com, o
     * destino vai para a lixeira, menos o que a AURORA mantém no lugar
     * (`cannot_replace`). */
    move: (from: string, to: string, overwrite = false) =>
      aurora<MovedPath>('project.move', { from, to, overwrite }),
    /** Converte da estrutura de pastas do YANC 6.x para a do 7.1
     * (`Project::convert_layout`). */
    convertLayout: () => aurora<LayoutConversion>('project.convert'),
    /** `null` se o nome de projeto serve; senão o erro do Core. */
    checkName: (name: string) => aurora<NameCheck>('project.check_name', { name }).then(nameError),
    /** Muda a posição de um arquivo na lista dele (`aurora order`); devolve
     * a lista na ordem nova. */
    reorder: (path: string, position: ListPosition) =>
      aurora<ProjectFile[]>('project.order', { file: path, position }),
    /** `null` se o nome de processador serve; senão o erro do Core. */
    checkProcessorName: (name: string) =>
      aurora<NameCheck>('processor.check_name', { name }).then(nameError),
    hierarchy: () => call<HierarchyResult>('project_hierarchy'),
  },

  fs: {
    readDir: (path: string) => call<DirEntry[]>('fs_read_dir', { path }),
    readText: (path: string) => call<TextFile>('fs_read_text', { path }),
    writeText: (path: string, content: string, expectedModifiedMs?: number | null) =>
      call<number>('fs_write_text', {
        path,
        content,
        expectedModifiedMs: expectedModifiedMs ?? null,
      }),
    stat: (path: string) => call<FileStat | null>('fs_stat', { path }),
    createFile: (path: string, content?: string) =>
      call<void>('fs_create_file', { path, content: content ?? null }),
    createDir: (path: string) => call<void>('fs_create_dir', { path }),
    rename: (from: string, to: string) => call<void>('fs_rename', { from, to }),
    /** Copia um arquivo ou pasta (de qualquer lugar) para dentro de `toDir`. */
    copy: (from: string, toDir: string, overwrite = false) =>
      call<string>('fs_copy', { from, toDir, overwrite }),
    trash: (path: string) => call<void>('fs_trash', { path }),
    listFiles: () => call<string[]>('fs_list_files'),
    search: (query: string, regex: boolean, caseSensitive: boolean, maxResults?: number) =>
      call<SearchMatch[]>('fs_search', {
        query,
        regex,
        caseSensitive,
        maxResults: maxResults ?? null,
      }),
  },

  history: {
    list: () => aurora<RunSummary[]>('history.list'),
    show: (id?: string | null) => aurora<ReportView>('history.show', { id: id ?? null }),
    compare: (id?: string | null, against?: string | null) =>
      aurora<RunComparison>('history.compare', { id: id ?? null, against: against ?? null }),
    planCleanup: (cleanup: CleanupRequest) =>
      aurora<string[]>('history.plan_cleanup', { cleanup: cleanupOf(cleanup) }),
    clean: (ids: string[]) => aurora<CleanReport>('history.clean', { ids }),
  },
  /** Os exercícios do `aurora learn` (commands/learn.rs); a correção é o fluxo `learn`. */
  learn: {
    tracks: () => aurora<LearnTracks>('learn.tracks'),
    /** Abre a pasta de exercícios que contém `root` e a põe em dia com a
     * trilha (`learn.status`). */
    open: (root: string, lang?: string | null) => aurora<LearnSnapshot>('learn.status', { root, lang: lang ?? null }),
    init: (dir: string, track?: string | null, lang?: string | null) =>
      aurora<LearnSnapshot>('learn.init', { dir, track: track ?? null, lang: lang ?? null }),
    setCurrent: (root: string, name: string, lang?: string | null) =>
      aurora<LearnSnapshot>('learn.set_current', { root, name, lang: lang ?? null }),
    reset: (root: string, name: string) => aurora<void>('learn.reset', { root, name }),
  },

  /** Os recursos experimentais da AURORA, pelo servidor: o mesmo
   * `experimental.json` do `aurora experimental` da CLI. */
  experimental: {
    list: () => aurora<ExperimentalReport>('experimental.list'),
    set: (feature: Feature, enabled: boolean) =>
      aurora<ExperimentalReport>('experimental.set', { feature, enabled }),
  },

  /** A placa FPGA, experimental: desligada, cada método daqui responde
   * `experimental_disabled`. Tudo passa pelo servidor; a compilação é o
   * fluxo `fpga_build`. */
  fpga: {
    boards: () => aurora<Board[]>('fpga.boards'),
    /** O `fpga.json`; `null` se ainda não existe. */
    config: () => aurora<FpgaConfig | null>('fpga.config'),
    setConfig: (config: FpgaConfig) => aurora<void>('fpga.config_set', { config }),
    /** Muda uma ligação a partir da placa (um bit, ou o sinal inteiro, numa
     * porta do topo) e grava o `fpga.json`; sem ele, parte das portas ligadas
     * pelo nome com `board`. */
    connect: (change: Reconnect, board?: string | null) =>
      aurora<FpgaConfig>('fpga.connect', { change, board: board ?? null }),
    /** O topo que vai para a placa e as portas dele. */
    top: (top?: string | null) => aurora<ModuleInterface>('fpga.top', { top: top ?? null }),
    check: () => aurora<FpgaPrepared>('fpga.check'),
    /** Os módulos do projeto que podem ir para a placa. */
    modules: () => aurora<string[]>('fpga.modules'),
    /** O arquivo de gravação: pronto, desatualizado (e por quê) ou ausente. */
    status: () => aurora<BitstreamState>('fpga.status'),
    /** Os cabos de gravação ligados (`quartus_pgm -l`, cerca de 1 s). */
    cables: () => aurora<string[]>('fpga.cables'),
  },

  /** O controle de versão, experimental (`git`): desligado, cada método
   * daqui responde `experimental_disabled`. É o git instalado na máquina,
   * pelo servidor; os caminhos dos arquivos são relativos ao topo do
   * repositório, com `/`. */
  git: {
    /** O git da máquina e o repositório que contém o projeto; fora de
     * repositório, `repository` vem `null`. */
    status: () => aurora<GitStatus>('git.status'),
    readFile: (revision: Revision, path: string) => aurora<FileVersion>('git.read_file', { revision, path }),
    /** Commit exatamente dos `paths` (os marcados na vista). */
    commit: (summary: string, description: string | null, paths: string[]) =>
      aurora<CommitResult>('git.commit', { summary, description, paths }),
    /** A versão descartada vai para a lixeira do sistema. */
    discard: (paths: string[]) => aurora<DiscardResult>('git.discard', { paths }),
    init: () => aurora<InitResult>('git.init'),
    log: (skip: number, limit: number) => aurora<GitLog>('git.log', { skip, limit }),
    commitFiles: (oid: string) => aurora<CommitFiles>('git.commit_files', { oid }),
    branches: () => aurora<Branches>('git.branches'),
    /** Troca de ramo (um remoto `origin/x` vira o local `x`) ou, com
     * `create`, cria um a partir do commit atual. Volta o ramo atual. */
    switchTo: (branch: string, create = false) => aurora<string>('git.switch', { branch, create }),
    fetch: () => aurora<SyncResult>('git.fetch'),
    pull: () => aurora<SyncResult>('git.pull'),
    /** Envia; sem ramo remoto, publica. */
    push: () => aurora<SyncResult>('git.push'),
    setIdentity: (name: string, email: string, global: boolean) =>
      aurora<null>('git.set_identity', { name, email, global }),
    setRemote: (name: string, url: string) => aurora<null>('git.set_remote', { name, url }),
    removeRemote: (name: string) => aurora<null>('git.remove_remote', { name }),
  },

  /** A conta do GitHub do git desta máquina (recurso `git`). */
  github: {
    /** `null` sem login; o servidor guarda a resposta por 10 minutos. */
    account: () => aurora<Account | null>('github.account'),
    loginStart: () => aurora<DeviceCode>('github.login_start'),
    loginPoll: (deviceCode: string) => aurora<LoginPoll>('github.login_poll', { device_code: deviceCode }),
    /** `true` se havia um login. */
    logout: () => aurora<boolean>('github.logout'),
  },

  /** A placa virtual (da placa FPGA, experimental): o projeto com os
   * periféricos da placa simulados. A sessão é o `vboard.run` do servidor,
   * por um canal próprio (`vboard.rs`), fora das operações. */
  vboard: {
    /** A placa e os periféricos, ligados ou não, sem compilar. `board` vale
     * num projeto sem `fpga.json`: as portas com o nome de um sinal dela
     * ligam a ele. */
    layout: (board?: string | null) => aurora<BoardLayout>('vboard.layout', { board: board ?? null }),
    /** Compila e liga a placa; as mensagens chegam em `onMessage`, começando
     * por `started` (com o `run` que as entradas citam). */
    start: (
      options: { board?: string | null; limit?: number | null; fps?: number | null },
      onMessage: (m: VboardMessage) => void,
    ) => {
      const channel = new Channel<VboardMessage>();
      channel.onmessage = onMessage;
      return call<void>('vboard_start', {
        board: options.board ?? null,
        limit: options.limit ?? null,
        fps: options.fps ?? null,
        channel,
      });
    },
    input: (run: number, input: BoardInput) => aurora<void>('vboard.input', { run, input }),
    stop: (run: number) => call<void>('vboard_stop', { run }),
  },

  toolchain: {
    info: () => call<ToolchainInfo>('toolchain_info'),
    verify: () => aurora<FileMismatch[]>('toolchain.verify'),
    updateCheck: () => call<unknown>('aurora_update_check'),
    install: (components: string[], onMessage: (m: JobMessage) => void) => {
      const channel = new Channel<JobMessage>();
      channel.onmessage = onMessage;
      return call<number>('aurora_install', { components, channel });
    },
    /** `aurora update --yes --json`: só depois de o usuário confirmar. */
    update: (onMessage: (m: JobMessage) => void) => {
      const channel = new Channel<JobMessage>();
      channel.onmessage = onMessage;
      return call<number>('aurora_update', { channel });
    },
  },

  flow: {
    /** Começa um fluxo; as mensagens chegam em `onMessage`, terminando em
     * `finished` ou `failed`. Devolve o identificador da operação. */
    start: (request: FlowRequest, onMessage: (m: JobMessage) => void) => {
      const channel = new Channel<JobMessage>();
      channel.onmessage = onMessage;
      return call<number>('flow_start', { request, channel });
    },
    cancel: (job?: number | null) => call<boolean>('flow_cancel', { job: job ?? null }),
    running: () => call<number | null>('flow_running'),
  },

  assistant: {
    info: () => call<AssistantInfo>('assistant_info'),
    /** Abre uma sessão; os eventos chegam por `onEvent`, começando por `ready`. */
    start: (resume: string | null, onEvent: (event: AgentEvent) => void) => {
      const channel = new Channel<AgentEvent>();
      channel.onmessage = onEvent;
      return call<number>('assistant_start', { resume, channel });
    },
    send: (id: number, text: string) => call<void>('assistant_send', { id, text }),
    interrupt: (id: number) => call<void>('assistant_interrupt', { id }),
    stop: (id: number) => call<void>('assistant_stop', { id }),
  },

  terminal: {
    spawn: (
      cwd: string | null,
      cols: number,
      rows: number,
      onMessage: (m: TerminalMessage) => void,
    ) => {
      const channel = new Channel<TerminalMessage>();
      channel.onmessage = onMessage;
      return call<number>('terminal_spawn', { cwd, cols, rows, channel });
    },
    write: (id: number, data: string) => call<void>('terminal_write', { id, data }),
    /** O xterm escreveu `units` (o `length` das strings) do que o terminal
     * mandou: sem as confirmações, o backend para de ler o shell. */
    ack: (id: number, units: number) => call<void>('terminal_ack', { id, units }),
    /** Digita no shell o `cd` para `path`, na sintaxe do shell dele. */
    cd: (id: number, path: string) => call<void>('terminal_cd', { id, path }),
    resize: (id: number, cols: number, rows: number) =>
      call<void>('terminal_resize', { id, cols, rows }),
    kill: (id: number) => call<void>('terminal_kill', { id }),
  },

  events: {
    onFsChanged: (handler: (event: FsChanged) => void): Promise<UnlistenFn> =>
      listen<FsChanged>('studio://fs-changed', (e) => handler(e.payload)),
  },
};
