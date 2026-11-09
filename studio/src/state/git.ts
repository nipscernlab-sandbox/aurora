// O controle de versão (recurso experimental `git`): o repositório que
// contém o projeto, pelo `git.status` do servidor, e o que a vista Controle
// de versão, o Explorador e a barra de status leem dele. A conta do GitHub
// fica em github.ts.
//
// O estado é relido quando algo muda no disco (o vigia avisa também do HEAD,
// do índice e das referências do .git, watcher.rs), quando a janela volta a
// ter o foco, depois de cada operação da vista, ao gravar um arquivo e ao
// abrir ou fechar o projeto. Com o recurso desligado, a store fica vazia e
// nada é pedido ao servidor.
//
// A seleção dos arquivos do commit segue o GitHub Desktop: todos marcados, e
// a store guarda os desmarcados. Um arquivo novo entra marcado; um que saiu
// da lista sai também dos desmarcados.

import { create } from 'zustand';

import { errorText, t } from '../i18n';
import { api } from '../ipc/api';
import type { Branches, ChangedFile, CommitFiles, CommitSummary, GitStatus, RepositoryStatus } from '../ipc/aurora-types';
import type { IpcError } from '../ipc/types';
import { baseName, systemPath } from '../util/paths';
import { confirm } from './dialogs';
import { useEditor } from './editor';
import { featureOn, useExperimental } from './experimental';
import { useLayout } from './layout';
import { useProject } from './project';
import { showError, useToasts } from './toasts';

export type GitChange = ChangedFile['change'];

/** A operação da vista que está rodando: uma de cada vez. */
export type GitBusy = 'commit' | 'push' | 'pull' | 'fetch' | 'switch' | 'discard' | 'init' | 'identity' | 'remote';

/** O que o botão de sincronizar faz agora, como no GitHub Desktop. */
export type SyncKind = 'remote' | 'publish' | 'pull' | 'push' | 'fetch';

/** Quantos commits o histórico traz por vez. */
const LOG_PAGE = 50;

interface GitState {
  /** O último `git.status`; `null` sem projeto, com o recurso desligado ou
   * quando a leitura falhou (`error`). */
  status: GitStatus | null;
  /** O erro da última leitura: `git_missing`, `git_dubious_ownership`... */
  error: IpcError | null;
  loading: boolean;
  /** Já houve uma leitura (com ou sem erro) para o projeto aberto. */
  loaded: boolean;
  /** Os arquivos desmarcados para o commit, pelo caminho relativo ao topo. */
  excluded: ReadonlySet<string>;
  /** A mudança de cada arquivo, pelo caminho absoluto (gitKey). */
  marks: ReadonlyMap<string, GitChange>;
  /** As pastas que contêm um arquivo mudado (gitKey). */
  dirs: ReadonlySet<string>;
  busy: GitBusy | null;
  /** A aba da vista. */
  tab: 'changes' | 'history';
  /** O que está escrito na caixa de commit. */
  summary: string;
  description: string;
  /** O formulário do remoto aberto na vista: um remoto novo (o repositório
   * não tem nenhum) ou o endereço de um que existe, para corrigir. */
  remoteForm: RemoteForm | null;
  /** A troca de ramo parou nestes arquivos (`git_switch_blocked`). */
  switchBlocked: string[] | null;
  /** O erro de criar ou trocar de ramo que aparece no seletor. */
  branchError: IpcError | null;
  branches: Branches | null;
  branchesLoading: boolean;
  log: { commits: CommitSummary[]; more: boolean } | null;
  logLoading: boolean;
  logError: IpcError | null;
  /** Os arquivos de cada commit aberto no histórico. */
  commitFiles: Record<string, CommitFiles>;
  /** Os commits abertos no histórico. */
  expanded: ReadonlySet<string>;

  setTab: (tab: 'changes' | 'history') => void;
  setDraft: (draft: { summary?: string; description?: string }) => void;
  setExcluded: (paths: string[], excluded: boolean) => void;
  toggleCommit: (oid: string) => void;
  dismissSwitchBlocked: () => void;
  closeRemoteForm: () => void;
}

