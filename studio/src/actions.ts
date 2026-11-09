// Todas as ações do Studio num lugar só. Os menus, a barra de ferramentas,
// o navegador de fluxo, a paleta de comandos e os atalhos de teclado leem
// daqui: uma ação nova aparece em todos de uma vez. Uma ação de recurso
// experimental desligado (`feature`, state/experimental.ts) some de todos.
//
// Os atalhos seguem os da AURORA onde ela tinha (F5 a F10, Shift+F5,
// Ctrl+Alt+P, Ctrl+Alt+N, Ctrl+Shift+O, Ctrl+Shift+P) e os do VS Code no
// resto (Ctrl+P, Ctrl+B, Ctrl+J, Ctrl+`, Ctrl+,). A tabela completa está em
// docs/USER_GUIDE.md.

import { isTauri } from '@tauri-apps/api/core';
import { open as openDialogNative } from '@tauri-apps/plugin-dialog';
import { openUrl, revealItemInDir } from '@tauri-apps/plugin-opener';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { LucideIcon } from 'lucide-react';
import {
  Activity,
  Boxes,
  CircuitBoard,
  Microchip,
  MonitorPlay,
  Upload,
  CircleStop,
  Columns2,
  Cpu,
  FilePlus,
  FileCode,
  FolderOpen,
  FolderPlus,
  Bot,
  CircleUserRound,
  GitBranch,
  GraduationCap,
  Hammer,
  ListChecks,
  ListFilter,
  Play,
  Save,
  Search,
  Settings,
  SquareTerminal,
  Workflow,
  Zap,
} from 'lucide-react';
import { create } from 'zustand';

import { activeEditor } from './editor/host';
import { vimActive } from './editor/vim';
import { t, type Key } from './i18n';
import { api } from './ipc/api';
import type { Feature, Simulator } from './ipc/aurora-types';
import { useApp } from './state/app';
import { confirm, openDialog, prompt } from './state/dialogs';
import { MAX_GROUPS, saveActive, useEditor } from './state/editor';
import { featureOn } from './state/experimental';
import { useHierarchy } from './state/hierarchy';
import { useJobs } from './state/jobs';
import { isOnScreen, useLayout, type RegionId, type ViewId } from './state/layout';
import { currentExercise, useLearn } from './state/learn';
import { CONSOLE_CHANNELS, regionOf, TOOLBAR_STYLES, VIEW_INFO, type ToolbarStyle } from './state/layoutModel';
import { convertLayout, useProject } from './state/project';
import { resetLayout, restoreLayout, saveLayout, saveLayoutAs } from './state/savedLayouts';
import { useSettingsPage } from './state/settingsPage';
import { guarded, showError, useToasts } from './state/toasts';
import { useTour } from './state/tour';
import { openWaveTab, waveInTab } from './state/waves';
import { toggledMode } from './themes';
import { baseName, extension, isInside, joinPath, relativeTo, resolveFrom } from './util/paths';

export type ActionCategory = 'file' | 'edit' | 'view' | 'project' | 'flow' | 'tools' | 'help';

export interface Action {
  id: string;
  /** O nome na paleta de comandos e, sem `menuLabel`, nos menus. */
  label: Key;
  /** O nome nos menus, quando é outro: um item que liga e desliga se chama
   * pelo que mostra ("Barra de status", com a marca), e na paleta pelo que
   * faz ("Mostrar ou ocultar a barra de status"). */
  menuLabel?: Key;
  category: ActionCategory;
  keys?: string;
  icon?: LucideIcon;
  enabled?: () => boolean;
  /** Liga e desliga: os menus mostram a marca quando é `true`. */
  checked?: () => boolean;
  /** O recurso experimental de que a ação faz parte: desligado, ela não
   * aparece em lugar nenhum e não roda. */
  feature?: Feature;
  run: () => unknown;
}

// Condições -------------------------------------------------------------

const hasProject = () => useProject.getState().snapshot !== null;
/** Há projeto e o `.spf` dele existe: toda operação de projeto (compilar,
 * simular, mexer nas fontes, o histórico) precisa dele. Os arquivos da
 * pasta, a busca e o fechar continuam sem ele (`spfMissing`). */
const hasSpf = () => hasProject() && !useProject.getState().spfMissing;
const idle = () => useJobs.getState().running === null;
const canRun = () => hasSpf() && idle();
const hasEditor = () => activeEditor() !== null;
const hasLearnExercise = () => currentExercise(useLearn.getState().snapshot) !== null;

/** Mostra uma vista que o usuário pediu, mesmo se estava escondida, na
 * região onde o layout a pôs. */
function reveal(view: ViewId) {
  useLayout.getState().revealView(view, { explicit: true });
}

const live = () => useLayout.getState().live;
const regionShown = (region: RegionId) => live().regions[region].visible;

/** O lado da barra de atividades antes de ela ser escondida, para voltar a
 * ele (na sessão; depois de reabrir, volta à esquerda). */
let activitySide: 'left' | 'right' = 'left';

function toggleActivityBar() {
  const side = live().bars.activitybar;
  if (side === 'hidden') {
    useLayout.getState().setBar('activitybar', activitySide);
  } else {
    activitySide = side;
    useLayout.getState().setBar('activitybar', 'hidden');
  }
}

function toggleBar(bar: 'menubar' | 'toolbar' | 'statusbar') {
  useLayout.getState().setBar(bar, !live().bars[bar]);
}

/** As ações que escolhem o modelo da barra de ferramentas. */
export const TOOLBAR_STYLE_ACTIONS: Record<ToolbarStyle, string> = {
  classic: 'toolbarClassic',
  titlebar: 'toolbarTitlebar',
  editor: 'toolbarEditor',
};

/** Troca o modelo da barra de ferramentas e a mostra, se estava escondida:
 * quem escolhe um modelo quer vê-lo. */
function setToolbarStyle(style: ToolbarStyle) {
  useLayout.getState().setBar('toolbarStyle', style);
  if (!live().bars.toolbar) useLayout.getState().setBar('toolbar', true);
}

/** Preferências > Layout da janela: as Preferências abertas nessa página. */
function customizeLayout() {
  useSettingsPage.getState().setPage('layout');
  useEditor.getState().openView('settings');
}

/** O componente está no bundle. Sem a informação do bundle ainda, `true`:
 * o erro, se houver, vem da operação. */
