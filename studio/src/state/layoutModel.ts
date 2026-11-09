// O modelo do layout da janela: as vistas, as regiões onde elas ficam, as
// barras e o que está escondido. Sem React e sem store: a store da janela
// (layout.ts), os layouts salvos (savedLayouts.ts) e as Preferências usam as
// mesmas funções.
//
// Três regiões fixas, como no VS Code: a barra lateral esquerda, a direita e
// o painel (embaixo ou à direita do editor). Cada uma das dezessete vistas fica
// em exatamente uma delas e pode ir para qualquer outra. O editor e as abas de
// vista (Preferências, Wave, Esquemático) ficam no centro, fora disso.
//
// O layout guarda o que está escondido, não o que aparece: uma vista ou um
// item que uma versão futura acrescentar aparece sozinho, e uma vista que não
// está em região nenhuma vai para a região padrão dela.
//
// Uma vista de recurso experimental desligado (o console Placa, sem a placa
// FPGA) não existe na sessão: não aparece em região, seletor nem menu, mas
// continua no layout, no mesmo lugar, para voltar quando o recurso ligar.

import type { Key } from '../i18n';
import type { Feature } from '../ipc/aurora-types';

/** Os terminais de etapa, com os nomes da AURORA (console/consoles.ts).
 * Ficam fechados até o usuário abrir um; o INFO recebe tudo o que eles
 * recebem. */
export const CONSOLE_CHANNELS = ['cmm', 'asm', 'verilog', 'wave', 'rtlViewer', 'fpga'] as const;
export type ConsoleChannel = (typeof CONSOLE_CHANNELS)[number];

/** Um console: o INFO, com a saída de todas as etapas, ou o de uma etapa. */
export type ConsoleId = 'info' | ConsoleChannel;

export function isConsoleChannel(value: unknown): value is ConsoleChannel {
  return typeof value === 'string' && (CONSOLE_CHANNELS as readonly string[]).includes(value);
}

export const VIEW_IDS = [
  'explorer',
  'flow',
  'search',
  'git',
  'reports',
  'learn',
  'profile',
  'assistant',
  'info',
  ...CONSOLE_CHANNELS,
  'problems',
  'terminal',
] as const;
export type ViewId = (typeof VIEW_IDS)[number];

export const REGION_IDS = ['left', 'right', 'panel'] as const;
export type RegionId = (typeof REGION_IDS)[number];

export type PanelPosition = 'bottom' | 'right';
export type ActivityBarSide = 'left' | 'right' | 'hidden';

export interface ViewInfo {
  label: Key;
  /** Onde a vista fica quando o layout não diz. */
  region: RegionId;
  /** A ação que revela a vista (o atalho dela aparece nas dicas). */
  action: string;
  /** O recurso experimental de que a vista faz parte (state/experimental.ts). */
  feature?: Feature;
  /** Na barra de atividades, fica embaixo, perto das Preferências, como as
   * Contas do VS Code. */
  bottom?: boolean;
}

export const VIEW_INFO: Record<ViewId, ViewInfo> = {
  explorer: { label: 'sidebar.explorer', region: 'left', action: 'viewExplorer' },
  flow: { label: 'sidebar.flow', region: 'left', action: 'viewFlow' },
  search: { label: 'sidebar.search', region: 'left', action: 'findInFiles' },
  git: { label: 'sidebar.git', region: 'left', action: 'viewGit', feature: 'git' },
  reports: { label: 'sidebar.reports', region: 'left', action: 'viewReports' },
  learn: { label: 'sidebar.learn', region: 'left', action: 'viewLearn' },
  profile: { label: 'sidebar.profile', region: 'left', action: 'viewProfile', feature: 'git', bottom: true },
  assistant: { label: 'sidebar.assistant', region: 'right', action: 'viewAssistant', feature: 'assistant' },
  info: { label: 'panel.info', region: 'panel', action: 'viewConsoleInfo' },
  cmm: { label: 'panel.cmm', region: 'panel', action: 'viewConsoleCmm' },
  asm: { label: 'panel.asm', region: 'panel', action: 'viewConsoleAsm' },
  verilog: { label: 'panel.verilog', region: 'panel', action: 'viewConsoleVerilog' },
  wave: { label: 'panel.wave', region: 'panel', action: 'viewConsoleWave' },
  rtlViewer: { label: 'panel.rtlViewer', region: 'panel', action: 'viewConsoleRTLViewer' },
  fpga: { label: 'panel.fpga', region: 'panel', action: 'viewConsoleFpga', feature: 'fpga' },
  problems: { label: 'panel.problems', region: 'panel', action: 'showProblems' },
  terminal: { label: 'panel.terminal', region: 'panel', action: 'toggleTerminal' },
};