/** O formulário do remoto. */
export interface RemoteForm {
  /** O nome do remoto (`origin`). */
  name: string;
  /** O endereço de agora; vazio num remoto novo. */
  url: string;
  /** O remoto já existe: o formulário troca o endereço e pode removê-lo. */
  exists: boolean;
  /** Por que o formulário abriu sozinho: o remoto não respondeu nesse
   * endereço. */
  error: IpcError | null;
  /** O que refazer depois de gravar o endereço. */
  retry: 'push' | 'pull' | 'fetch' | null;
}

const EMPTY = {
  status: null,
  error: null,
  loading: false,
  loaded: false,
  excluded: new Set<string>(),
  marks: new Map<string, GitChange>(),
  dirs: new Set<string>(),
  busy: null,
  remoteForm: null,
  switchBlocked: null,
  branchError: null,
  branches: null,
  branchesLoading: false,
  log: null,
  logLoading: false,
  logError: null,
  commitFiles: {},
  expanded: new Set<string>(),
} satisfies Partial<GitState>;

export const useGit = create<GitState>((set, get) => ({
  ...EMPTY,
  tab: 'changes',
  summary: '',
  description: '',

  setTab: (tab) => {
    set({ tab });
    if (tab === 'history' && !get().log && !get().logLoading) void loadLog(true);
  },
  setDraft: (draft) => set(draft),
  setExcluded: (paths, excluded) => {
    const next = new Set(get().excluded);
    for (const path of paths) {
      if (excluded) next.add(path);
      else next.delete(path);
    }
    set({ excluded: next });
  },
  toggleCommit: (oid) => {
    const next = new Set(get().expanded);
    if (next.has(oid)) next.delete(oid);
    else next.add(oid);
    set({ expanded: next });
    if (next.has(oid) && !get().commitFiles[oid]) void loadCommitFiles(oid);
  },
  dismissSwitchBlocked: () => set({ switchBlocked: null }),
  closeRemoteForm: () => set({ remoteForm: null }),
}));

// Caminhos ---------------------------------------------------------------

/** A chave de um caminho absoluto nas marcas: com `/`, sem barra no fim e
 * com a letra da unidade em maiúscula (o git escreve `C:/`, o backend
 * `C:\`). */
export function gitKey(path: string): string {
  const slashed = path.replace(/\\/g, '/').replace(/\/+$/, '');
  return /^[a-z]:/.test(slashed) ? slashed[0].toUpperCase() + slashed.slice(1) : slashed;
}

/** O caminho absoluto, no formato do sistema, de um arquivo do repositório. */
export function repoFile(repo: RepositoryStatus, path: string): string {
  return systemPath(`${repo.top.replace(/\/+$/, '')}/${path}`);
}

function marksOf(repo: RepositoryStatus | null): { marks: Map<string, GitChange>; dirs: Set<string> } {
  const marks = new Map<string, GitChange>();
  const dirs = new Set<string>();
  if (!repo) return { marks, dirs };
  const top = gitKey(repo.top);
  for (const file of repo.files) {
    const key = gitKey(`${top}/${file.path}`);
    marks.set(key, file.change);
    for (let dir = key.slice(0, key.lastIndexOf('/')); dir.length >= top.length && !dirs.has(dir); dir = dir.slice(0, dir.lastIndexOf('/'))) {
      dirs.add(dir);
    }
  }
  return { marks, dirs };
}

/** A marca do git de uma linha do Explorador: a mudança do arquivo, `dir`
 * para uma pasta que contém mudança, ou `null`. */
export function useGitMark(path: string | undefined, dir: boolean): GitChange | 'dir' | null {
  return useGit((s) => {
    if (!path || s.marks.size === 0) return null;
    const key = gitKey(path);
    if (dir) return s.dirs.has(key) ? 'dir' : null;
    return s.marks.get(key) ?? null;
  });
}

/** O que o botão de sincronizar faz com o repositório como está. */
export function syncKind(repo: RepositoryStatus): SyncKind {
  if (repo.remotes.length === 0) return 'remote';
  if (!repo.branch.name) return 'fetch';
  if (!repo.branch.upstream) return 'publish';
  if (repo.branch.behind > 0) return 'pull';
  if (repo.branch.ahead > 0) return 'push';
  return 'fetch';
}

