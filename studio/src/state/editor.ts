// As abas da área central: arquivos (no Monaco) e vistas (boas-vindas,
// preferências, ferramentas, esquemático, síntese, relatórios,
// processador).
//
// A área se divide em até três grupos lado a lado, cada um com as suas
// abas e o seu editor. `tabs` tem cada aba aberta uma vez, em
// qualquer grupo; os grupos guardam só os ids, na ordem da barra. Um
// arquivo pode estar em dois grupos ao mesmo tempo (o mesmo modelo em dois
// editores); uma vista, só em um. `activeId` é a aba ativa do grupo ativo,
// a que os menus, a barra de status e os atalhos usam.
//
// Cada arquivo aberto tem um modelo no Monaco (editor/monaco.ts) e um
// DocInfo aqui, com o horário de modificação lido do disco. Gravar manda
// esse horário junto: se o arquivo mudou por fora, o backend recusa
// (`conflict`) e o usuário escolhe.
//
// A vista de um recurso experimental desligado (a Placa FPGA, sem a placa
// ligada) não abre, nem pela sessão gravada, e fecha quando ele desliga.
//
// A aba de diff (o controle de versão) compara duas versões de um arquivo.
// Com as mudanças do disco, o lado novo é o próprio arquivo: a aba leva o
// caminho dele em `path`, como uma aba de arquivo, e usa o mesmo modelo e o
// mesmo DocInfo. Editar no diff edita o arquivo, Ctrl+S grava, e o modelo
// só sai quando nenhuma das duas abas o usa mais.

import { create } from 'zustand';

import { disposeModel, ensureModel, getModel, uriFor } from '../editor/monaco';
import { t } from '../i18n';
import { api } from '../ipc/api';
import type { Feature } from '../ipc/aurora-types';
import type { IpcError } from '../ipc/types';
import { baseName, isInside, systemPath } from '../util/paths';
import { confirm } from './dialogs';
import { featureOn, useExperimental } from './experimental';
import { showError } from './toasts';

export type ViewKind =
  | 'settings'
  | 'toolchain'
  | 'schematic'
  | 'synthesis'
  | 'board'
  | 'report'
  | 'compare'
  | 'processor'
  | 'about'
  | 'wave'
  | 'learn'
  | 'assistant'
  | 'vboard'
  | 'diff';

export interface Tab {
  id: string;
  kind: 'file' | ViewKind;
  /** Arquivo (abas de arquivo). */
  path?: string;
  /** Parâmetros da vista: id do relatório, nome do processador. */
  data?: Record<string, string | null>;
  dirty?: boolean;
  /** Aba provisória: a próxima abertura provisória a substitui. */
  preview?: boolean;
}

/** O que a aba de diff compara (DiffView.tsx). */
export interface DiffSpec {
  /** O arquivo, relativo ao topo do repositório, com `/`. */
  repo: string;
  /** O commit do lado novo; vazio ou `null`, as mudanças do disco contra o
   * último commit. */
  rev: string | null;
  /** O identificador curto do commit, para o título. */
  short?: string | null;
  /** O pai do commit, o lado antigo; `null` no primeiro commit. */
  parent?: string | null;
  /** O caminho antigo, num arquivo renomeado. */
  original?: string | null;
  /** A mudança (`modified`, `added`, `deleted`...). */
  change: string;
  /** O arquivo no disco, para as mudanças do disco de um arquivo que existe:
   * o lado novo é ele, editável. */
  file?: string | null;
}

export interface DocInfo {
  path: string;
  modifiedMs: number;
  savedVersion: number;
  binary: boolean;
  tooLarge: boolean;
  deleted?: boolean;
}

export interface Reveal {
  path: string;
  line: number;
  column: number;
  /** O grupo cujo editor vai à linha. */
  group: string;
  nonce: number;
}

export interface EditorGroup {
  id: string;
  /** As abas do grupo, na ordem da barra. */
  tabIds: string[];
  activeId: string | null;
}

/** Quantos grupos cabem lado a lado, como os painéis da AURORA. */
export const MAX_GROUPS = 3;

/** Para que lado mandar uma aba: o grupo vizinho, ou um novo se couber. */
export type Side = 'left' | 'right';

interface EditorState {
  /** Cada aba aberta, uma vez, esteja em que grupo estiver. */
  tabs: Tab[];
  /** Os grupos, da esquerda para a direita. Sempre há pelo menos um. */
  groups: EditorGroup[];
  activeGroup: string;
  /** A aba ativa do grupo ativo. */
  activeId: string | null;
  docs: Record<string, DocInfo>;
  closed: Tab[];
  reveal: Reveal | null;
  cursor: { line: number; column: number } | null;
  languageId: string | null;