export function hasComponent(name: string): boolean {
  const toolchain = useApp.getState().toolchain;
  if (!toolchain?.found) return true;
  return toolchain.components.some((c) => c.name === name);
}

/** A onda do alvo (o projeto ou o processador escolhido) já existe. */
function hasWave(): boolean {
  const snapshot = useProject.getState().snapshot;
  if (!snapshot || useProject.getState().spfMissing) return false;
  const name = target();
  if (!name) return snapshot.waveform !== null;
  return snapshot.processors.some((p) => p.name === name && p.waveform !== null);
}

// Fluxos ------------------------------------------------------------------

function target(): string | null {
  return useProject.getState().target;
}

function settings() {
  return useApp.getState().settings;
}

export function runBuild(processor?: string | null) {
  const name = processor === undefined ? target() : processor;
  return useJobs.getState().run({ flow: 'build', processors: name ? [name] : [] }, name ? `build:${name}` : 'build');
}

/** Verifica o projeto ou, com um processador no alvo, só ele e o testbench
 * que o YANC gerou (`aurora check -p`). Um arquivo pedido vale para o projeto. */
export function runCheck(lint = false, file?: string) {
  const name = file ? null : target();
  const key = file ? `check:${file}` : `${lint ? 'lint' : 'check'}${name ? `:${name}` : ''}`;
  return useJobs.getState().run({ flow: 'check', lint, file: file ?? null, processor: name }, key);
}

/** A simulação com onda (Wave, F8), no simulador das preferências; abre a
 * onda no fim com `openWave`. */
export function runSimulation(openWave: boolean, processor?: string | null) {
  const name = processor === undefined ? target() : processor;
  const s = settings();
  return useJobs.getState().run(
    {
      flow: 'simulate',
      processor: name,
      simulator: s?.simulator ?? 'icarus',
      timeout_s: s?.sim_timeout_s ?? null,
      open_wave: openWave,
    },
    name ? `simulate:${name}` : 'simulate',
  );
}

/** O simulador da simulação rápida do alvo (ou do processador dado), pela
 * regra do Core (`SimulationOptions::fast`): o Verilator, menos para um
 * testbench cocotb, que roda os testes no simulador das preferências. */
export function fastSimulator(processor?: string | null): Simulator {
  const name = processor === undefined ? target() : processor;
  const testbench = useProject.getState().snapshot?.selected_testbench;
  if (!name && testbench && extension(testbench) === 'py') return settings()?.simulator ?? 'icarus';
  return 'verilator';
}

/** A simulação rápida (Rápida, F9), o Fast Sim da AURORA: roda sem gravar
 * onda, para ver a saída do testbench, as portas do processador e os testes
 * cocotb na velocidade do simulador (`fastSimulator`). */
export function runFastSimulation(processor?: string | null) {
  const name = processor === undefined ? target() : processor;
  const s = settings();
  return useJobs.getState().run(
    {
      flow: 'simulate',
      processor: name,
      simulator: s?.simulator ?? 'icarus',
      fast: true,
      timeout_s: s?.sim_timeout_s ?? null,
    },
    name ? `fastSim:${name}` : 'fastSim',
  );
}

/** A síntese (RTLViewer, F10). O esquemático é o Studio que desenha, a partir
 * do netlist (src/schematic); o do Graphviz (`schematic: true`) fica para a
 * CLI. */
export function runSynthesis(processor?: string | null) {
  const name = processor === undefined ? target() : processor;
  return useJobs.getState().run(
    { flow: 'synthesize', processor: name, schematic: false },
    name ? `synthesize:${name}` : 'synthesize',
  );
}

/** A compilação para a placa do `fpga.json`, pelo Quartus (`aurora fpga build`). */
export function runFpgaBuild() {
  return useJobs.getState().run({ flow: 'fpga_build' }, 'fpgaBuild');
}

/** A gravação na placa do `.sof` da última compilação (`aurora fpga program`). */
export function runFpgaProgram() {
  return useJobs.getState().run({ flow: 'fpga_program' }, 'fpgaProgram');
}

/** Compila para a placa e, se deu certo, grava: o caminho de sempre. */
export async function runFpgaBuildAndProgram() {
  const built = await runFpgaBuild();
  if (built?.succeeded) await runFpgaProgram();
}

/** A onda do processador (ou do projeto, sem processador), se já existe. */
function wavePath(processor: string | null): string | null {
  const snapshot = useProject.getState().snapshot;
  if (!snapshot) return null;
  if (!processor) return snapshot.waveform;
  return snapshot.processors.find((p) => p.name === processor)?.waveform ?? null;
}

/** Abre a onda do alvo (ou do processador dado) onde a preferência manda:
 * numa aba ou em janela (`state/waves.ts`). */
export async function openWave(processor?: string | null) {
  const name = processor === undefined ? target() : processor;
  const path = wavePath(name);
  if (path && waveInTab()) {
    openWaveTab(path);
    return;
  }
  const opened = await guarded(() => api.app.openWave(name));
  if (opened) {
    useToasts.getState().push({ kind: 'info', title: t('console.waveOpened', { pid: opened.pid, path: baseName(opened.waveform) }) });
  }
}

/** A escolha de sinais da onda do projeto em poucas palavras, para os botões
 * que abrem a janela Sinais da onda: "todos", "1 marcado", "3 marcados". */
export function signalChoice(count: number): string {
  if (count === 0) return t('wave.choiceAll');
  return count === 1 ? t('wave.choiceOne') : t('wave.choiceSome', { count });
}

/** Abre `path` no surfer-aurora em janela, qualquer que seja a preferência. */
export async function openWaveWindow(path: string) {
  const opened = await guarded(() => api.app.openWave(null, path));
  if (opened) {
    useToasts.getState().push({ kind: 'info', title: t('console.waveOpened', { pid: opened.pid, path: baseName(opened.waveform) }) });
    for (const processor of opened.outdated) {
      useToasts.getState().push({ kind: 'warning', title: processor, detail: t('wave.outdated') });
    }
  }
}

async function fullFlow() {
  // O F5 da AURORA: compila tudo, verifica e, se passou, simula. A
  // verificação não compila os processadores, então o build vem antes.
  if ((useProject.getState().snapshot?.processors.length ?? 0) > 0) {
    const built = await runBuild();
    if (!built?.succeeded) return;
  }
  const checked = await runCheck(false);
  if (checked?.succeeded) await runSimulation(settings()?.open_wave_after_sim ?? true);
}

// Arquivos e projeto --------------------------------------------------