/** Os arquivos que vão no commit: os da lista que não foram desmarcados. */
export function selectedFiles(state: Pick<GitState, 'status' | 'excluded'>): ChangedFile[] {
  return (state.status?.repository?.files ?? []).filter((file) => !state.excluded.has(file.path));
}

// Leitura ----------------------------------------------------------------

/** O ramo e o commit de agora, para saber quando o histórico e os ramos
 * precisam ser relidos. */
function headKey(status: GitStatus | null): string {
  const branch = status?.repository?.branch;
  return branch ? `${branch.name}|${branch.commit}|${branch.upstream}|${branch.ahead}|${branch.behind}` : '';
}

/** Os erros da leitura que não são do git: o projeto fechou ou sumiu, ou o
 * recurso foi desligado por fora. A vista só fica vazia. */
const QUIET = new Set(['no_project', 'spf_missing', 'experimental_disabled', 'server_gone']);

let reading: Promise<void> | null = null;
let again = false;
let timer = 0;

function active(): boolean {
  return featureOn('git') && useProject.getState().snapshot !== null;
}

/** Relê o estado do repositório agora. Uma leitura pedida enquanto outra
 * roda acontece logo depois dela. */
export async function refreshGit(): Promise<void> {
  if (!active()) {
    clearGit();
    return;
  }
  if (reading) {
    again = true;
    return reading;
  }
  reading = (async () => {
    do {
      again = false;
      const spf = useProject.getState().snapshot?.spf;
      if (!spf || !featureOn('git')) break;
      useGit.setState({ loading: true });
      try {
        const status = await api.git.status();
        if (spf !== useProject.getState().snapshot?.spf) continue;
        applyStatus(status);
      } catch (error) {
        if (spf !== useProject.getState().snapshot?.spf) continue;
        const e = error as IpcError;
        if (e.code === 'experimental_disabled') void useExperimental.getState().load();
        useGit.setState({ status: null, error: QUIET.has(e.code) ? null : e, loaded: true, marks: EMPTY.marks, dirs: EMPTY.dirs });
      } finally {
        useGit.setState({ loading: false });
      }
    } while (again);
  })();
  try {
    await reading;
  } finally {
    reading = null;
  }
}

/** Relê daqui a pouco, juntando os pedidos que chegarem antes. */
export function refreshGitSoon(ms = 250): void {
  if (!active()) return;
  window.clearTimeout(timer);
  timer = window.setTimeout(() => void refreshGit(), ms);
}

function clearGit(): void {
  window.clearTimeout(timer);
  useGit.setState({ ...EMPTY });
}

function applyStatus(status: GitStatus): void {
  const previous = useGit.getState();
  const repo = status.repository;
  const present = new Set((repo?.files ?? []).map((file) => file.path));
  const excluded = new Set([...previous.excluded].filter((path) => present.has(path)));
  // Um arquivo de fora da pasta do projeto (o projeto dentro de um
  // repositório maior) chega desmarcado: o commit não leva por engano a
  // mudança de outra parte do repositório. Os de dentro chegam marcados.
  const known = new Set((previous.status?.repository?.files ?? []).map((file) => file.path));
  for (const file of repo?.files ?? []) {
    if (!file.in_project && !known.has(file.path)) excluded.add(file.path);
  }
  const { marks, dirs } = marksOf(repo);
  const moved = headKey(previous.status) !== headKey(status);
  useGit.setState({
    status,
    error: null,
    loaded: true,
    excluded,
    marks,
    dirs,
    // O formulário de um remoto que sumiu (removido no terminal) fecha.
    remoteForm:
      previous.remoteForm && (!previous.remoteForm.exists || repo?.remotes.some((r) => r.name === previous.remoteForm?.name))
        ? previous.remoteForm
        : null,
  });
  if (!repo) {
    useGit.setState({ branches: null, log: null, commitFiles: {}, expanded: new Set() });
    return;
  }
  if (moved && previous.log) void loadLog(true);
  if (moved && previous.branches) void loadBranches();
}

export async function loadBranches(): Promise<void> {
  if (!active()) return;
  useGit.setState({ branchesLoading: true });
  try {
    useGit.setState({ branches: await api.git.branches() });
  } catch (error) {
    showError(error);
  } finally {
    useGit.setState({ branchesLoading: false });
  }
}