  /** Abre no grupo ativo, ou no vizinho da direita com `side`. */
  openFile: (path: string, options?: { line?: number; column?: number; preview?: boolean; side?: boolean }) => Promise<void>;
  openView: (kind: ViewKind, data?: Record<string, string | null>) => void;
  /** Abre a comparação de um arquivo (o controle de versão). Com `preview`,
   * a aba é provisória, como a de um arquivo aberto com um clique. */
  openDiff: (spec: DiffSpec, options?: { preview?: boolean }) => Promise<void>;
  /** Ativa a aba no grupo dado, no ativo se ela estiver lá, ou no primeiro que a tiver. */
  activate: (id: string, group?: string) => void;
  focusGroup: (group: string) => void;
  /** Ativa o grupo de posição `index` (0, 1, 2), se existir. */
  focusGroupAt: (index: number) => void;
  pin: (id: string) => void;
  /** Fecha a aba no grupo dado; sem grupo, em todos. Só pergunta pelo que
   * não foi salvo quando a aba sai do último grupo que a tinha. */
  closeTab: (id: string, group?: string) => Promise<boolean>;
  closeOthers: (id: string, group?: string) => Promise<void>;
  closeAll: () => Promise<boolean>;
  closeGroup: (group: string) => Promise<boolean>;
  /** Abre a aba (a ativa, sem `id`) também no grupo do lado. Uma vista muda
   * de grupo, porque só fica em um. */
  split: (id?: string, side?: Side) => void;
  /** Tira a aba do grupo `from` e a põe no vizinho. */
  moveTo: (id: string, from: string, side: Side) => void;
  /** Põe a aba no grupo `to`, na posição `index` (a barra de abas, ao soltar
   * uma aba arrastada). Do mesmo grupo, só reordena. */
  dropTab: (id: string, from: string, to: string, index: number) => void;
  save: (id?: string) => Promise<boolean>;
  saveAll: () => Promise<boolean>;
  setDirty: (path: string, dirty: boolean) => void;
  onDiskChange: (paths: string[]) => Promise<void>;
  renamed: (from: string, to: string) => void;
  reopenClosed: () => void;
  setCursor: (line: number, column: number, languageId: string | null) => void;
  reset: () => void;
  persistSession: (spf: string) => void;
  restoreSession: (spf: string) => Promise<void>;
}

export const fileTabId = (path: string) => `file:${path}`;

/** As vistas de recurso experimental (state/experimental.ts). */
const VIEW_FEATURE: Partial<Record<ViewKind, Feature>> = { board: 'fpga', vboard: 'fpga', assistant: 'assistant', diff: 'git' };

/** A vista existe nesta sessão: não é de recurso experimental, ou o dela
 * está ligado. */
export function viewAvailable(kind: ViewKind): boolean {
  const feature = VIEW_FEATURE[kind];
  return !feature || featureOn(feature);
}

export function viewTabId(kind: ViewKind, data?: Record<string, string | null>): string {
  switch (kind) {
    case 'report':
      return `view:report:${data?.id ?? ''}`;
    case 'compare':
      return `view:compare:${data?.id ?? ''}:${data?.against ?? ''}`;
    case 'processor':
      return `view:processor:${data?.name ?? ''}`;
    case 'wave':
      return `view:wave:${data?.path ?? ''}`;
    case 'diff':
      return `view:diff:${data?.rev ?? ''}:${data?.repo ?? ''}`;
    default:
      return `view:${kind}`;
  }
}

/** Modelos que já têm o ouvinte de alteração. */
const watched = new Set<string>();
let revealNonce = 0;
let groupCount = 1;

const FIRST_GROUP = 'g1';
const newGroup = (): EditorGroup => ({ id: `g${++groupCount}`, tabIds: [], activeId: null });

type Snapshot = Pick<EditorState, 'tabs' | 'groups' | 'activeGroup' | 'docs'>;

/**
 * O estado depois de mexer nos grupos: grupos vazios somem (fica sempre um),
 * o grupo ativo continua o mesmo ou passa ao vizinho da esquerda, e as abas
 * que não estão em grupo nenhum saem de `tabs` e de `docs`. Devolve também
 * essas abas, para o chamador descartar os modelos delas.
 */