/** A pasta de projetos: a última escolhida no "Novo projeto", ou a padrão
 * (`Projetos` na pasta do usuário, que o backend cria). A padrão não fica
 * guardada, para que mudá-la valha também para quem já usou o Studio. */
export async function projectsFolder() {
  const saved = localStorage.getItem('aurora-ide:projectsParent');
  if (saved) return saved;
  return isTauri() ? api.project.defaultParent() : 'Projetos';
}

export async function openProjectDialog() {
  if (!isTauri()) return;
  const defaultPath = await projectsFolder();
  const chosen = await openDialogNative({
    multiple: false,
    directory: false,
    defaultPath,
    filters: [{ name: 'AURORA (.spf)', extensions: ['spf'] }],
  });
  if (typeof chosen === 'string') await useProject.getState().open(chosen);
}

async function openAnyFile() {
  if (!isTauri()) return;
  const chosen = await openDialogNative({ multiple: false, directory: false, defaultPath: await projectsFolder() });
  if (typeof chosen === 'string') await useEditor.getState().openFile(chosen);
}

/** Cria um arquivo vazio em `dir` (a raiz do projeto, sem `dir`). */
export async function newFileIn(dir?: string) {
  const root = useProject.getState().snapshot?.root;
  if (!root) return;
  const name = await prompt({ title: t('dialog.newFile.title'), placeholder: 'rtl/modulo.v' });
  if (!name) return;
  const path = resolveFrom(dir ?? root, name);
  if ((await guarded(() => api.fs.createFile(path))) === undefined) return;
  useProject.getState().bumpTree();
  await useEditor.getState().openFile(path);
}

export async function newFolderIn(dir?: string) {
  const root = useProject.getState().snapshot?.root;
  if (!root) return;
  const name = await prompt({ title: t('dialog.newFolder.title') });
  if (!name) return;
  if ((await guarded(() => api.fs.createDir(resolveFrom(dir ?? root, name)))) !== undefined) {
    useProject.getState().bumpTree();
  }
}

/** Escolhe arquivos .v e .sv, e testbenches cocotb (.py), no disco e os
 * registra (`aurora add`). */
export async function addVerilogFiles(paths?: string[]) {
  const snapshot = useProject.getState().snapshot;
  if (!snapshot || (!paths && !isTauri())) return;
  let files = paths;
  if (!files) {
    const chosen = await openDialogNative({
      multiple: true,
      directory: false,
      defaultPath: await projectsFolder(),
      filters: [
        { name: 'Verilog, cocotb', extensions: ['v', 'sv', 'vh', 'svh', 'py'] },
        { name: 'Verilog', extensions: ['v', 'sv', 'vh', 'svh'] },
        { name: 'cocotb (Python)', extensions: ['py'] },
      ],
    });
    files = Array.isArray(chosen) ? chosen : typeof chosen === 'string' ? [chosen] : [];
  }
  // Um arquivo: o aviso diz onde ele entrou. Vários (o + de "Fora do
  // projeto", uma seleção no diálogo): um aviso só, com as contas.
  const added = { module: 0, testbench: 0 };
  for (const file of files) {
    try {
      const result = await api.project.addVerilog(file, false);
      const testbench = result.role === 'testbench';
      added[testbench ? 'testbench' : 'module'] += 1;
      if (files.length === 1) {
        useToasts.getState().push({
          kind: 'success',
          title: baseName(result.path),
          detail: testbench ? t('explorer.testbenches') : t('explorer.modules'),
        });
      }
    } catch (error) {
      showError(error);
    }
  }
  const count = added.module + added.testbench;
  if (files.length > 1 && count > 0) {
    useToasts.getState().push({
      kind: 'success',
      title: t('explorer.addedMany', { count }),
      detail: t('explorer.addedManyDetail', { modules: added.module, testbenches: added.testbench }),
    });
  }
  await useProject.getState().refresh();
}

/** Tira um processador do projeto (`aurora proc remove`), depois de perguntar
 * o que fazer com a pasta dele: ficar no disco ou ir para a lixeira. A aba
 * do processador fecha; com a pasta na lixeira, fecham também as abas dos
 * arquivos de dentro dela, como no apagar do Explorador. */
export async function removeProcessor(name: string) {
  const snapshot = useProject.getState().snapshot;
  const processor = snapshot?.processors.find((p) => p.name === name);
  if (!snapshot || !processor) return;
  const folder = relativeTo(processor.dir, snapshot.root);
  const answer = await confirm({
    title: t('dialog.removeProcessor.title', { name }),
    message: t('dialog.removeProcessor.message', { folder }),
    // Três botões, um deles de texto longo: no diálogo estreito, não cabem.
    wide: true,
    buttons: [
      { label: t('dialog.removeProcessor.keep'), value: 'keep', primary: true },
      { label: t('dialog.removeProcessor.trash'), value: 'trash', danger: true },
      { label: t('common.cancel'), value: 'cancel' },
    ],
  });
  if (answer !== 'keep' && answer !== 'trash') return;
  const result = await guarded(() => api.project.removeProcessor(name, answer === 'trash'));
  if (!result) return;
  const editor = useEditor.getState();
  for (const tab of editor.tabs) {
    const own = tab.kind === 'processor' && tab.data?.name === name;
    const inside = result.trashed && !!tab.path && isInside(tab.path, processor.dir);
    if (!own && !inside) continue;
    if (tab.path) editor.setDirty(tab.path, false);
    await editor.closeTab(tab.id);
  }
  useToasts.getState().push({
    kind: 'success',
    title: t('project.processorRemoved', { name }),
    detail: t(result.trashed ? 'project.processorRemoved.trashed' : 'project.processorRemoved.kept', { folder }),
  });
  useProject.getState().bumpTree();
  await useProject.getState().refresh();
}

function applyZoom(zoom: number) {
  useLayout.getState().setZoom(zoom);
  void getCurrentWebview()
    .setZoom(useLayout.getState().zoom)
    .catch(() => undefined);
}

function editorCommand(id: string) {
  const editor = activeEditor();
  if (!editor) return;
  editor.focus();
  const action = editor.getAction(id);
  if (action) void action.run();
  else editor.trigger('menu', id, null);
}

// Links de ajuda: só os endereços que existem.
export const AURORA_CLI_DOCS = 'https://github.com/ART3121/lace/blob/main/docs/CLI.md';
export const SAPHO_MANUAL = 'https://nipscern.com/library/sapho';
export const NIPSCERN_SITE = 'https://nipscern.com';
export const NIPSCERN_GITHUB = 'https://github.com/nipscernlab';