/** A primeira página do histórico (`reset`) ou a próxima. Ao reler a
 * primeira, traz pelo menos tantos commits quanto já estavam na lista. */
export async function loadLog(reset: boolean): Promise<void> {
  if (!active() || useGit.getState().logLoading) return;
  const current = useGit.getState().log;
  const skip = reset || !current ? 0 : current.commits.length;
  const limit = reset && current ? Math.min(500, Math.max(LOG_PAGE, current.commits.length)) : LOG_PAGE;
  useGit.setState({ logLoading: true });
  try {
    const page = await api.git.log(skip, limit);
    const commits = skip === 0 ? page.commits : [...(current?.commits ?? []), ...page.commits];
    useGit.setState({ log: { commits, more: page.more }, logError: null });
  } catch (error) {
    useGit.setState({ logError: error as IpcError });
  } finally {
    useGit.setState({ logLoading: false });
  }
}

async function loadCommitFiles(oid: string): Promise<void> {
  try {
    const files = await api.git.commitFiles(oid);
    useGit.setState((s) => ({ commitFiles: { ...s.commitFiles, [oid]: files } }));
  } catch (error) {
    showError(error);
    useGit.setState((s) => {
      const expanded = new Set(s.expanded);
      expanded.delete(oid);
      return { expanded };
    });
  }
}

// Operações --------------------------------------------------------------

/** O que refazer depois de corrigir o remoto, pela operação que falhou. */
function retryOf(busy: GitBusy | null): RemoteForm['retry'] {
  return busy === 'push' || busy === 'pull' || busy === 'fetch' ? busy : busy === 'remote' ? 'push' : null;
}

/** Os erros que a vista mostra do jeito dela; o resto vira aviso. */
function handleGitError(error: IpcError, busy: GitBusy | null): void {
  switch (error.code) {
    case 'git_no_remote':
      openRemoteForm(null, retryOf(busy));
      return;
    case 'git_remote_unreachable': {
      const name = typeof error.data?.remote === 'string' ? error.data.remote : null;
      openRemoteForm(name, retryOf(busy), error);
      return;
    }
    case 'git_switch_blocked': {
      const files = Array.isArray(error.data?.files) ? (error.data.files as unknown[]).map(String) : [];
      useGit.setState({ switchBlocked: files });
      return;
    }
    case 'git_invalid_branch_name':
    case 'git_branch_exists':
    case 'git_branch_not_found':
      useGit.setState({ branchError: error });
      return;
    case 'git_authentication':
      useToasts.getState().push({
        kind: 'error',
        title: errorText(error),
        detail: error.message,
        action: { label: t('git.signIn'), run: () => useLayout.getState().revealView('profile', { explicit: true }) },
      });
      return;
    case 'git_push_rejected':
      useToasts.getState().push({
        kind: 'error',
        title: errorText(error),
        detail: error.message,
        action: { label: t('git.sync.pullNow'), run: () => void pull() },
      });
      return;
    default:
      showError(error);
  }
}

/** Roda uma operação da vista: uma de cada vez, com o erro tratado, e o
 * estado relido no fim. `true` se deu certo. */
async function run(busy: GitBusy, operation: () => Promise<void>): Promise<boolean> {
  if (useGit.getState().busy) return false;
  useGit.setState({ busy });
  try {
    await operation();
    return true;
  } catch (error) {
    handleGitError(error as IpcError, busy);
    return false;
  } finally {
    useGit.setState({ busy: null });
    void refreshGit();
  }
}

function repository(): RepositoryStatus | null {
  return useGit.getState().status?.repository ?? null;
}

/** Os arquivos marcados que estão abertos com alteração não gravada:
 * pergunta se grava antes, porque o commit leva o que está no disco.
 * `false` se o usuário cancelou ou a gravação falhou. */