function settle(state: Snapshot, groups: EditorGroup[], activeGroup: string): { patch: Partial<EditorState>; removed: Tab[] } {
  let kept = groups.filter((group) => group.tabIds.length > 0);
  if (kept.length === 0) kept = [{ id: FIRST_GROUP, tabIds: [], activeId: null }];
  let active = kept.find((group) => group.id === activeGroup);
  if (!active) {
    const index = groups.findIndex((group) => group.id === activeGroup);
    const left = groups.slice(0, Math.max(index, 0)).reverse();
    active = left.map((g) => kept.find((k) => k.id === g.id)).find(Boolean) ?? kept[0];
  }
  const open = new Set(kept.flatMap((group) => group.tabIds));
  const removed = state.tabs.filter((tab) => !open.has(tab.id));
  let docs = state.docs;
  if (removed.some((tab) => tab.path)) {
    // O arquivo de uma aba de diff que continua aberta (ou o contrário)
    // fica com o DocInfo.
    const still = new Set(state.tabs.filter((tab) => open.has(tab.id) && tab.path).map((tab) => tab.path));
    docs = { ...docs };
    for (const tab of removed) if (tab.path && !still.has(tab.path)) delete docs[tab.path];
  }
  return {
    patch: {
      groups: kept,
      activeGroup: active.id,
      activeId: active.activeId,
      tabs: removed.length ? state.tabs.filter((tab) => open.has(tab.id)) : state.tabs,
      docs,
    },
    removed,
  };
}

/** O grupo `id` sem a aba; a ativa passa à vizinha. */
function without(group: EditorGroup, id: string): EditorGroup {
  const index = group.tabIds.indexOf(id);
  if (index < 0) return group;
  const tabIds = group.tabIds.filter((tabId) => tabId !== id);
  const activeId = group.activeId === id ? (tabIds[Math.min(index, tabIds.length - 1)] ?? null) : group.activeId;
  return { ...group, tabIds, activeId };
}

/**
 * O grupo vizinho de `from` para o lado pedido. `null`: criar um grupo novo
 * ali, porque não há vizinho e ainda cabe. Sem vizinho e sem espaço, o do
 * outro lado.
 */
function neighbor(groups: EditorGroup[], from: string, side: Side): string | null {
  const index = groups.findIndex((group) => group.id === from);
  const next = groups[side === 'right' ? index + 1 : index - 1];
  if (next) return next.id;
  if (groups.length < MAX_GROUPS) return null;
  return groups[side === 'right' ? index - 1 : index + 1]?.id ?? from;
}

/**
 * Põe a aba `id` no grupo `target` (`null`: um grupo novo ao lado de
 * `beside`, do lado `side`) e a ativa no grupo. Com `preview`, ela toma o
 * lugar da aba provisória limpa do grupo. Sem `index`, entra depois da aba
 * ativa do grupo. Devolve os grupos e o id do grupo onde ela ficou.
 */
function place(
  state: Snapshot,
  id: string,
  target: string | null,
  options: { preview?: boolean; beside?: string; side?: Side; index?: number } = {},
): { groups: EditorGroup[]; group: string } {
  let groups = state.groups;
  let gid = target;
  if (gid === null) {
    const group = newGroup();
    const at = groups.findIndex((g) => g.id === (options.beside ?? state.activeGroup));
    const insert = options.side === 'left' ? at : at + 1;
    groups = [...groups.slice(0, insert), group, ...groups.slice(insert)];
    gid = group.id;
  }
  const mapped = groups.map((group) => {
    if (group.id !== gid) return group;
    if (group.tabIds.includes(id) && options.index === undefined) return { ...group, activeId: id };
    let tabIds = group.tabIds.filter((tabId) => tabId !== id);
    const previewIndex = options.preview
      ? tabIds.findIndex((tabId) => {
          const tab = state.tabs.find((t) => t.id === tabId);
          return tab?.preview && !tab.dirty;
        })
      : -1;
    if (previewIndex >= 0) {
      tabIds = [...tabIds.slice(0, previewIndex), id, ...tabIds.slice(previewIndex + 1)];
    } else {
      const at = options.index ?? tabIds.indexOf(group.activeId ?? '') + 1;
      const index = Math.max(0, Math.min(at, tabIds.length));
      tabIds = [...tabIds.slice(0, index), id, ...tabIds.slice(index)];
    }
    return { ...group, tabIds, activeId: id };
  });
  return { groups: mapped, group: gid };
}

/** O formato gravado da sessão. O antigo, de antes dos grupos, era
 * `{ tabs, active }`, e ainda é lido como um grupo só. */