/** Os recursos experimentais ligados. Começa sem nenhum, como o Core até
 * responder; quem atualiza é a store da janela (layout.ts). */
let enabledFeatures: ReadonlySet<Feature> = new Set();

export function setEnabledFeatures(features: ReadonlySet<Feature>): void {
  enabledFeatures = features;
}

/** A vista existe nesta sessão: não é de recurso experimental, ou o dela
 * está ligado. */
export function isViewAvailable(view: ViewId): boolean {
  const feature = VIEW_INFO[view].feature;
  return !feature || enabledFeatures.has(feature);
}

export const REGION_LABELS: Record<RegionId, Key> = {
  left: 'layout.region.left',
  right: 'layout.region.right',
  panel: 'layout.region.panel',
};

/** Onde ficam os botões do fluxo, um modelo por layout: `classic`, a barra
 * de ferramentas numa linha própria, com os botões lado a lado (Toolbar.tsx);
 * `titlebar`, o Alvo, o Executar e o Parar no fim da barra de título; e
 * `editor`, os mesmos no canto das abas do editor, que num fonte C± ou C++
 * sugerem compilar o processador dele (RunControls.tsx, nos dois últimos). Os três são "a barra de
 * ferramentas" do layout: `bars.toolbar` mostra ou esconde a do modelo. */
export const TOOLBAR_STYLES = ['classic', 'titlebar', 'editor'] as const;
export type ToolbarStyle = (typeof TOOLBAR_STYLES)[number];

export function isToolbarStyle(value: unknown): value is ToolbarStyle {
  return typeof value === 'string' && (TOOLBAR_STYLES as readonly string[]).includes(value);
}

/** Os grupos de botões do fluxo na barra clássica, entre separadores. */
export type FlowSection = 'compile' | 'simulate' | 'wave' | 'hardware';

export interface ToolbarItem {
  id: string;
  label: Key;
  /** Na barra clássica: os de arquivo, o alvo, os do fluxo e o que está
   * rodando. */
  group?: 'file' | 'target' | 'flow' | 'running';
  section?: FlowSection;
}

/** Os itens que podem ser escondidos em cada modelo, na ordem dele. O que
 * está escondido fica numa lista só do layout (`hidden.toolbar`): o Alvo,
 * escondido num modelo, também fica escondido nos outros. */
export const TOOLBAR_ITEMS: Record<ToolbarStyle, ToolbarItem[]> = {
  classic: [
    { id: 'newProject', label: 'action.newProject', group: 'file' },
    { id: 'openProject', label: 'action.openProject', group: 'file' },
    { id: 'save', label: 'action.save', group: 'file' },
    { id: 'target', label: 'toolbar.target', group: 'target' },
    // Os botões do fluxo com o nome que aparece neles (C±, Verilog, Wave...).
    { id: 'build', label: 'toolbar.build', group: 'flow', section: 'compile' },
    { id: 'check', label: 'toolbar.check', group: 'flow', section: 'compile' },
    { id: 'simulate', label: 'toolbar.simulate', group: 'flow', section: 'simulate' },
    { id: 'fastSim', label: 'toolbar.fastSim', group: 'flow', section: 'simulate' },
    { id: 'openWave', label: 'toolbar.openWave', group: 'flow', section: 'wave' },
    { id: 'waveSignals', label: 'toolbar.waveSignals', group: 'flow', section: 'wave' },
    { id: 'synthesize', label: 'toolbar.synthesize', group: 'flow', section: 'hardware' },
    { id: 'showBoard', label: 'toolbar.board', group: 'flow', section: 'hardware' },
    { id: 'showVirtualBoard', label: 'toolbar.vboard', group: 'flow', section: 'hardware' },
    { id: 'running', label: 'layout.item.running', group: 'running' },
  ],
  titlebar: [
    { id: 'target', label: 'toolbar.target' },
    { id: 'run', label: 'run.label' },
    { id: 'stop', label: 'toolbar.stop' },
  ],
  editor: [
    { id: 'target', label: 'toolbar.target' },
    { id: 'run', label: 'run.label' },
    { id: 'stop', label: 'toolbar.stop' },
  ],
};