// A lista ----------------------------------------------------------------

export const ACTIONS: Action[] = [
  // Arquivo
  { id: 'newProject', label: 'action.newProject', category: 'file', keys: 'Ctrl+Alt+N', icon: FolderPlus, run: () => openDialog({ kind: 'newProject' }) },
  { id: 'openProject', label: 'action.openProject', category: 'file', keys: 'Ctrl+Shift+O', icon: FolderOpen, run: openProjectDialog },
  { id: 'closeProject', label: 'action.closeProject', category: 'file', enabled: hasProject, run: () => useProject.getState().close() },
  {
    id: 'convertLayout',
    label: 'action.convertLayout',
    category: 'file',
    enabled: () => !!useProject.getState().snapshot?.issues.some((i) => i.kind === 'legacy_layout'),
    run: () => convertLayout(),
  },
  {
    id: 'revealProject',
    label: 'action.revealProject',
    category: 'file',
    enabled: hasProject,
    run: () => revealItemInDir(useProject.getState().snapshot!.spf).catch(showError),
  },
  { id: 'newFile', label: 'action.newFile', category: 'file', keys: 'Ctrl+N', icon: FilePlus, enabled: hasProject, run: () => newFileIn() },
  { id: 'newFolder', label: 'action.newFolder', category: 'file', enabled: hasProject, run: () => newFolderIn() },
  { id: 'openFile', label: 'action.openFile', category: 'file', keys: 'Ctrl+O', run: openAnyFile },
  { id: 'save', label: 'action.save', category: 'file', keys: 'Ctrl+S', icon: Save, run: saveActive },
  { id: 'saveAll', label: 'action.saveAll', category: 'file', keys: 'Ctrl+Shift+S', run: () => useEditor.getState().saveAll() },
  {
    id: 'closeTab',
    label: 'action.closeTab',
    category: 'file',
    keys: 'Ctrl+W',
    run: () => {
      const { activeId, activeGroup, closeTab } = useEditor.getState();
      if (activeId) void closeTab(activeId, activeGroup);
    },
  },
  { id: 'reopenTab', label: 'action.reopenTab', category: 'file', keys: 'Ctrl+Shift+T', run: () => useEditor.getState().reopenClosed() },
  { id: 'settings', label: 'action.settings', category: 'file', keys: 'Ctrl+,', icon: Settings, run: () => useEditor.getState().openView('settings') },
  { id: 'quit', label: 'action.quit', category: 'file', keys: 'Ctrl+Q', run: () => getCurrentWindow().close() },

  // Editar
  { id: 'undo', label: 'action.undo', category: 'edit', keys: 'Ctrl+Z', enabled: hasEditor, run: () => editorCommand('undo') },
  { id: 'redo', label: 'action.redo', category: 'edit', keys: 'Ctrl+Y', enabled: hasEditor, run: () => editorCommand('redo') },
  // Com um arquivo na frente, a busca no arquivo; com o RTLViewer, a busca
  // dele; sem nenhum dos dois (outra vista, o editor vazio), a Busca do
  // projeto.
  {
    id: 'find',
    label: 'action.find',
    category: 'edit',
    keys: 'Ctrl+F',
    enabled: () => hasEditor() || hasProject(),
    run: () => {
      if (hasEditor()) return editorCommand('actions.find');
      const { tabs, activeId } = useEditor.getState();
      const viewerSearch = document.querySelector<HTMLInputElement>('.rtlViewer-search input');
      if (tabs.find((tab) => tab.id === activeId)?.kind === 'schematic' && viewerSearch) {
        viewerSearch.focus();
        viewerSearch.select();
        return;
      }
      reveal('search');
    },
  },
  {
    id: 'replace',
    label: 'action.replace',
    category: 'edit',
    keys: 'Ctrl+H',
    enabled: hasEditor,
    run: () => editorCommand('editor.action.startFindReplaceAction'),
  },
  { id: 'findInFiles', label: 'action.findInFiles', category: 'edit', keys: 'Ctrl+Shift+F', icon: Search, enabled: hasProject, run: () => reveal('search') },
  { id: 'goToLine', label: 'action.goToLine', category: 'edit', keys: 'Ctrl+G', enabled: hasEditor, run: () => editorCommand('editor.action.gotoLine') },
  {
    id: 'toggleVim',
    label: 'action.toggleVim',
    category: 'edit',
    checked: () => !!settings()?.editor.vim_mode,
    run: () =>
      useApp.getState().updateSettings((s) => ({ ...s, editor: { ...s.editor, vim_mode: !s.editor.vim_mode } })),
  },
  {
    id: 'formatDocument',
    label: 'action.formatDocument',
    category: 'edit',
    keys: 'Shift+Alt+F',
    enabled: hasEditor,
    run: () => editorCommand('editor.action.formatDocument'),
  },

  // Exibir
  { id: 'commandPalette', label: 'action.commandPalette', category: 'view', keys: 'Ctrl+Shift+P', run: () => openDialog({ kind: 'palette' }) },
  { id: 'quickOpen', label: 'action.quickOpen', category: 'view', keys: 'Ctrl+P', enabled: hasProject, run: () => openDialog({ kind: 'quickOpen' }) },
  { id: 'viewExplorer', label: 'action.viewExplorer', category: 'view', keys: 'Ctrl+Shift+E', run: () => reveal('explorer') },
  { id: 'viewFlow', label: 'action.viewFlow', category: 'view', icon: Workflow, run: () => reveal('flow') },
  { id: 'viewSearch', label: 'action.viewSearch', category: 'view', run: () => reveal('search') },
  { id: 'viewReports', label: 'action.viewReports', category: 'view', run: () => reveal('reports') },
  { id: 'viewLearn', label: 'action.viewLearn', category: 'view', icon: GraduationCap, run: () => reveal('learn') },
  // O controle de versão e a conta do GitHub, experimentais.
  { id: 'viewGit', label: 'action.viewGit', category: 'view', keys: 'Ctrl+Shift+G', icon: GitBranch, feature: 'git', run: () => reveal('git') },
  { id: 'viewProfile', label: 'action.viewProfile', category: 'view', icon: CircleUserRound, feature: 'git', run: () => reveal('profile') },
  // O assistente de IA, experimental.
  { id: 'viewAssistant', label: 'action.viewAssistant', category: 'view', icon: Bot, feature: 'assistant', run: () => reveal('assistant') },
  { id: 'assistantTab', label: 'action.assistantTab', category: 'view', feature: 'assistant', run: () => useEditor.getState().openView('assistant') },
  {
    id: 'splitEditor',
    label: 'action.splitEditor',
    category: 'view',
    keys: 'Ctrl+\\',
    icon: Columns2,
    enabled: () => {
      const { activeId, groups } = useEditor.getState();
      return !!activeId && groups.length < MAX_GROUPS;
    },
    run: () => useEditor.getState().split(),
  },
  {
    id: 'closeEditorGroup',
    label: 'action.closeEditorGroup',
    category: 'view',
    enabled: () => useEditor.getState().groups.length > 1,
    run: () => useEditor.getState().closeGroup(useEditor.getState().activeGroup),
  },
  ...[0, 1, 2].map((index) => ({
    id: `focusGroup${index + 1}`,
    label: `action.focusGroup${index + 1}` as Key,
    category: 'view' as const,
    keys: `Ctrl+${index + 1}`,
    enabled: () => useEditor.getState().groups.length > index,
    run: () => useEditor.getState().focusGroupAt(index),
  })),
  {
    id: 'toggleSidebar',
    label: 'action.toggleSidebar',
    menuLabel: 'layout.region.left',
    category: 'view',
    keys: 'Ctrl+B',
    checked: () => regionShown('left'),
    run: () => useLayout.getState().toggleRegion('left'),
  },
  {
    id: 'toggleRightSidebar',
    label: 'action.toggleRightSidebar',
    menuLabel: 'layout.region.right',
    category: 'view',
    keys: 'Ctrl+Alt+B',
    checked: () => regionShown('right'),
    run: () => useLayout.getState().toggleRegion('right'),
  },
  {
    id: 'togglePanel',
    label: 'action.togglePanel',
    menuLabel: 'layout.region.panel',
    category: 'view',
    keys: 'Ctrl+J',
    checked: () => regionShown('panel'),
    run: () => useLayout.getState().toggleRegion('panel'),
  },
  {
    id: 'togglePanelPosition',
    label: 'action.togglePanelPosition',
    menuLabel: 'layout.panelRight',
    category: 'view',
    checked: () => live().panelPosition === 'right',
    run: () => useLayout.getState().setPanelPosition(live().panelPosition === 'right' ? 'bottom' : 'right'),
  },
  {
    id: 'maximizePanel',
    label: 'action.maximizePanel',
    menuLabel: 'panel.maximize',
    category: 'view',
    checked: () => useLayout.getState().panelMaximized && regionShown('panel'),
    run: () => {
      const layout = useLayout.getState();
      layout.setPanelMaximized(!(layout.panelMaximized && regionShown('panel')));
    },
  },
  { id: 'toggleMenuBar', label: 'action.toggleMenuBar', menuLabel: 'layout.bar.menubar', category: 'view', checked: () => live().bars.menubar, run: () => toggleBar('menubar') },
  { id: 'toggleToolbar', label: 'action.toggleToolbar', menuLabel: 'layout.bar.toolbar', category: 'view', checked: () => live().bars.toolbar, run: () => toggleBar('toolbar') },
  ...TOOLBAR_STYLES.map(
    (style): Action => ({
      id: TOOLBAR_STYLE_ACTIONS[style],
      label: `action.toolbar.${style}` as const,
      menuLabel: `layout.toolbar.${style}` as const,
      category: 'view',
      checked: () => live().bars.toolbarStyle === style,
      run: () => setToolbarStyle(style),
    }),
  ),
  {
    id: 'toggleActivityBar',
    label: 'action.toggleActivityBar',
    menuLabel: 'layout.bar.activitybar',
    category: 'view',
    checked: () => live().bars.activitybar !== 'hidden',
    run: toggleActivityBar,
  },
  {
    id: 'activityBarRight',
    label: 'action.activityBarRight',
    menuLabel: 'layout.bar.activitybarRight',
    category: 'view',
    checked: () => live().bars.activitybar === 'right',
    run: () => useLayout.getState().setBar('activitybar', live().bars.activitybar === 'right' ? 'left' : 'right'),
  },
  { id: 'toggleStatusBar', label: 'action.toggleStatusBar', menuLabel: 'layout.bar.statusbar', category: 'view', checked: () => live().bars.statusbar, run: () => toggleBar('statusbar') },
  // Layouts com nome (state/savedLayouts.ts)
  { id: 'selectLayout', label: 'action.selectLayout', category: 'view', keys: 'Ctrl+K L', run: () => openDialog({ kind: 'layout' }) },
  { id: 'saveLayout', label: 'action.saveLayout', category: 'view', run: saveLayout },
  { id: 'saveLayoutAs', label: 'action.saveLayoutAs', category: 'view', run: saveLayoutAs },
  { id: 'restoreLayout', label: 'action.restoreLayout', category: 'view', run: restoreLayout },
  { id: 'resetLayout', label: 'action.resetLayout', category: 'view', run: resetLayout },
  { id: 'customizeLayout', label: 'action.customizeLayout', category: 'view', run: customizeLayout },
  {
    id: 'toggleTerminal',
    label: 'action.toggleTerminal',
    category: 'view',
    keys: 'Ctrl+`',
    icon: SquareTerminal,
    run: () => {
      const layout = useLayout.getState();
      // No zen, o terminal abre numa gaveta embaixo do editor, sem as regiões.
      if (layout.zen) layout.toggleZenShell();
      else if (isOnScreen(layout, 'terminal')) layout.setRegionVisible(regionOf(layout.live, 'terminal'), false);
      else reveal('terminal');
    },
  },
  { id: 'toggleZen', label: 'action.toggleZen', category: 'view', keys: 'Ctrl+K Z', run: () => useLayout.getState().toggleZen() },
  { id: 'showProblems', label: 'action.showProblems', category: 'view', keys: 'Ctrl+Shift+M', run: () => reveal('problems') },
  // O INFO e os terminais de etapa: abrem um terminal fechado, ou trazem
  // para a frente um aberto.
  { id: 'viewConsoleInfo', label: 'action.viewConsoleInfo', category: 'view', keys: 'Ctrl+Shift+U', run: () => reveal('info') },
  ...CONSOLE_CHANNELS.map((channel) => ({
    id: VIEW_INFO[channel].action,
    label: `action.${VIEW_INFO[channel].action}` as Key,
    category: 'view' as const,
    feature: VIEW_INFO[channel].feature,
    run: () => reveal(channel),
  })),
  { id: 'zoomIn', label: 'action.zoomIn', category: 'view', keys: 'Ctrl+=', run: () => applyZoom(useLayout.getState().zoom + 0.1) },
  { id: 'zoomOut', label: 'action.zoomOut', category: 'view', keys: 'Ctrl+-', run: () => applyZoom(useLayout.getState().zoom - 0.1) },
  { id: 'zoomReset', label: 'action.zoomReset', category: 'view', keys: 'Ctrl+0', run: () => applyZoom(1) },
  {
    id: 'toggleTheme',
    label: 'action.toggleTheme',
    category: 'view',
    run: () =>
      useApp.getState().updateSettings((s) => ({ ...s, theme_mode: toggledMode(useApp.getState().theme) })),
  },
  { id: 'selectTheme', label: 'action.selectTheme', category: 'view', keys: 'Ctrl+K T', run: () => openDialog({ kind: 'theme' }) },

  // Projeto
  { id: 'addVerilog', label: 'action.addVerilog', category: 'project', enabled: hasSpf, run: () => addVerilogFiles() },
  { id: 'newVerilog', label: 'action.newVerilog', category: 'project', icon: FileCode, enabled: hasSpf, run: () => openDialog({ kind: 'newVerilog', testbench: false }) },
  { id: 'newTestbench', label: 'action.newTestbench', category: 'project', enabled: hasSpf, run: () => openDialog({ kind: 'newVerilog', testbench: true }) },
  { id: 'newCocotb', label: 'action.newCocotb', category: 'project', enabled: hasSpf, run: () => openDialog({ kind: 'newVerilog', testbench: true, cocotb: true }) },
  { id: 'newProcessor', label: 'action.newProcessor', category: 'project', keys: 'Ctrl+Alt+P', icon: Cpu, enabled: hasSpf, run: () => openDialog({ kind: 'newProcessor' }) },
  {
    id: 'removeProcessor',
    label: 'action.removeProcessor',
    category: 'project',
    // Não no meio de uma operação: ela pode estar compilando ou simulando o
    // processador.
    enabled: () => canRun() && (useProject.getState().snapshot?.processors.length ?? 0) > 0,
    run: () => openDialog({ kind: 'removeProcessor' }),
  },
  { id: 'chooseTop', label: 'action.chooseTop', category: 'project', enabled: hasSpf, run: () => openDialog({ kind: 'chooseTop' }) },
  { id: 'chooseTestbench', label: 'action.chooseTestbench', category: 'project', enabled: hasSpf, run: () => openDialog({ kind: 'chooseTestbench' }) },
  { id: 'refreshProject', label: 'action.refreshProject', category: 'project', enabled: hasProject, run: () => { useProject.getState().bumpTree(); return useProject.getState().refresh(); } },

  // Fluxo
  { id: 'build', label: 'action.build', category: 'flow', keys: 'F6', icon: Hammer, enabled: canRun, run: () => runBuild() },
  { id: 'check', label: 'action.check', category: 'flow', keys: 'F7', icon: ListChecks, enabled: canRun, run: () => runCheck(false) },
  { id: 'lint', label: 'action.lint', category: 'flow', keys: 'Shift+F7', enabled: () => canRun() && hasComponent('verilator'), run: () => runCheck(true) },
  { id: 'simulate', label: 'action.simulate', category: 'flow', keys: 'F8', icon: Play, enabled: canRun, run: () => runSimulation(settings()?.open_wave_after_sim ?? true) },
  {
    id: 'fastSim',
    label: 'action.fastSim',
    category: 'flow',
    keys: 'F9',
    icon: Zap,
    enabled: () => canRun() && hasComponent(fastSimulator()),
    run: () => runFastSimulation(),
  },
  { id: 'openWave', label: 'action.openWave', category: 'flow', keys: 'Ctrl+F8', icon: Activity, enabled: hasWave, run: () => openWave() },
  { id: 'waveSignals', label: 'action.waveSignals', category: 'flow', icon: ListFilter, enabled: hasSpf, run: () => openDialog({ kind: 'waveSignals' }) },
  { id: 'synthesize', label: 'action.synthesize', category: 'flow', keys: 'F10', icon: CircuitBoard, enabled: canRun, run: () => runSynthesis() },
  { id: 'showSchematic', label: 'action.showSchematic', category: 'flow', enabled: hasProject, run: () => useEditor.getState().openView('schematic') },
  { id: 'showStatistics', label: 'action.showStatistics', category: 'flow', enabled: hasProject, run: () => useEditor.getState().openView('synthesis') },
  // A placa FPGA, experimental.
  { id: 'fpgaBuildProgram', label: 'action.fpgaBuildProgram', category: 'flow', icon: Upload, feature: 'fpga', enabled: canRun, run: () => runFpgaBuildAndProgram() },
  { id: 'fpgaBuild', label: 'action.fpgaBuild', category: 'flow', icon: Hammer, feature: 'fpga', enabled: canRun, run: () => runFpgaBuild() },
  { id: 'fpgaProgram', label: 'action.fpgaProgram', category: 'flow', icon: Upload, feature: 'fpga', enabled: canRun, run: () => runFpgaProgram() },
  { id: 'showBoard', label: 'action.showBoard', category: 'flow', icon: Microchip, feature: 'fpga', enabled: hasSpf, run: () => useEditor.getState().openView('board') },
  { id: 'showVirtualBoard', label: 'action.showVirtualBoard', category: 'flow', icon: MonitorPlay, feature: 'fpga', enabled: hasSpf, run: () => useEditor.getState().openView('vboard') },
  { id: 'fullFlow', label: 'action.fullFlow', category: 'flow', keys: 'F5', enabled: canRun, run: fullFlow },
  {
    id: 'cancel',
    label: 'action.cancel',
    category: 'flow',
    keys: 'Shift+F5',
    icon: CircleStop,
    // A atualização da AURORA não para no meio (commands/toolchain.rs).
    enabled: () => !idle() && useJobs.getState().running?.flow !== 'update',
    run: () => useJobs.getState().cancel(),
  },
  {
    id: 'useIcarus',
    label: 'action.useIcarus',
    category: 'flow',
    enabled: () => hasComponent('icarus'),
    checked: () => settings()?.simulator === 'icarus',
    run: () => useApp.getState().updateSettings((s) => ({ ...s, simulator: 'icarus' })),
  },
  {
    id: 'useVerilator',
    label: 'action.useVerilator',
    category: 'flow',
    enabled: () => hasComponent('verilator'),
    checked: () => settings()?.simulator === 'verilator',
    run: () => useApp.getState().updateSettings((s) => ({ ...s, simulator: 'verilator' })),
  },
  { id: 'chooseTarget', label: 'action.chooseTarget', category: 'flow', enabled: () => hasSpf() && idle(), run: () => openDialog({ kind: 'target' }) },
  {
    id: 'viewHierarchy',
    label: 'action.viewHierarchy',
    category: 'flow',
    icon: Boxes,
    enabled: hasSpf,
    run: () => {
      useLayout.getState().setExplorerMode('hierarchy');
      reveal('explorer');
      if (idle()) void useHierarchy.getState().refresh();
    },
  },

  // Ferramentas
  { id: 'toolchain', label: 'action.toolchain', category: 'tools', run: () => useEditor.getState().openView('toolchain') },
  { id: 'installComponents', label: 'action.installComponents', category: 'tools', enabled: idle, run: () => openDialog({ kind: 'install' }) },
  {
    id: 'checkUpdates',
    label: 'action.checkUpdates',
    category: 'tools',
    run: () => useEditor.getState().openView('toolchain', { check: String(Date.now()) }),
  },
  { id: 'history', label: 'action.history', category: 'tools', keys: 'Ctrl+Shift+H', enabled: hasSpf, run: () => reveal('reports') },
  {
    id: 'lastReport',
    label: 'action.lastReport',
    category: 'tools',
    enabled: hasSpf,
    run: async () => {
      try {
        const report = await api.history.show(null);
        useEditor.getState().openView('report', { id: report.id });
      } catch (error) {
        // Sem relatório ainda não é erro: o projeto só não rodou nada.
        if ((error as { code?: string }).code === 'no_reports') {
          useToasts.getState().push({ kind: 'info', title: t('reports.empty') });
        } else showError(error);
      }
    },
  },
  {
    id: 'compareReports',
    label: 'action.compareReports',
    category: 'tools',
    enabled: hasSpf,
    run: () => useEditor.getState().openView('compare', { id: null, against: null }),
  },
  { id: 'cleanReports', label: 'action.cleanReports', category: 'tools', enabled: canRun, run: () => openDialog({ kind: 'cleanReports' }) },

  // Exercícios (aurora learn)
  { id: 'learnCheck', label: 'action.learnCheck', category: 'tools', keys: 'Ctrl+Alt+L', enabled: () => idle() && hasLearnExercise(), run: () => useLearn.getState().check() },
  {
    id: 'learnHint',
    label: 'action.learnHint',
    category: 'tools',
    enabled: hasLearnExercise,
    run: () => {
      const exercise = currentExercise(useLearn.getState().snapshot);
      if (!exercise) return;
      useLearn.getState().showHint(exercise.name);
      useEditor.getState().openView('learn');
    },
  },
  { id: 'learnWave', label: 'action.learnWave', category: 'tools', enabled: hasLearnExercise, run: () => useLearn.getState().openWave() },
  { id: 'learnNext', label: 'action.learnNext', category: 'tools', enabled: () => idle() && !!currentExercise(useLearn.getState().snapshot)?.solved, run: () => useLearn.getState().next() },

  // Ajuda
  { id: 'auroraDocs', label: 'action.auroraDocs', category: 'help', run: () => openUrl(AURORA_CLI_DOCS).catch(showError) },
  { id: 'saphoManual', label: 'action.saphoManual', category: 'help', run: () => openUrl(SAPHO_MANUAL).catch(showError) },
  { id: 'startTour', label: 'action.startTour', category: 'help', run: () => useTour.getState().start() },
  { id: 'shortcuts', label: 'action.shortcuts', category: 'help', run: () => openDialog({ kind: 'shortcuts' }) },
  { id: 'about', label: 'action.about', category: 'help', run: () => useEditor.getState().openView('about') },
];