async function saveBeforeCommit(repo: RepositoryStatus, files: ChangedFile[]): Promise<boolean> {
  const wanted = new Set(files.map((file) => gitKey(repoFile(repo, file.path))));
  const editor = useEditor.getState();
  const dirty = editor.tabs.filter((tab) => tab.dirty && tab.path && wanted.has(gitKey(tab.path)));
  if (dirty.length === 0) return true;
  const names = [...new Set(dirty.map((tab) => baseName(tab.path!)))];
  const answer = await confirm({
    title: t('git.unsaved.title'),
    message: t('git.unsaved.message', { files: names.join(', ') }),
    buttons: [
      { label: t('git.unsaved.save'), value: 'save', primary: true },
      { label: t('git.unsaved.skip'), value: 'skip' },
      { label: t('common.cancel'), value: 'cancel' },
    ],
  });
  if (answer === 'skip') return true;
  if (answer !== 'save') return false;
  for (const tab of dirty) {
    if (!(await useEditor.getState().save(tab.id))) return false;
  }
  return true;
}

export async function commit(): Promise<boolean> {
  const state = useGit.getState();
  const repo = repository();
  const summary = state.summary.trim();
  const files = selectedFiles(state);
  if (!repo || !summary || files.length === 0 || state.busy) return false;
  if (!(await saveBeforeCommit(repo, files))) return false;
  return run('commit', async () => {
    const description = useGit.getState().description.trim();
    const result = await api.git.commit(summary, description || null, files.map((file) => file.path));
    useGit.setState({ summary: '', description: '' });
    useToasts.getState().push({ kind: 'success', title: t('git.committed', { short: result.short }), detail: result.summary });
  });
}

/** Descarta as mudanças de um arquivo, depois da confirmação. */
export async function discard(file: ChangedFile): Promise<boolean> {
  const name = baseName(file.path);
  const created = file.change === 'untracked' || file.change === 'added';
  const answer = await confirm({
    title: t('git.discard.title'),
    message: t(created ? 'git.discard.messageNew' : 'git.discard.message', { name }),
    buttons: [
      { label: t('git.discard.confirm'), value: 'yes', danger: true },
      { label: t('common.cancel'), value: 'no' },
    ],
  });
  if (answer !== 'yes') return false;
  return run('discard', async () => {
    await api.git.discard([file.path]);
  });
}

export function init(): Promise<boolean> {
  return run('init', async () => {
    const result = await api.git.init();
    useToasts.getState().push({
      kind: 'success',
      title: t('git.init.done', { branch: result.branch }),
      detail: result.ignored.length ? t('git.init.ignored', { lines: result.ignored.join(', ') }) : undefined,
    });
  });
}

export function setIdentity(name: string, email: string, global: boolean): Promise<boolean> {
  return run('identity', async () => {
    await api.git.setIdentity(name.trim(), email.trim(), global);
  });
}

/** Troca de ramo, ou cria um (`create`). `true` se trocou. */
export function switchBranch(branch: string, create = false): Promise<boolean> {
  useGit.setState({ switchBlocked: null, branchError: null });
  return run('switch', async () => {
    await api.git.switchTo(branch, create);
  });
}

export function fetch(): Promise<boolean> {
  return run('fetch', async () => {
    await api.git.fetch();
  });
}

export function pull(): Promise<boolean> {
  return run('pull', async () => {
    const result = await api.git.pull();
    useToasts.getState().push({ kind: 'success', title: t('git.pulled', { upstream: result.upstream ?? result.remote }) });
  });
}

export function push(): Promise<boolean> {
  return run('push', async () => {
    const result = await api.git.push();
    useToasts.getState().push({
      kind: 'success',
      title: t(result.published ? 'git.published' : 'git.pushed', { upstream: result.upstream ?? result.remote }),
    });
  });
}

/** O botão de sincronizar: publica, traz, envia ou busca, conforme o ramo.
 * Sem remoto, abre o campo do endereço. */
export function sync(): Promise<boolean> {
  const repo = repository();
  if (!repo) return Promise.resolve(false);
  switch (syncKind(repo)) {
    case 'remote':
      openRemoteForm(null, 'push');
      return Promise.resolve(false);
    case 'pull':
      return pull();
    case 'publish':
    case 'push':
      return push();
    default:
      return fetch();
  }
}

/** Abre o formulário do remoto `name` (o do ramo, ou o primeiro, sem nome);
 * sem remoto nenhum, o de um remoto novo, `origin`. */