/** Os itens da barra de status que podem ser escondidos. A espera de um
 * atalho de duas etapas e a linha do Vim não estão aqui: aparecem sempre. Um
 * item de recurso experimental desligado (`feature`) não aparece na barra
 * nem nas listas. */
export const STATUS_ITEMS: { id: string; label: Key; feature?: Feature }[] = [
  { id: 'project', label: 'layout.item.project' },
  { id: 'git', label: 'layout.item.git', feature: 'git' },
  { id: 'top', label: 'layout.item.top' },
  { id: 'testbench', label: 'layout.item.testbench' },
  { id: 'target', label: 'toolbar.target' },
  { id: 'operation', label: 'layout.item.operation' },
  { id: 'problems', label: 'panel.problems' },
  { id: 'cursor', label: 'layout.item.cursor' },
  { id: 'language', label: 'layout.item.language' },
  { id: 'simulator', label: 'layout.item.simulator' },
  { id: 'bundle', label: 'layout.item.bundle' },
  { id: 'regions', label: 'layout.item.regions' },
];

/** Os itens da barra de status que existem nesta sessão. */
export function availableStatusItems(): { id: string; label: Key }[] {
  return STATUS_ITEMS.filter((item) => !item.feature || enabledFeatures.has(item.feature));
}

export interface RegionLayout {
  /** As vistas da região, na ordem das abas ou dos ícones. Uma vista
   * escondida continua aqui: ao reaparecer, volta ao mesmo lugar. */
  views: ViewId[];
  active: ViewId | null;
  visible: boolean;
}

export interface LayoutSizes {
  /** Larguras e alturas em pixels CSS (com o zoom, como o resto). */
  left: number;
  right: number;
  panelBottom: number;
  panelRight: number;
}

export interface LayoutBars {
  menubar: boolean;
  toolbar: boolean;
  toolbarStyle: ToolbarStyle;
  statusbar: boolean;
  activitybar: ActivityBarSide;
}

export interface LayoutBody {
  regions: Record<RegionId, RegionLayout>;
  panelPosition: PanelPosition;
  sizes: LayoutSizes;
  bars: LayoutBars;
  hidden: { views: ViewId[]; toolbar: string[]; statusbar: string[] };
}

/** A versão do formato. Um layout gravado por uma versão mais nova do Studio
 * é lido como der e fica só para leitura, como os prontos. */
export const LAYOUT_VERSION = 1;

export interface LayoutDef extends LayoutBody {
  v: number;
  id: string;
  name: string;
}

export const DEFAULT_SIZES: LayoutSizes = { left: 290, right: 290, panelBottom: 260, panelRight: 420 };

/** Os limites só barram valores absurdos; os mínimos de verdade são os dos
 * painéis (Workbench.tsx). */
const SIZE_MIN = 60;
const SIZE_MAX = 4000;

/** O Padrão: no painel, o INFO e o shell lado a lado, depois Problemas. Os
 * terminais de etapa ficam entre o shell e Problemas, fechados. Os layouts
 * prontos listam todas as vistas: uma que faltasse entraria no fim da região
 * dela ao aplicar (`normalizeBody`), e o layout pareceria modificado. */
export const DEFAULT_BODY: LayoutBody = {
  regions: {
    left: { views: ['explorer', 'flow', 'search', 'git', 'reports', 'learn', 'profile'], active: 'explorer', visible: true },
    right: { views: ['assistant'], active: null, visible: false },
    panel: { views: ['info', 'terminal', ...CONSOLE_CHANNELS, 'problems'], active: 'info', visible: true },
  },
  panelPosition: 'bottom',
  sizes: DEFAULT_SIZES,
  bars: { menubar: true, toolbar: true, toolbarStyle: 'classic', statusbar: true, activitybar: 'left' },
  hidden: { views: [...CONSOLE_CHANNELS], toolbar: [], statusbar: [] },
};

/** O Padrão com o Executar na barra de título: a janela ganha a linha da
 * barra de ferramentas. */
const TITLEBAR_BODY: LayoutBody = { ...DEFAULT_BODY, bars: { ...DEFAULT_BODY.bars, toolbarStyle: 'titlebar' } };