const BY_ID = new Map(ACTIONS.map((action) => [action.id, action]));

export function action(id: string): Action {
  const found = BY_ID.get(id);
  if (!found) throw new Error(`Unknown action ${id}`);
  return found;
}

/** A ação existe nesta sessão: não é de recurso experimental, ou o dela
 * está ligado. Quem lista ações (menus, paleta, atalhos, barra, navegador
 * de fluxo) pula as que não existem. */
export function isAvailable(a: Action): boolean {
  return !a.feature || featureOn(a.feature);
}

/** Um item da barra de ferramentas existe: o de uma ação, se ela existe; os
 * outros (o alvo, o que está rodando), sempre. */
export function toolbarItemAvailable(id: string): boolean {
  const a = BY_ID.get(id);
  return !a || isAvailable(a);
}

export function isEnabled(a: Action): boolean {
  return isAvailable(a) && (a.enabled ? a.enabled() : true);
}

/** Roda uma ação se estiver habilitada. */
export function runAction(id: string): void {
  const a = action(id);
  if (!isEnabled(a)) return;
  try {
    const result = a.run();
    if (result instanceof Promise) result.catch(showError);
  } catch (error) {
    showError(error);
  }
}

// Teclado ---------------------------------------------------------------

const isMac = navigator.platform.toLowerCase().includes('mac');