export function openRemoteForm(name: string | null, retry: RemoteForm['retry'] = null, error: IpcError | null = null): void {
  const repo = repository();
  const remotes = repo?.remotes ?? [];
  const chosen = remotes.find((r) => r.name === name) ?? remotes.find((r) => r.name === repo?.branch.remote) ?? remotes[0];
  useGit.setState({
    remoteForm: chosen
      ? { name: chosen.name, url: chosen.url, exists: true, error, retry }
      : { name: name ?? 'origin', url: '', exists: false, error, retry },
  });
}

/** Grava o endereço do remoto do formulário (cria ou corrige) e refaz a
 * operação que tinha falhado; um remoto novo, publica o ramo. */
export function saveRemote(url: string): Promise<boolean> {
  const form = useGit.getState().remoteForm;
  if (!form || !url.trim()) return Promise.resolve(false);
  return run('remote', async () => {
    await api.git.setRemote(form.name, url.trim());
    useGit.setState({ remoteForm: null });
    const retry = form.retry ?? (form.exists ? null : 'push');
    if (retry === 'push') {
      const result = await api.git.push();
      useToasts.getState().push({
        kind: 'success',
        title: t(result.published ? 'git.published' : 'git.pushed', { upstream: result.upstream ?? result.remote }),
      });
    } else if (retry === 'pull') {
      const result = await api.git.pull();
      useToasts.getState().push({ kind: 'success', title: t('git.pulled', { upstream: result.upstream ?? result.remote }) });
    } else if (retry === 'fetch') {
      await api.git.fetch();
    } else {
      useToasts.getState().push({ kind: 'success', title: t('git.remote.saved', { name: form.name }) });
    }
  });
}

/** Tira o remoto do formulário, depois da confirmação. */
export async function removeRemote(): Promise<boolean> {
  const form = useGit.getState().remoteForm;
  if (!form?.exists) return false;
  const answer = await confirm({
    title: t('git.remote.removeTitle', { name: form.name }),
    message: t('git.remote.removeMessage', { name: form.name, url: form.url }),
    buttons: [
      { label: t('git.remote.remove'), value: 'yes', danger: true },
      { label: t('common.cancel'), value: 'no' },
    ],
  });
  if (answer !== 'yes') return false;
  return run('remote', async () => {
    await api.git.removeRemote(form.name);
    useGit.setState({ remoteForm: null });
  });
}

// Abrir as comparações ---------------------------------------------------

/** A comparação das mudanças do disco de um arquivo com o último commit. O
 * lado novo é o próprio arquivo, editável; um arquivo apagado não tem lado
 * novo. */
export function openChange(file: ChangedFile, preview = true): void {
  const repo = repository();
  if (!repo) return;
  void useEditor.getState().openDiff(
    {
      repo: file.path,
      rev: null,
      original: file.original,
      change: file.change,
      file: file.change === 'deleted' ? null : repoFile(repo, file.path),
    },
    { preview },
  );
}

/** A comparação de um arquivo de um commit com o pai dele. */
export function openCommitChange(
  commit: Pick<CommitSummary, 'oid' | 'short'>,
  parent: string | null,
  file: { path: string; original: string | null; change: GitChange },
  preview = true,
): void {
  void useEditor.getState().openDiff(
    { repo: file.path, rev: commit.oid, short: commit.short, parent, original: file.original, change: file.change },
    { preview },
  );
}

// Quando reler -----------------------------------------------------------

// O recurso ligou ou desligou.
useExperimental.subscribe((state, previous) => {
  if (state.enabled === previous.enabled) return;
  if (state.enabled.has('git')) void refreshGit();
  else clearGit();
});

// Outro projeto (ou nenhum): a lista, a seleção e o histórico são de outro
// repositório.
useProject.subscribe((state, previous) => {
  if (state.snapshot?.spf === previous.snapshot?.spf) return;
  useGit.setState({ ...EMPTY, summary: '', description: '', tab: useGit.getState().tab });
  if (state.snapshot) void refreshGit();
});

// Um arquivo gravado pelo editor: o vigia avisa dos que estão na pasta do
// projeto, mas o repositório pode ser maior que ela.
useEditor.subscribe((state, previous) => {
  if (state.docs === previous.docs) return;
  for (const [path, doc] of Object.entries(state.docs)) {
    const before = previous.docs[path];
    if (before && before.modifiedMs !== doc.modifiedMs) {
      refreshGitSoon();
      return;
    }
  }
});