/** O Padrão com o Executar no canto das abas do editor e o Navegador de fluxo
 * na barra lateral direita, com todas as etapas à vista. */
const EDITOR_BODY: LayoutBody = {
  ...DEFAULT_BODY,
  regions: {
    ...DEFAULT_BODY.regions,
    left: { views: ['explorer', 'search', 'git', 'reports', 'learn', 'profile'], active: 'explorer', visible: true },
    right: { views: ['flow', 'assistant'], active: 'flow', visible: true },
  },
  bars: { ...DEFAULT_BODY.bars, toolbarStyle: 'editor' },
};

export interface Preset {
  id: string;
  /** O nome é traduzido: o layout pronto troca de nome com o idioma. */
  nameKey: Key;
  body: LayoutBody;
}

export const DEFAULT_LAYOUT_ID = 'default';

export const PRESETS: Preset[] = [
  { id: DEFAULT_LAYOUT_ID, nameKey: 'layout.preset.default', body: DEFAULT_BODY },
  { id: 'run-titlebar', nameKey: 'layout.preset.titlebar', body: TITLEBAR_BODY },
  { id: 'run-editor', nameKey: 'layout.preset.editor', body: EDITOR_BODY },
];

// Validação ----------------------------------------------------------------

type Loose = Record<string, unknown>;

function isObject(value: unknown): value is Loose {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function isViewId(value: unknown): value is ViewId {
  return typeof value === 'string' && (VIEW_IDS as readonly string[]).includes(value);
}

export function isRegionId(value: unknown): value is RegionId {
  return typeof value === 'string' && (REGION_IDS as readonly string[]).includes(value);
}

function uniqueStrings(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return [...new Set(value.filter((item): item is string => typeof item === 'string'))];
}

export function clampSize(value: unknown, fallback: number): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) return fallback;
  return Math.round(Math.min(SIZE_MAX, Math.max(SIZE_MIN, value)));
}

/** A primeira vista da região que aparece. */
function firstShown(views: ViewId[], hidden: ViewId[]): ViewId | null {
  return views.find((view) => !hidden.includes(view) && isViewAvailable(view)) ?? null;
}

/** Acerta a vista ativa de cada região: uma que está nela e aparece, ou
 * nenhuma. */
export function settleActive(body: LayoutBody): LayoutBody {
  let regions = body.regions;
  for (const id of REGION_IDS) {
    const region = regions[id];
    const ok =
      region.active !== null &&
      region.views.includes(region.active) &&
      !body.hidden.views.includes(region.active) &&
      isViewAvailable(region.active);
    if (ok) continue;
    regions = { ...regions, [id]: { ...region, active: firstShown(region.views, body.hidden.views) } };
  }
  return regions === body.regions ? body : { ...body, regions };
}

/**
 * Um layout válido a partir do que veio do disco (ou de qualquer lugar).
 * O que falta ou não serve fica com o valor de `fallback`; uma vista que
 * aparece em duas regiões fica na primeira (esquerda, direita, painel); uma
 * que não está em nenhuma vai para a região padrão dela. Os itens
 * escondidos das barras guardam também ids desconhecidos, para um layout
 * de uma versão mais nova voltar inteiro.
 */