/** O atalho de um evento, no formato da lista (`Ctrl+Shift+P`, `F8`). */
export function comboOf(event: KeyboardEvent): string {
  const parts: string[] = [];
  if (event.ctrlKey || (isMac && event.metaKey)) parts.push('Ctrl');
  if (event.shiftKey) parts.push('Shift');
  if (event.altKey) parts.push('Alt');
  let key = event.key;
  if (event.code === 'Backquote') key = '`';
  else if (event.code === 'Comma') key = ',';
  else if (event.code === 'Equal' || key === '+') key = '=';
  else if (event.code === 'Minus') key = '-';
  else if (event.code.startsWith('Key')) key = event.code.slice(3);
  else if (event.code.startsWith('Digit')) key = event.code.slice(5);
  else if (key.length === 1) key = key.toUpperCase();
  parts.push(key);
  return parts.join('+');
}

const BY_KEYS = new Map(ACTIONS.filter((a) => a.keys).map((a) => [a.keys!, a]));

/** Os atalhos de duas etapas (`Ctrl+K Z`): a primeira tecla de cada um. */
const CHORD_STARTS = new Set(ACTIONS.filter((a) => a.keys?.includes(' ')).map((a) => a.keys!.split(' ')[0]!));

/** A primeira tecla de um atalho de duas etapas, esperando a segunda. A
 * barra de status e o indicador do zen mostram. */