interface SavedSession {
  groups?: { tabs: Pick<Tab, 'kind' | 'path' | 'data'>[]; active: string | null }[];
  activeGroup?: number;
  tabs?: Pick<Tab, 'kind' | 'path' | 'data'>[];
  active?: string | null;
}

export const useEditor = create<EditorState>((set, get) => {
  /** Aplica uma mudança de grupos e descarta os modelos dos arquivos que
   * deixaram de estar abertos. */
  const commit = (update: (state: EditorState) => { groups: EditorGroup[]; activeGroup: string; extra?: Partial<EditorState> }) => {
    let removed: Tab[] = [];
    set((state) => {
      const { groups, activeGroup, extra } = update(state);
      const base = extra ? { ...state, ...extra } : state;
      const settled = settle(base, groups, activeGroup);
      removed = settled.removed;
      return { ...extra, ...settled.patch };
    });
    // O modelo do Monaco é um por URI, e a URI não distingue `/` de `\`: só
    // sai quando nenhuma aba que ficou aberta o usa.
    const inUse = new Set(get().tabs.flatMap((tab) => (tab.path ? [uriFor(tab.path).toString()] : [])));
    for (const tab of removed) if (tab.path && !inUse.has(uriFor(tab.path).toString())) disposeModel(tab.path);
    return removed;
  };

  return {
    tabs: [],
    groups: [{ id: FIRST_GROUP, tabIds: [], activeId: null }],
    activeGroup: FIRST_GROUP,
    activeId: null,
    docs: {},
    closed: [],
    reveal: null,
    cursor: null,
    languageId: null,

    openFile: async (requested, options = {}) => {
      // Um arquivo, uma aba: o mesmo caminho escrito com outro separador
      // (o link do INFO usa `/`) acha a aba que já existe.
      const path = systemPath(requested);
      const id = fileTabId(path);
      let doc: DocInfo | null = null;
      // Aberto numa aba de diff, o arquivo já tem modelo e DocInfo.
      if (!get().tabs.some((tab) => tab.id === id) && !get().docs[path]) {
        doc = await loadDoc(path);
        if (!doc) return;
      }
      // O grupo é escolhido depois da leitura: o ativo pode ter mudado.
      const state = get();
      const target = options.side ? neighbor(state.groups, state.activeGroup, 'right') : state.activeGroup;
      const existing = state.tabs.find((tab) => tab.id === id);
      const already = target !== null && state.groups.find((g) => g.id === target)?.tabIds.includes(id);
      let landed = target;
      commit((s) => {
        const extra: Partial<EditorState> = {};
        if (!existing) {
          const dirty = s.tabs.some((tab) => tab.path === path && tab.dirty);
          extra.tabs = [...s.tabs, { id, kind: 'file', path, preview: options.preview && !dirty, dirty }];
          if (doc) extra.docs = { ...s.docs, [path]: doc };
        }
        const placed = place({ ...s, ...extra }, id, target, { preview: options.preview && !already, beside: s.activeGroup, side: 'right' });
        landed = placed.group;
        return { groups: placed.groups, activeGroup: placed.group, extra };
      });
      if (existing && !options.preview && existing.preview) get().pin(id);
      if (options.line) {
        set({ reveal: { path, line: options.line, column: options.column ?? 1, group: landed!, nonce: ++revealNonce } });
      }
    },

    openView: (kind, data) => {
      if (!viewAvailable(kind)) return;
      const id = viewTabId(kind, data);
      const holder = get().groups.find((group) => group.tabIds.includes(id));
      if (holder) {
        get().activate(id, holder.id);
        return;
      }
      commit((s) => {
        const extra = { tabs: [...s.tabs, { id, kind, data } as Tab] };
        return { groups: place({ ...s, ...extra }, id, s.activeGroup).groups, activeGroup: s.activeGroup, extra };
      });
    },

    openDiff: async (spec, options = {}) => {
      if (!viewAvailable('diff')) return;
      const data: Record<string, string | null> = {
        repo: spec.repo,
        rev: spec.rev ?? '',
        short: spec.short ?? null,
        parent: spec.parent ?? null,
        original: spec.original ?? null,
        change: spec.change,
      };
      const id = viewTabId('diff', data);
      const path = spec.file ? systemPath(spec.file) : undefined;
      let doc: DocInfo | null = null;
      if (path && !get().docs[path]) {
        doc = await loadDoc(path);
        if (!doc) return;
      }
      const existing = get().tabs.find((tab) => tab.id === id);
      commit((s) => {
        const dirty = !!path && s.tabs.some((tab) => tab.path === path && tab.dirty);
        const tabs: Tab[] = existing
          ? s.tabs.map((tab) => (tab.id === id ? { ...tab, data, path, dirty } : tab))
          : [...s.tabs, { id, kind: 'diff', data, path, dirty, preview: options.preview && !dirty }];
        const extra: Partial<EditorState> = { tabs };
        if (doc && path) extra.docs = { ...s.docs, [path]: doc };
        const holder = s.groups.find((group) => group.tabIds.includes(id))?.id ?? s.activeGroup;
        const placed = place({ ...s, ...extra }, id, holder, { preview: options.preview && !existing });
        return { groups: placed.groups, activeGroup: placed.group, extra };
      });
      if (existing?.preview && !options.preview) get().pin(id);
    },

    activate: (id, group) => {
      const { groups, activeGroup } = get();
      const holder =
        groups.find((g) => g.id === group && g.tabIds.includes(id)) ??
        groups.find((g) => g.id === activeGroup && g.tabIds.includes(id)) ??
        groups.find((g) => g.tabIds.includes(id));
      if (!holder) return;
      commit((s) => ({
        groups: s.groups.map((g) => (g.id === holder.id ? { ...g, activeId: id } : g)),
        activeGroup: holder.id,
      }));
    },

    focusGroup: (group) => {
      const state = get();
      if (state.activeGroup === group) return;
      const target = state.groups.find((g) => g.id === group);
      if (target) set({ activeGroup: group, activeId: target.activeId });
    },

    focusGroupAt: (index) => {
      const target = get().groups[index];
      if (target) get().focusGroup(target.id);
    },

    pin: (id) => set((state) => ({ tabs: state.tabs.map((t) => (t.id === id ? { ...t, preview: false } : t)) })),

    closeTab: async (id, group) => {
      const tab = get().tabs.find((t) => t.id === id);
      if (!tab) return true;
      const holders = get().groups.filter((g) => g.tabIds.includes(id));
      const closing = group ? holders.filter((g) => g.id === group) : holders;
      if (closing.length === 0) return true;
      const last = closing.length === holders.length;
      // Um arquivo também aberto noutra aba (a de arquivo e a de diff) não
      // perde nada ao fechar esta: o modelo continua com a outra.
      const shared = !!tab.path && get().tabs.some((other) => other.id !== id && other.path === tab.path);
      if (last && tab.dirty && tab.path && !shared) {
        const answer = await confirm({
          title: t('dialog.unsaved.title'),
          message: t('dialog.unsaved.message', { name: baseName(tab.path) }),
          buttons: [
            { label: t('common.save'), value: 'save', primary: true },
            { label: t('common.discard'), value: 'discard', danger: true },
            { label: t('common.cancel'), value: 'cancel' },
          ],
        });
        if (answer === 'save') {
          if (!(await get().save(id))) return false;
        } else if (answer !== 'discard') {
          return false;
        }
      }
      const from = new Set(closing.map((g) => g.id));
      const removed = commit((s) => ({
        groups: s.groups.map((g) => (from.has(g.id) ? without(g, id) : g)),
        activeGroup: s.activeGroup,
      }));
      if (removed.length) {
        set((state) => ({ closed: [...state.closed.slice(-9), ...removed.map((r) => ({ ...r, dirty: false, preview: false }))] }));
      }
      return true;
    },

    closeOthers: async (id, group) => {
      const holder = get().groups.find((g) => g.id === (group ?? get().activeGroup) && g.tabIds.includes(id)) ?? get().groups.find((g) => g.tabIds.includes(id));
      if (!holder) return;
      for (const other of holder.tabIds.filter((tabId) => tabId !== id)) {
        if (!(await get().closeTab(other, holder.id))) return;
      }
    },

    closeAll: async () => {
      const dirty = get().tabs.filter((t) => t.dirty);
      if (dirty.length > 1) {
        const answer = await confirm({
          title: t('dialog.unsaved.title'),
          message: t('dialog.unsavedMany.message', { count: dirty.length }),
          buttons: [
            { label: t('action.saveAll'), value: 'save', primary: true },
            { label: t('common.discard'), value: 'discard', danger: true },
            { label: t('common.cancel'), value: 'cancel' },
          ],
        });
        if (answer === 'save') {
          if (!(await get().saveAll())) return false;
        } else if (answer !== 'discard') {
          return false;
        }
        for (const tab of dirty) get().setDirty(tab.path!, false);
      }
      for (const tab of [...get().tabs]) {
        if (!(await get().closeTab(tab.id))) return false;
      }
      return true;
    },

    closeGroup: async (group) => {
      const target = get().groups.find((g) => g.id === group);
      if (!target) return true;
      for (const id of [...target.tabIds].reverse()) {
        if (!(await get().closeTab(id, group))) return false;
      }
      return true;
    },

    split: (id, side = 'right') => {
      const state = get();
      const tabId = id ?? state.activeId;
      if (!tabId) return;
      const from =
        state.groups.find((g) => g.id === state.activeGroup && g.tabIds.includes(tabId)) ??
        state.groups.find((g) => g.tabIds.includes(tabId));
      const tab = state.tabs.find((t) => t.id === tabId);
      if (!from || !tab) return;
      if (tab.kind !== 'file') {
        get().moveTo(tabId, from.id, side);
        return;
      }
      commit((s) => {
        const placed = place(s, tabId, neighbor(s.groups, from.id, side), { beside: from.id, side });
        return {
          groups: placed.groups,
          activeGroup: placed.group,
          // Dividida, a aba deixa de ser provisória.
          extra: { tabs: s.tabs.map((t) => (t.id === tabId ? { ...t, preview: false } : t)) },
        };
      });
    },

    moveTo: (id, from, side) => {
      commit((s) => {
        const target = neighbor(s.groups, from, side);
        if (target === from) return { groups: s.groups, activeGroup: s.activeGroup };
        const placed = place(s, id, target, { beside: from, side });
        return { groups: placed.groups.map((g) => (g.id === from ? without(g, id) : g)), activeGroup: placed.group };
      });
    },

    dropTab: (id, from, to, index) => {
      commit((s) => {
        const source = s.groups.find((g) => g.id === from);
        if (!source?.tabIds.includes(id)) return { groups: s.groups, activeGroup: s.activeGroup };
        // Contado sem a própria aba, quando ela anda dentro do grupo.
        const at = from === to && source.tabIds.indexOf(id) < index ? index - 1 : index;
        // Arrastada para outro grupo, a aba sai do de origem, como no VS
        // Code. Se o arquivo já estava no destino, fica uma aba só lá.
        const groups = s.groups.map((g) => (g.id === from && from !== to ? without(g, id) : g));
        return { groups: place({ ...s, groups }, id, to, { index: at }).groups, activeGroup: to };
      });
    },

    save: async (id) => {
      const tab = get().tabs.find((t) => t.id === (id ?? get().activeId));
      if (!tab?.path) return true;
      const path = tab.path;
      const doc = get().docs[path];
      const model = getModel(path);
      if (!doc || doc.binary || doc.tooLarge) return true;
      // Sem modelo não há texto para gravar: dizer que gravou fecharia a
      // aba e perderia o que estava nela.
      if (!model) {
        showError({ code: 'internal', message: `No editor model for ${path}: the file was not saved` });
        return false;
      }
      const write = async (expected: number | null) => {
        // O texto e a versão do mesmo instante: o que se digita enquanto o
        // arquivo grava continua por gravar, e a aba continua alterada. A
        // versão lida depois da gravação marcava como gravada uma tecla
        // que não chegou ao disco.
        const text = model.getValue();
        const savedVersion = model.getAlternativeVersionId();
        const modifiedMs = await api.fs.writeText(path, text, expected);
        set((state) => ({
          docs: { ...state.docs, [path]: { ...state.docs[path], modifiedMs, savedVersion, deleted: false } },
          tabs: state.tabs.map((t) =>
            t.path === path ? { ...t, dirty: model.getAlternativeVersionId() !== savedVersion, preview: false } : t,
          ),
        }));
      };
      try {
        await write(doc.deleted ? null : doc.modifiedMs);
        return true;
      } catch (error) {
        if ((error as IpcError).code !== 'conflict') {
          showError(error);
          return false;
        }
      }
      const answer = await confirm({
        title: t('dialog.conflict.title'),
        message: t('dialog.conflict.message', { name: baseName(path) }),
        buttons: [
          { label: t('common.overwrite'), value: 'overwrite', danger: true },
          { label: t('common.reload'), value: 'reload' },
          { label: t('common.cancel'), value: 'cancel' },
        ],
      });
      if (answer === 'overwrite') {
        try {
          await write(null);
          return true;
        } catch (error) {
          showError(error);
          return false;
        }
      }
      if (answer === 'reload') await reload(path);
      return false;
    },

    saveAll: async () => {
      for (const tab of get().tabs.filter((t) => t.dirty)) {
        if (!(await get().save(tab.id))) return false;
      }
      return true;
    },

    setDirty: (path, dirty) => {
      const tab = get().tabs.find((t) => t.path === path);
      if (!tab || tab.dirty === dirty) return;
      set((state) => ({
        tabs: state.tabs.map((t) => (t.path === path ? { ...t, dirty, preview: dirty ? false : t.preview } : t)),
      }));
    },

    onDiskChange: async (paths) => {
      const open = new Set(Object.keys(get().docs));
      for (const path of new Set(paths)) {
        if (!open.has(path)) continue;
        const doc = get().docs[path];
        const stat = await api.fs.stat(path).catch(() => null);
        if (!stat) {
          set((state) => ({ docs: { ...state.docs, [path]: { ...doc, deleted: true } } }));
          continue;
        }
        if (stat.modified_ms === doc.modifiedMs) continue;
        const tab = get().tabs.find((t) => t.path === path);
        if (tab?.dirty) {
          // Com alterações locais, não recarrega: o próximo "salvar" avisa
          // do conflito e deixa escolher.
          continue;
        }
        await reload(path);
      }
    },

    renamed: (from, to) => {
      // `isInside` aceita os dois separadores: no Windows os caminhos vêm com `\`.
      // Um arquivo pode estar numa aba de arquivo e numa de diff: o modelo
      // muda uma vez, e as duas abas passam ao caminho novo. A de diff fica
      // com o mesmo id (o caminho no repositório).
      const paths = new Set(get().tabs.flatMap((t) => (t.path && isInside(t.path, from) ? [t.path] : [])));
      for (const oldPath of paths) {
        const newPath = to + oldPath.slice(from.length);
        const model = getModel(oldPath);
        const content = model?.getValue() ?? '';
        const dirty = get().tabs.some((t) => t.path === oldPath && t.dirty);
        disposeModel(oldPath);
        const fresh = ensureModel(newPath, content);
        const ids = new Map(get().tabs.filter((t) => t.path === oldPath && t.kind === 'file').map((t) => [t.id, fileTabId(newPath)]));
        const rename = (tabId: string | null) => (tabId !== null && ids.has(tabId) ? ids.get(tabId)! : tabId);
        set((state) => {
          const docs = { ...state.docs };
          const doc = docs[oldPath];
          delete docs[oldPath];
          docs[newPath] = { ...doc, path: newPath, savedVersion: dirty ? -1 : fresh.getAlternativeVersionId() };
          return {
            docs,
            tabs: state.tabs.map((t) => (t.path === oldPath ? { ...t, id: rename(t.id)!, path: newPath } : t)),
            groups: state.groups.map((g) => ({ ...g, tabIds: g.tabIds.map((tabId) => rename(tabId)!), activeId: rename(g.activeId) })),
            activeId: rename(state.activeId),
          };
        });
        watched.delete(oldPath);
        watch(newPath, fresh);
      }
    },

    reopenClosed: () => {
      const closed = [...get().closed];
      const tab = closed.pop();
      set({ closed });
      if (!tab) return;
      if (tab.kind === 'file' && tab.path) void get().openFile(tab.path);
      else if (tab.kind === 'diff') void get().openDiff(diffSpecOf(tab));
      else if (tab.kind !== 'file') get().openView(tab.kind, tab.data);
    },

    setCursor: (line, column, languageId) => set({ cursor: { line, column }, languageId }),

    reset: () => {
      for (const tab of get().tabs) if (tab.path) disposeModel(tab.path);
      set({
        tabs: [],
        groups: [{ id: FIRST_GROUP, tabIds: [], activeId: null }],
        activeGroup: FIRST_GROUP,
        activeId: null,
        docs: {},
        closed: [],
        reveal: null,
        cursor: null,
        languageId: null,
      });
    },

    persistSession: (spf) => {
      const { tabs, groups, activeGroup } = get();
      const kept = groups
        .map((group) => ({
          tabs: group.tabIds
            .map((id) => tabs.find((tab) => tab.id === id))
            .filter((tab): tab is Tab => !!tab && !tab.preview && tab.kind !== 'compare' && tab.kind !== 'diff')
            .map(({ kind, path, data }) => ({ kind, path, data })),
          active: group.activeId,
          id: group.id,
        }))
        .filter((group) => group.tabs.length > 0);
      const session: SavedSession = {
        groups: kept.map(({ tabs, active }) => ({ tabs, active })),
        activeGroup: Math.max(0, kept.findIndex((group) => group.id === activeGroup)),
      };
      try {
        localStorage.setItem(`aurora-ide:session:${spf}`, JSON.stringify(session));
      } catch {
        // Sem armazenamento, a sessão só não é lembrada.
      }
    },

    restoreSession: async (spf) => {
      let session: SavedSession | null = null;
      try {
        session = JSON.parse(localStorage.getItem(`aurora-ide:session:${spf}`) ?? 'null');
      } catch {
        session = null;
      }
      const saved = session?.groups ?? (session?.tabs ? [{ tabs: session.tabs, active: session.active ?? null }] : []);
      const restored: string[] = [];
      for (const [index, group] of saved.slice(0, MAX_GROUPS).entries()) {
        if (index > 0) {
          // Cada grupo gravado vira um grupo novo à direita, e as abas dele
          // abrem ali (abrir usa o grupo ativo).
          const fresh = newGroup();
          set((state) => ({ groups: [...state.groups, fresh], activeGroup: fresh.id, activeId: null }));
        }
        for (const tab of group.tabs ?? []) {
          if (tab.kind === 'file' && tab.path) {
            const exists = await api.fs.stat(tab.path).catch(() => null);
            if (exists && !exists.is_dir) await get().openFile(tab.path);
          } else if (tab.kind !== 'file' && tab.kind !== 'compare' && tab.kind !== 'diff' && String(tab.kind) !== 'welcome') {
            // A aba de boas-vindas saiu (o editor vazio mostra a mesma tela),
            // mas pode estar numa sessão gravada antes.
            get().openView(tab.kind, tab.data);
          }
        }
        const current = get().activeGroup;
        restored.push(current);
        if (group.active) get().activate(group.active, current);
      }
      // Grupos que ficaram vazios (arquivos que sumiram) saem aqui.
      const focus = restored[session?.activeGroup ?? 0] ?? restored[0];
      commit((s) => ({ groups: s.groups, activeGroup: focus ?? s.activeGroup }));
    },
  };
});