export function normalizeBody(raw: unknown, fallback: LayoutBody = DEFAULT_BODY): LayoutBody {
  const source = isObject(raw) ? raw : {};
  const regionsRaw = isObject(source.regions) ? source.regions : null;
  const seen = new Set<ViewId>();
  const regions = {} as Record<RegionId, RegionLayout>;
  for (const id of REGION_IDS) {
    const region = regionsRaw && isObject(regionsRaw[id]) ? regionsRaw[id] : null;
    const views: ViewId[] = [];
    const listed = region ? region.views : regionsRaw ? [] : fallback.regions[id].views;
    if (Array.isArray(listed)) {
      for (const view of listed) {
        if (isViewId(view) && !seen.has(view)) {
          seen.add(view);
          views.push(view);
        }
      }
    }
    regions[id] = {
      views,
      active: region && isViewId(region.active) ? region.active : fallback.regions[id].active,
      visible: region && typeof region.visible === 'boolean' ? region.visible : fallback.regions[id].visible,
    };
  }
  // Um layout gravado antes do INFO não o tem em região nenhuma.
  const beforeInfo = regionsRaw !== null && !seen.has('info');
  for (const view of VIEW_IDS) {
    if (!seen.has(view)) regions[VIEW_INFO[view].region].views.push(view);
  }

  const hiddenRaw = isObject(source.hidden) ? source.hidden : {};
  const sizesRaw = isObject(source.sizes) ? source.sizes : {};
  const barsRaw = isObject(source.bars) ? source.bars : {};
  const bool = (value: unknown, otherwise: boolean) => (typeof value === 'boolean' ? value : otherwise);

  const body: LayoutBody = {
    regions,
    panelPosition: source.panelPosition === 'bottom' || source.panelPosition === 'right' ? source.panelPosition : fallback.panelPosition,
    sizes: {
      left: clampSize(sizesRaw.left, fallback.sizes.left),
      right: clampSize(sizesRaw.right, fallback.sizes.right),
      panelBottom: clampSize(sizesRaw.panelBottom, fallback.sizes.panelBottom),
      panelRight: clampSize(sizesRaw.panelRight, fallback.sizes.panelRight),
    },
    bars: {
      menubar: bool(barsRaw.menubar, fallback.bars.menubar),
      toolbar: bool(barsRaw.toolbar, fallback.bars.toolbar),
      toolbarStyle: isToolbarStyle(barsRaw.toolbarStyle) ? barsRaw.toolbarStyle : fallback.bars.toolbarStyle,
      statusbar: bool(barsRaw.statusbar, fallback.bars.statusbar),
      activitybar:
        barsRaw.activitybar === 'left' || barsRaw.activitybar === 'right' || barsRaw.activitybar === 'hidden'
          ? barsRaw.activitybar
          : fallback.bars.activitybar,
    },
    hidden: {
      // Sem regiões nem o que esconder, vale o arranjo inteiro de `fallback`.
      views: (isObject(source.hidden) || regionsRaw ? uniqueStrings(hiddenRaw.views) : fallback.hidden.views).filter(isViewId),
      toolbar: uniqueStrings(hiddenRaw.toolbar),
      statusbar: uniqueStrings(hiddenRaw.statusbar),
    },
  };
  return settleActive(beforeInfo ? withInfoConsole(body) : body);
}

/**
 * Um layout de antes do INFO tinha um console aberto por etapa. O INFO entra
 * no lugar do primeiro deles, de preferência no painel, e o shell, se está na
 * mesma região, vem logo depois dele. Os de etapa fecham: a saída deles está
 * no INFO, e cada um volta pelo seletor de terminal ao lugar onde estava.
 * Uma região que mostrava um console de etapa passa a mostrar o INFO.
 */
function withInfoConsole(body: LayoutBody): LayoutBody {
  const regions = { ...body.regions };
  const places = REGION_IDS.map((region) => ({ region, index: regions[region].views.findIndex(isConsoleChannel) })).filter(
    (place) => place.index >= 0,
  );
  const target = places.find((place) => place.region === 'panel') ?? places[0];
  if (target) {
    // O INFO acabou de entrar no fim da região padrão dele: tirá-lo de lá não
    // muda a posição do console que ele vai ocupar.
    for (const region of REGION_IDS) {
      regions[region] = { ...regions[region], views: regions[region].views.filter((view) => view !== 'info') };
    }
    let views = [...regions[target.region].views];
    views.splice(target.index, 0, 'info');
    if (views.includes('terminal')) {
      views = views.filter((view) => view !== 'terminal');
      views.splice(views.indexOf('info') + 1, 0, 'terminal');
    }
    regions[target.region] = { ...regions[target.region], views };
  }
  for (const region of REGION_IDS) {
    if (!isConsoleChannel(regions[region].active)) continue;
    regions[region] = { ...regions[region], active: regions[region].views.includes('info') ? 'info' : null };
  }
  const hidden = [...new Set<ViewId>([...body.hidden.views, ...CONSOLE_CHANNELS])];
  return { ...body, regions, hidden: { ...body.hidden, views: hidden } };
}

/** Um layout salvo, validado; `null` sem id ou sem nome. `newer`: gravado
 * por uma versão mais nova do formato. */
export function parseLayoutDef(raw: unknown): (LayoutDef & { newer: boolean }) | null {
  if (!isObject(raw)) return null;
  const { id, name } = raw;
  if (typeof id !== 'string' || !id.trim() || typeof name !== 'string' || !name.trim()) return null;
  const v = typeof raw.v === 'number' && Number.isFinite(raw.v) ? raw.v : 1;
  return { ...normalizeBody(raw), v, id, name: name.trim(), newer: v > LAYOUT_VERSION };
}