export const useChord = create<{ pending: string | null }>(() => ({ pending: null }));

/** Os atalhos de duas etapas, para o Monaco também os reconhecer (App.tsx). */
export function chordActions(): Action[] {
  return ACTIONS.filter((a) => a.keys?.includes(' '));
}

const MODIFIERS = new Set(['Control', 'Shift', 'Alt', 'Meta']);
let lastEscape = 0;

/**
 * Esc duas vezes, em meio segundo, sai do zen, como no VS Code. Não vale
 * no terminal (o Esc é do shell, de programas como `vim` e `less`), no
 * editor com o Vim ligado (o Esc é do Vim) nem num diálogo (o Esc o fecha).
 * O Esc segue para quem tem o foco: fechar uma sugestão do editor também
 * conta como o primeiro.
 */
function zenEscape(target: Element | null): void {
  if (!useLayout.getState().zen) return;
  if (target?.closest('.shell-terminal, .vboard-keys, .dialog') || (target?.closest('.monaco-editor') && vimActive())) {
    lastEscape = 0;
    return;
  }
  const now = performance.now();
  if (now - lastEscape < 500) {
    lastEscape = 0;
    useLayout.getState().exitZen();
  } else {
    lastEscape = now;
  }
}

/** Teclas que o editor resolve sozinho, e que o Studio deixa passar quando
 * o foco está nele. O Ctrl+F só sem o Vim: com ele, o monaco-vim o tomaria
 * como página para baixo, e a busca no arquivo vem antes (handleShortcut). */