/** Lê o arquivo do disco para um modelo (o que já existe, se houver) e
 * devolve o DocInfo dele; `null` se não deu para ler (o erro já apareceu). */
async function loadDoc(path: string): Promise<DocInfo | null> {
  let file;
  try {
    file = await api.fs.readText(path);
  } catch (error) {
    showError(error);
    return null;
  }
  const model = ensureModel(path, file.content);
  watch(path, model);
  return {
    path,
    modifiedMs: file.modified_ms,
    savedVersion: model.getAlternativeVersionId(),
    binary: file.binary,
    tooLarge: file.too_large,
  };
}

/** O que uma aba de diff compara, de volta à forma de `openDiff`. */
export function diffSpecOf(tab: Tab): DiffSpec {
  const data = tab.data ?? {};
  return {
    repo: data.repo ?? '',
    rev: data.rev || null,
    short: data.short ?? null,
    parent: data.parent ?? null,
    original: data.original ?? null,
    change: data.change ?? 'modified',
    file: tab.path ?? null,
  };
}

/** Marca a aba como alterada quando o modelo muda (uma vez por modelo). */
function watch(path: string, model: NonNullable<ReturnType<typeof getModel>>): void {
  if (watched.has(path)) return;
  watched.add(path);
  model.onDidChangeContent(() => {
    const doc = useEditor.getState().docs[path];
    if (doc) useEditor.getState().setDirty(path, model.getAlternativeVersionId() !== doc.savedVersion);
  });
  model.onWillDispose(() => watched.delete(path));
}

/** Relê um arquivo do disco para o modelo, descartando o que não foi salvo. */
async function reload(path: string): Promise<void> {
  const model = getModel(path);
  if (!model) return;
  try {
    const file = await api.fs.readText(path);
    model.setValue(file.content);
    useEditor.setState((state) => ({
      docs: {
        ...state.docs,
        [path]: { ...state.docs[path], modifiedMs: file.modified_ms, savedVersion: model.getAlternativeVersionId(), deleted: false },
      },
      tabs: state.tabs.map((t) => (t.path === path ? { ...t, dirty: false } : t)),
    }));
  } catch (error) {
    showError(error);
  }
}

/** Grava a aba ativa (Ctrl+S). */
export async function saveActive(): Promise<void> {
  await useEditor.getState().save();
}

// Um recurso experimental desligado leva as abas das vistas dele.
useExperimental.subscribe((state, previous) => {
  if (state.enabled === previous.enabled) return;
  for (const tab of useEditor.getState().tabs) {
    if (tab.kind !== 'file' && !viewAvailable(tab.kind)) void useEditor.getState().closeTab(tab.id);
  }
});