/** Só o corpo de um layout salvo, para comparar ou aplicar. */
export function bodyOf(def: LayoutBody): LayoutBody {
  const { regions, panelPosition, sizes, bars, hidden } = def;
  return { regions, panelPosition, sizes, bars, hidden };
}

// Consultas e mudanças -------------------------------------------------------

export function regionOf(body: LayoutBody, view: ViewId): RegionId {
  return REGION_IDS.find((id) => body.regions[id].views.includes(view)) ?? VIEW_INFO[view].region;
}

/** As vistas da região que aparecem, na ordem. */
export function visibleViews(body: LayoutBody, region: RegionId): ViewId[] {
  return body.regions[region].views.filter((view) => !body.hidden.views.includes(view) && isViewAvailable(view));
}

export function withRegion(body: LayoutBody, region: RegionId, change: Partial<RegionLayout>): LayoutBody {
  return { ...body, regions: { ...body.regions, [region]: { ...body.regions[region], ...change } } };
}

/** Leva a vista para `to`, na posição `index` (no fim, sem ela), e a deixa
 * ativa lá. Na região de onde saiu, a ativa passa à primeira que sobrou. */
export function moveView(body: LayoutBody, view: ViewId, to: RegionId, index?: number): LayoutBody {
  const from = regionOf(body, view);
  const regions = { ...body.regions };
  regions[from] = { ...regions[from], views: regions[from].views.filter((v) => v !== view) };
  const target = regions[to].views.filter((v) => v !== view);
  const at = index === undefined ? target.length : Math.max(0, Math.min(target.length, index));
  target.splice(at, 0, view);
  regions[to] = { ...regions[to], views: target, active: body.hidden.views.includes(view) ? regions[to].active : view };
  return settleActive({ ...body, regions });
}

/** Esconde ou mostra uma vista. A ativa escondida passa à próxima. */
export function setViewHidden(body: LayoutBody, view: ViewId, hidden: boolean): LayoutBody {
  const views = body.hidden.views.filter((v) => v !== view);
  if (hidden) views.push(view);
  return settleActive({ ...body, hidden: { ...body.hidden, views } });
}

export function setItemHidden(list: string[], id: string, hidden: boolean): string[] {
  const rest = list.filter((item) => item !== id);
  return hidden ? [...rest, id] : rest;
}

/** A barra de ferramentas (a do modelo do layout) e a de status: o item
 * aparece? */
export function itemShown(body: LayoutBody, bar: 'toolbar' | 'statusbar', id: string): boolean {
  return !body.hidden[bar].includes(id);
}

const sameList = <T>(a: readonly T[], b: readonly T[]) => a.length === b.length && a.every((item, i) => item === b[i]);
const sameSet = (a: readonly string[], b: readonly string[]) => a.length === b.length && a.every((item) => b.includes(item));

/**
 * O mesmo arranjo? Entram as vistas e a ordem delas em cada região, o que
 * está escondido, as barras, a posição do painel, a visibilidade das regiões
 * e os tamanhos, com folga de `tolerancePx`. A vista ativa de cada região
 * não entra: trocar de aba é navegar, não mudar o layout.
 */
export function sameBody(a: LayoutBody, b: LayoutBody, tolerancePx = 16): boolean {
  for (const id of REGION_IDS) {
    if (!sameList(a.regions[id].views, b.regions[id].views)) return false;
    if (a.regions[id].visible !== b.regions[id].visible) return false;
  }
  if (a.panelPosition !== b.panelPosition) return false;
  const bars = Object.keys(a.bars) as (keyof LayoutBars)[];
  if (bars.some((bar) => a.bars[bar] !== b.bars[bar])) return false;
  if (!sameSet(a.hidden.views, b.hidden.views)) return false;
  if (!sameSet(a.hidden.toolbar, b.hidden.toolbar)) return false;
  if (!sameSet(a.hidden.statusbar, b.hidden.statusbar)) return false;
  const sizes = Object.keys(a.sizes) as (keyof LayoutSizes)[];
  return sizes.every((size) => Math.abs(a.sizes[size] - b.sizes[size]) <= tolerancePx);
}