const EDITOR_KEYS = new Set(['Ctrl+Z', 'Ctrl+Y', 'Ctrl+F', 'Ctrl+H', 'Ctrl+G', 'Shift+Alt+F']);

/** Com o Vim ligado, estas também ficam com o editor: Ctrl+B (página para
 * cima) e Ctrl+O (voltar no histórico de saltos) no lugar de mostrar a barra
 * lateral e de abrir arquivo. */
const VIM_KEYS = new Set(['Ctrl+B', 'Ctrl+O']);

/** Teclas que um campo de texto resolve sozinho: no campo da Busca, o Ctrl+Z
 * desfazia a última digitação do editor e levava o foco para lá. */
const FIELD_KEYS = new Set(['Ctrl+Z', 'Ctrl+Y']);

const NOT_TEXT_INPUTS = new Set(['button', 'checkbox', 'color', 'file', 'image', 'radio', 'range', 'reset', 'submit']);

/** O foco está num campo de texto da interface (fora do editor e do
 * terminal, que têm as regras deles). */
function inTextField(target: Element | null): boolean {
  if (!target || target.closest('.monaco-editor, .shell-terminal, .vboard-keys')) return false;
  if (target instanceof HTMLTextAreaElement) return true;
  if (target instanceof HTMLInputElement) return !NOT_TEXT_INPUTS.has(target.type);
  return target instanceof HTMLElement && target.isContentEditable;
}

/**
 * O tratador global, na fase de captura: roda antes do Monaco e do xterm.
 * No terminal de shell, só as teclas de função e Ctrl+Shift saem dele, para
 * Ctrl+C, Ctrl+W e companhia continuarem do shell.
 *
 * Atalhos de duas etapas: a primeira tecla fica esperando (`useChord`), e a
 * seguinte completa o atalho ou só cancela a espera. No editor, quem
 * reconhece é o próprio Monaco (bindEditorChord, em editor/monaco.ts),
 * porque ele tem os seus (Ctrl+K Ctrl+C comenta a linha) e o Ctrl+K não
 * pode parar aqui. Com o Vim ligado é o contrário: o monaco-vim fica com as
 * teclas antes do Monaco (o `z` é prefixo de comando dele), os atalhos
 * Ctrl+K do Monaco já não valem, e quem reconhece é este tratador. No
 * terminal o Ctrl+K é do shell (apaga até o fim da linha), então lá não há
 * atalho de duas etapas.
 */
export function handleShortcut(event: KeyboardEvent): void {
  if (MODIFIERS.has(event.key)) return;
  const combo = comboOf(event);
  const target = event.target instanceof Element ? event.target : null;
  const inEditor = !!target?.closest('.monaco-editor');
  // O teclado da placa virtual fica com as teclas, como o terminal: só as
  // de função e as com Ctrl+Shift saem dele.
  const inShell = !!target?.closest('.shell-terminal, .vboard-keys');
  const inDialog = !!target?.closest('.dialog');

  const pending = useChord.getState().pending;
  if (pending) {
    useChord.setState({ pending: null });
    event.preventDefault();
    event.stopPropagation();
    const chord = BY_KEYS.get(`${pending} ${combo}`);
    if (chord) runAction(chord.id);
    return;
  }
  if (combo === 'Escape') {
    zenEscape(target);
    return;
  }
  if (CHORD_STARTS.has(combo) && (!inEditor || vimActive()) && !inShell && !inDialog) {
    event.preventDefault();
    event.stopPropagation();
    useChord.setState({ pending: combo });
    return;
  }

  const a = BY_KEYS.get(combo);
  if (!a) return;
  const vimFind = combo === 'Ctrl+F' && vimActive();
  if (inEditor && !vimFind && (EDITOR_KEYS.has(combo) || (VIM_KEYS.has(combo) && vimActive()))) return;
  if (FIELD_KEYS.has(combo) && inTextField(target)) return;
  // No RTLViewer, o Ctrl+F é a busca dele (SchematicView).
  if (combo === 'Ctrl+F' && target?.closest('.rtlViewer')) return;
  if (inShell && !/^(Shift\+)?F\d+$/.test(combo) && !combo.startsWith('Ctrl+Shift+') && combo !== 'Ctrl+`') return;
  if (inDialog && a.id !== 'commandPalette' && a.id !== 'quickOpen') return;
  event.preventDefault();
  event.stopPropagation();
  runAction(a.id);
}

/** O nome de um arquivo do projeto a partir do caminho relativo digitado. */
export function projectPath(relative: string): string | null {
  const root = useProject.getState().snapshot?.root;
  return root ? joinPath(root, relative) : null;
}
