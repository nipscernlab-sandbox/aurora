// A vista Controle de versão (recurso experimental `git`), no modelo do
// GitHub Desktop, numa coluna que cabe na barra lateral estreita: o
// repositório e o ramo em cima, com o seletor de ramo e o botão de
// sincronizar; embaixo, as abas Mudanças e Histórico. Tudo abre dentro da
// vista: o seletor de ramo, o endereço do remoto, a identidade dos commits
// e os arquivos de um commit do histórico. A comparação de um arquivo abre
// numa aba de diff (views/DiffView.tsx).
//
// O estado e as operações ficam em state/git.ts; a conta do GitHub, em
// state/github.ts.

import { openUrl, revealItemInDir } from '@tauri-apps/plugin-opener';
import {
  ArrowDown,
  ArrowUp,
  Check,
  ChevronDown,
  ChevronRight,
  CloudUpload,
  Copy,
  Download,
  FileText,
  FolderGit2,
  GitBranch,
  GitBranchPlus,
  Pencil,
  RefreshCw,
  TriangleAlert,
  Undo2,
} from 'lucide-react';
import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';

import { errorText, t, useLang, useT, type Key } from '../../i18n';
import type { ChangedFile, CommitSummary, RepositoryStatus } from '../../ipc/aurora-types';
import type { IpcError } from '../../ipc/types';
import { useEditor, viewTabId } from '../../state/editor';
import {
  commit,
  discard,
  init,
  loadBranches,
  loadLog,
  openChange,
  openCommitChange,
  openRemoteForm,
  refreshGit,
  removeRemote,
  repoFile,
  saveRemote,
  selectedFiles,
  setIdentity,
  switchBranch,
  sync,
  syncKind,
  useGit,
} from '../../state/git';
import { useGitHub } from '../../state/github';
import { useProject } from '../../state/project';
import { showError, useToasts } from '../../state/toasts';
import { baseName, dirName } from '../../util/paths';
import { Button, Checkbox, Empty, IconButton, openContextMenu, Spinner, type MenuItem } from '../common';
import { ViewActions } from '../layout/ViewActions';
import { ChangeLetter } from './GitMark';

const GIT_DOWNLOAD = 'https://git-scm.com/downloads';
const GITHUB_NEW = 'https://github.com/new';

/** "há 3 minutos", "ontem", na língua da interface. */
function relativeTime(seconds: number, lang: string): string {
  const diff = seconds - Date.now() / 1000;
  const format = new Intl.RelativeTimeFormat(lang === 'en' ? 'en' : 'pt-BR', { numeric: 'auto' });
  const steps: [Intl.RelativeTimeFormatUnit, number][] = [
    ['second', 60],
    ['minute', 60],
    ['hour', 24],
    ['day', 30],
    ['month', 12],
    ['year', Number.POSITIVE_INFINITY],
  ];
  let value = diff;
  for (const [unit, size] of steps) {
    if (Math.abs(value) < size) return format.format(Math.round(value), unit);
    value /= size;
  }
  return '';
}

function fullDate(seconds: number, lang: string): string {
  return new Date(seconds * 1000).toLocaleString(lang === 'en' ? 'en' : 'pt-BR');
}

/** O caminho de um arquivo como a lista mostra: relativo à pasta do projeto,
 * para os de dentro dela; ao topo do repositório, para os outros. */
function shownPath(repo: RepositoryStatus, file: { path: string; in_project?: boolean }): string {
  const prefix = repo.prefix;
  if (file.in_project !== false && prefix && file.path.startsWith(`${prefix}/`)) return file.path.slice(prefix.length + 1);
  return file.path;
}

/** Uma caixa de marcar sem rótulo à vista, com o "alguns" do cabeçalho. */
function Tick({
  checked,
  mixed,
  label,
  disabled,
  onChange,
}: {
  checked: boolean;
  mixed?: boolean;
  label: string;
  disabled?: boolean;
  onChange: (value: boolean) => void;
}) {
  const ref = useRef<HTMLInputElement>(null);
  useLayoutEffect(() => {
    if (ref.current) ref.current.indeterminate = !!mixed;
  }, [mixed]);
  return (
    <label className={`check git-tick${disabled ? ' is-disabled' : ''}`} onClick={(e) => e.stopPropagation()} onDoubleClick={(e) => e.stopPropagation()}>
      <input ref={ref} type="checkbox" checked={checked} disabled={disabled} aria-label={label} title={label} onChange={(e) => onChange(e.target.checked)} />
      <span className="check__box" aria-hidden />
    </label>
  );
}

// O alto da vista ----------------------------------------------------------

function SyncButton({ repo }: { repo: RepositoryStatus }) {
  const busy = useGit((s) => s.busy);
  const kind = syncKind(repo);
  const { ahead, behind, name } = repo.branch;
  const remote = repo.branch.remote ?? repo.remotes[0]?.name ?? 'origin';
  const running = busy === 'push' || busy === 'pull' || busy === 'fetch' || busy === 'remote';
  const label = running
    ? t(`git.sync.running.${busy}` as Key)
    : kind === 'pull'
      ? t('git.sync.pull', { count: behind })
      : kind === 'push'
        ? t('git.sync.push', { count: ahead })
        : kind === 'fetch'
          ? t('git.sync.fetch')
          : t('git.sync.publish');
  const hint =
    kind === 'pull'
      ? t('git.sync.pullHint', { count: behind, upstream: repo.branch.upstream ?? '' })
      : kind === 'push'
        ? t('git.sync.pushHint', { count: ahead, upstream: repo.branch.upstream ?? '' })
        : kind === 'fetch'
          ? t('git.sync.fetchHint', { remote })
          : kind === 'publish'
            ? t('git.sync.publishHint', { branch: name ?? '', remote })
            : t('git.sync.remoteHint');
  const icon = running ? (
    <Spinner size={14} />
  ) : kind === 'pull' ? (
    <ArrowDown size={14} />
  ) : kind === 'push' ? (
    <ArrowUp size={14} />
  ) : kind === 'fetch' ? (
    <RefreshCw size={14} />
  ) : (
    <CloudUpload size={14} />
  );
  return (
    <button type="button" className="btn btn--sm git-sync" title={hint} disabled={!!busy} onClick={() => void sync()}>
      {icon}
      <span className="git-sync__label">{label}</span>
      {kind === 'pull' && ahead > 0 && !running && <span className="git-sync__extra">↑{ahead}</span>}
    </button>
  );
}

function BranchPicker({ repo, onClose }: { repo: RepositoryStatus; onClose: () => void }) {
  const branches = useGit((s) => s.branches);
  const loading = useGit((s) => s.branchesLoading);
  const blocked = useGit((s) => s.switchBlocked);
  const branchError = useGit((s) => s.branchError);
  const busy = useGit((s) => s.busy);
  const [filter, setFilter] = useState('');
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState('');
  const current = repo.branch.name;

  useEffect(() => {
    useGit.setState({ switchBlocked: null, branchError: null });
    void loadBranches();
  }, []);

  const choose = async (branch: string) => {
    if (branch === current) {
      onClose();
      return;
    }
    if (await switchBranch(branch)) onClose();
  };
  const create = async () => {
    if (!name.trim() || busy) return;
    if (await switchBranch(name.trim(), true)) onClose();
  };
  const escape = (e: KeyboardEvent, then: () => void) => {
    if (e.key !== 'Escape') return;
    e.preventDefault();
    e.stopPropagation();
    then();
  };

  const needle = filter.trim().toLowerCase();
  const local = (branches?.local ?? []).filter((b) => b.name.toLowerCase().includes(needle));
  const remote = (branches?.remote ?? []).filter((b) => b.name.toLowerCase().includes(needle));

  return (
    <div className="git-picker">
      <input
        className="input input--small"
        autoFocus
        placeholder={t('git.branch.filter')}
        aria-label={t('git.branch.filter')}
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        onKeyDown={(e) => escape(e, onClose)}
      />
      {blocked && (
        <div className="git-card git-card--warn">
          <p className="git-card__title">
            <TriangleAlert size={13} /> {t('git.branch.blocked')}
          </p>
          <ul className="git-card__files">
            {blocked.map((file) => (
              <li key={file} className="mono">
                {file}
              </li>
            ))}
          </ul>
          <div className="git-card__actions">
            <Button small onClick={() => useGit.getState().dismissSwitchBlocked()}>
              {t('common.ok')}
            </Button>
          </div>
        </div>
      )}
      <div className="git-picker__lists">
        {!branches && loading && (
          <p className="git-picker__empty">
            <Spinner size={12} /> {t('common.loading')}
          </p>
        )}
        {branches && (
          <>
            <p className="git-picker__label">{t('git.branch.local')}</p>
            {local.length === 0 && <p className="git-picker__empty">{t('git.branch.none')}</p>}
            <ul>
              {local.map((branch) => (
                <li key={branch.name}>
                  <button
                    type="button"
                    className={`git-picker__item${branch.name === current ? ' is-current' : ''}`}
                    disabled={!!busy}
                    title={branch.upstream ? t('git.branch.tracks', { upstream: branch.upstream }) : t('git.branch.unpublished')}
                    onClick={() => void choose(branch.name)}
                  >
                    {branch.name === current ? <Check size={13} /> : <GitBranch size={13} />}
                    <span className="git-picker__name">{branch.name}</span>
                    <span className="git-picker__meta">
                      {branch.gone ? t('git.branch.gone') : `${branch.ahead ? `↑${branch.ahead}` : ''}${branch.behind ? ` ↓${branch.behind}` : ''}`}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
            {remote.length > 0 && (
              <>
                <p className="git-picker__label">{t('git.branch.remote')}</p>
                <ul>
                  {remote.map((branch) => (
                    <li key={branch.name}>
                      <button
                        type="button"
                        className="git-picker__item"
                        disabled={!!busy}
                        title={t('git.branch.fromRemote', { branch: branch.branch })}
                        onClick={() => void choose(branch.name)}
                      >
                        <Download size={13} />
                        <span className="git-picker__name">{branch.name}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              </>
            )}
          </>
        )}
      </div>
      {creating ? (
        <div className="git-picker__create">
          <input
            className="input input--small"
            autoFocus
            placeholder={t('git.branch.newName')}
            aria-label={t('git.branch.newName')}
            aria-invalid={!!branchError}
            value={name}
            onChange={(e) => {
              setName(e.target.value);
              if (useGit.getState().branchError) useGit.setState({ branchError: null });
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void create();
              escape(e, () => setCreating(false));
            }}
          />
          {branchError && <span className="field__error">{errorText(branchError)}</span>}
          <div className="git-card__actions">
            <Button small onClick={() => setCreating(false)}>
              {t('common.cancel')}
            </Button>
            <Button small variant="primary" disabled={!name.trim() || !!busy} onClick={() => void create()}>
              {busy === 'switch' && <Spinner size={12} />}
              {t('git.branch.createAndSwitch')}
            </Button>
          </div>
        </div>
      ) : (
        <div className="git-picker__footer">
          <Button
            small
            icon={<GitBranchPlus size={13} />}
            onClick={() => {
              setCreating(true);
              setName(filter.trim());
              useGit.setState({ branchError: null });
            }}
          >
            {t('git.branch.create')}
          </Button>
          <Button small variant="ghost" onClick={onClose}>
            {t('common.close')}
          </Button>
        </div>
      )}
    </div>
  );
}

/** O endereço de um remoto como a vista mostra: sem o protocolo e sem o
 * `.git` do fim (`github.com/ana/projeto`). */
function shortUrl(url: string): string {
  return url
    .replace(/^[a-z+]+:\/\//i, '')
    .replace(/^git@([^:]+):/, '$1/')
    .replace(/\.git$/, '');
}

function RemoteLine({ repo }: { repo: RepositoryStatus }) {
  const busy = useGit((s) => s.busy);
  const open = useGit((s) => !!s.remoteForm);
  const remote = repo.remotes.find((r) => r.name === repo.branch.remote) ?? repo.remotes[0];
  if (!remote || open) return null;
  return (
    <button
      type="button"
      className="git-remote"
      title={t('git.remote.editHint', { name: remote.name, url: remote.url })}
      disabled={!!busy}
      onClick={() => openRemoteForm(remote.name)}
    >
      <span className="git-remote__name">{remote.name}</span>
      <span className="git-remote__url">{shortUrl(remote.url)}</span>
      <Pencil size={11} className="git-remote__edit" />
    </button>
  );
}

function RemoteForm() {
  const form = useGit((s) => s.remoteForm);
  const busy = useGit((s) => s.busy);
  const [url, setUrl] = useState(form?.url ?? '');
  useEffect(() => setUrl(form?.url ?? ''), [form?.name, form?.url, form?.error]);
  if (!form) return null;
  const typed = url.trim();
  const canSave = typed !== '' && (typed !== form.url || !!form.retry);
  const save = () => {
    if (canSave && !busy) void saveRemote(url);
  };
  const close = () => useGit.getState().closeRemoteForm();
  const saveLabel = !form.exists
    ? t('git.remote.publish')
    : form.retry
      ? t('git.remote.saveRetry')
      : t('git.remote.save');
  const stderr = typeof form.error?.data?.stderr === 'string' ? form.error.data.stderr : null;
  return (
    <div className={`git-card${form.error ? ' git-card--warn' : ''}`}>
      <p className="git-card__title">
        {form.error && <TriangleAlert size={13} />}{' '}
        {form.exists ? t('git.remote.editTitle', { name: form.name }) : t('git.remote.title')}
      </p>
      <p className="git-card__hint">
        {form.error
          ? t('git.remote.unreachableHint', { url: form.url })
          : form.exists
            ? t('git.remote.editText')
            : t('git.remote.hint')}
      </p>
      <input
        className="input input--small"
        autoFocus
        placeholder="https://github.com/usuario/projeto.git"
        aria-label={t('git.remote.url')}
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        onFocus={(e) => e.currentTarget.select()}
        onKeyDown={(e) => {
          if (e.key === 'Enter') save();
          if (e.key === 'Escape') close();
        }}
      />
      {stderr && (
        <details className="git-card__details">
          <summary>{t('git.remote.gitSaid')}</summary>
          <pre className="mono">{stderr}</pre>
        </details>
      )}
      <div className="git-card__actions">
        {form.exists ? (
          <Button small variant="ghost" disabled={!!busy} onClick={() => void removeRemote()}>
            {t('git.remote.remove')}
          </Button>
        ) : (
          <Button small variant="ghost" onClick={() => void openUrl(GITHUB_NEW).catch(showError)}>
            {t('git.remote.createOnGitHub')}
          </Button>
        )}
        <Button small onClick={close}>
          {t('common.cancel')}
        </Button>
        <Button small variant="primary" disabled={!canSave || !!busy} onClick={save}>
          {busy === 'remote' && <Spinner size={12} />}
          {saveLabel}
        </Button>
      </div>
    </div>
  );
}

function OperationBanner({ repo }: { repo: RepositoryStatus }) {
  const conflicted = repo.files.filter((file) => file.change === 'conflicted');
  if (!repo.operation) return null;
  return (
    <div className="git-card git-card--warn">
      <p className="git-card__title">
        <TriangleAlert size={13} /> {t(`git.operation.${repo.operation}` as Key)}
      </p>
      <p className="git-card__hint">{conflicted.length ? t('git.operation.conflicts') : t('git.operation.hint')}</p>
      {conflicted.length > 0 && (
        <ul className="git-card__files">
          {conflicted.map((file) => (
            <li key={file.path}>
              <button type="button" className="link mono" onClick={() => openChange(file, false)}>
                {shownPath(repo, file)}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function Head({ repo }: { repo: RepositoryStatus }) {
  const [picker, setPicker] = useState(false);
  const busy = useGit((s) => s.busy);
  const name = baseName(repo.top);
  const branch = repo.branch.name;
  return (
    <div className="git-head">
      <div className="git-head__repo" title={repo.prefix ? `${repo.top}\n${t('git.head.prefix', { prefix: repo.prefix })}` : repo.top}>
        <FolderGit2 size={14} />
        <span className="git-head__name">{name}</span>
        {repo.prefix && <span className="git-head__prefix">/{repo.prefix}</span>}
      </div>
      <div className="git-head__row">
        <button
          type="button"
          className={`btn btn--sm git-branch${picker ? ' is-open' : ''}`}
          aria-expanded={picker}
          title={branch ? t('git.branch.current', { branch }) : t('git.branch.detached')}
          disabled={!!busy && busy !== 'switch'}
          onClick={() => setPicker(!picker)}
        >
          <GitBranch size={14} />
          <span className="git-branch__name">{branch ?? `(${repo.branch.commit?.slice(0, 7) ?? t('git.branch.noCommit')})`}</span>
          {busy === 'switch' ? <Spinner size={12} /> : <ChevronDown size={13} className="git-branch__chevron" />}
        </button>
        <SyncButton repo={repo} />
      </div>
      <RemoteLine repo={repo} />
      {picker && <BranchPicker repo={repo} onClose={() => setPicker(false)} />}
      <RemoteForm />
      <OperationBanner repo={repo} />
    </div>
  );
}

// Mudanças -----------------------------------------------------------------

function changeMenu(repo: RepositoryStatus, file: ChangedFile): MenuItem[] {
  const abs = repoFile(repo, file.path);
  const exists = file.change !== 'deleted';
  return [
    { label: t('git.openDiff'), run: () => openChange(file, false) },
    { label: t('git.openFile'), disabled: !exists, run: () => void useEditor.getState().openFile(abs) },
    { separator: true },
    { label: t('git.discard.action'), danger: true, run: () => void discard(file) },
    { separator: true },
    { label: t('common.copyPath'), run: () => void navigator.clipboard.writeText(abs) },
    { label: t('git.copyRepoPath'), run: () => void navigator.clipboard.writeText(file.path) },
    { label: t('common.reveal'), disabled: !exists, run: () => void revealItemInDir(abs).catch(showError) },
  ];
}

function ChangeRow({ repo, file }: { repo: RepositoryStatus; file: ChangedFile }) {
  const checked = useGit((s) => !s.excluded.has(file.path));
  const busy = useGit((s) => s.busy);
  const selected = useEditor((s) => s.activeId === viewTabId('diff', { rev: '', repo: file.path }));
  const shown = shownPath(repo, file);
  const dir = dirName(shown);
  const name = baseName(shown);
  const renamed = file.original ? t('git.renamedFrom', { path: file.original }) : '';
  return (
    <li
      className={`git-file git-file--${file.change}${selected ? ' is-selected' : ''}`}
      role="button"
      tabIndex={0}
      title={[file.path, renamed, t(`git.change.${file.change}` as Key)].filter(Boolean).join('\n')}
      onClick={() => openChange(file)}
      onDoubleClick={() => openChange(file, false)}
      onKeyDown={(e) => {
        if (e.key === 'Enter') openChange(file, false);
        if (e.key === ' ') {
          e.preventDefault();
          useGit.getState().setExcluded([file.path], checked);
        }
      }}
      onContextMenu={(e) => openContextMenu(e, changeMenu(repo, file))}
    >
      <Tick checked={checked} label={t('git.includeFile', { name })} onChange={(value) => useGit.getState().setExcluded([file.path], !value)} />
      <span className="git-file__name">{name}</span>
      {dir && dir !== shown && <span className="git-file__dir">{dir}</span>}
      <span className="git-file__actions">
        {file.change !== 'deleted' && (
          <IconButton
            className="icon-btn--small"
            label={t('git.openFile')}
            onClick={(e) => {
              e.stopPropagation();
              void useEditor.getState().openFile(repoFile(repo, file.path));
            }}
          >
            <FileText size={13} />
          </IconButton>
        )}
        <IconButton
          className="icon-btn--small"
          label={t('git.discard.action')}
          disabled={!!busy}
          onClick={(e) => {
            e.stopPropagation();
            void discard(file);
          }}
        >
          <Undo2 size={13} />
        </IconButton>
      </span>
      <ChangeLetter change={file.change} />
    </li>
  );
}

/** A caixa do cabeçalho de uma lista: marca ou desmarca todos dela. */
function GroupTick({ files, label }: { files: ChangedFile[]; label: string }) {
  const excluded = useGit((s) => s.excluded);
  const off = files.filter((file) => excluded.has(file.path)).length;
  return (
    <Tick
      checked={off === 0 && files.length > 0}
      mixed={off > 0 && off < files.length}
      label={label}
      disabled={files.length === 0}
      onChange={(value) =>
        useGit.getState().setExcluded(
          files.map((file) => file.path),
          !value,
        )
      }
    />
  );
}

function OutsideFiles({ repo, files, open: initial }: { repo: RepositoryStatus; files: ChangedFile[]; open: boolean }) {
  const [open, setOpen] = useState(initial);
  const excluded = useGit((s) => s.excluded);
  const chosen = files.filter((file) => !excluded.has(file.path)).length;
  return (
    <div className="git-outside">
      <div className="git-list__header">
        <GroupTick files={files} label={t('git.outside.all')} />
        <button type="button" className="git-outside__toggle" aria-expanded={open} onClick={() => setOpen(!open)}>
          {open ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
          <span>{t('git.outside.title')}</span>
          <span className="git-list__count">{t('git.selectedCount', { chosen, total: files.length })}</span>
        </button>
      </div>
      {open && (
        <>
          <p className="git-outside__hint">{t('git.outside.hint')}</p>
          <ul className="git-list">
            {files.map((file) => (
              <ChangeRow key={file.path} repo={repo} file={file} />
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

function IdentityForm() {
  const account = useGitHub((s) => s.account);
  const busy = useGit((s) => s.busy);
  const [name, setName] = useState('');
  const [email, setEmail] = useState('');
  const [global, setGlobal] = useState(true);
  const valid = !!name.trim() && email.includes('@');
  return (
    <div className="git-card">
      <p className="git-card__title">{t('git.identity.title')}</p>
      <p className="git-card__hint">{t('git.identity.hint')}</p>
      <input className="input input--small" placeholder={t('git.identity.name')} aria-label={t('git.identity.name')} value={name} onChange={(e) => setName(e.target.value)} />
      <input
        className="input input--small"
        type="email"
        placeholder={t('git.identity.email')}
        aria-label={t('git.identity.email')}
        value={email}
        onChange={(e) => setEmail(e.target.value)}
      />
      <Checkbox checked={global} onChange={setGlobal} label={t('git.identity.global')} />
      <div className="git-card__actions">
        {account && (
          <Button
            small
            variant="ghost"
            onClick={() => {
              setName(account.name || account.login);
              setEmail(account.commit_email);
            }}
          >
            {t('git.identity.useGitHub')}
          </Button>
        )}
        <Button small variant="primary" disabled={!valid || !!busy} onClick={() => void setIdentity(name, email, global)}>
          {busy === 'identity' && <Spinner size={12} />}
          {t('common.save')}
        </Button>
      </div>
    </div>
  );
}

function CommitBox({ repo }: { repo: RepositoryStatus }) {
  const summary = useGit((s) => s.summary);
  const description = useGit((s) => s.description);
  const busy = useGit((s) => s.busy);
  const count = useGit((s) => selectedFiles(s).length);
  const branch = repo.branch.name;
  const identity = repo.identity;
  if (!identity) {
    return (
      <div className="git-commit">
        <IdentityForm />
      </div>
    );
  }
  if (repo.files.length === 0) return null;
  const ready = !!summary.trim() && count > 0 && !busy;
  const keys = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      if (ready) void commit();
    }
  };
  const label = branch ? t('git.commit.button', { branch }) : t('git.commit.buttonDetached');
  return (
    <div className="git-commit">
      <input
        className="input input--small"
        placeholder={t('git.commit.summary')}
        aria-label={t('git.commit.summary')}
        value={summary}
        onChange={(e) => useGit.getState().setDraft({ summary: e.target.value })}
        onKeyDown={keys}
      />
      <textarea
        className="input git-commit__description"
        rows={3}
        placeholder={t('git.commit.description')}
        aria-label={t('git.commit.description')}
        value={description}
        onChange={(e) => useGit.getState().setDraft({ description: e.target.value })}
        onKeyDown={keys}
      />
      <button
        type="button"
        className="btn btn--primary git-commit__button"
        disabled={!ready}
        title={count === 0 ? t('git.commit.noneSelected') : !summary.trim() ? t('git.commit.needSummary') : `${label} (Ctrl+Enter)`}
        onClick={() => void commit()}
      >
        {busy === 'commit' && <Spinner size={13} />}
        <span className="git-commit__label">{label}</span>
      </button>
    </div>
  );
}

function Changes({ repo }: { repo: RepositoryStatus }) {
  const inside = repo.files.filter((file) => file.in_project);
  const outside = repo.files.filter((file) => !file.in_project);
  const chosen = useGit((s) => selectedFiles(s).length);
  return (
    <>
      <div className="sidebar__body git-changes">
        {repo.files.length === 0 ? (
          <Empty icon={<Check size={20} />}>
            <p>{t('git.noChanges')}</p>
            <p className="git-muted">{repo.branch.commit ? t('git.noChangesHint') : t('git.firstCommitHint')}</p>
          </Empty>
        ) : (
          <>
            <div className="git-list__header">
              <GroupTick files={inside} label={t('git.selectAll')} />
              <span className="git-list__title">{t('git.changedCount', { count: inside.length })}</span>
              <span className="git-list__count">{t('git.selectedCount', { chosen, total: repo.files.length })}</span>
            </div>
            {inside.length === 0 && <p className="git-muted git-pad">{t('git.noChangesInProject')}</p>}
            <ul className="git-list">
              {inside.map((file) => (
                <ChangeRow key={file.path} repo={repo} file={file} />
              ))}
            </ul>
            {outside.length > 0 && <OutsideFiles repo={repo} files={outside} open={inside.length === 0} />}
            {repo.truncated && <p className="git-card git-card--warn git-pad">{t('git.truncated', { count: repo.files.length })}</p>}
          </>
        )}
      </div>
      <CommitBox repo={repo} />
    </>
  );
}

// Histórico ----------------------------------------------------------------

function CommitItem({ item }: { item: CommitSummary }) {
  const lang = useLang((s) => s.lang);
  const open = useGit((s) => s.expanded.has(item.oid));
  const files = useGit((s) => s.commitFiles[item.oid]);
  const repo = useGit((s) => s.status?.repository ?? null);
  const menu = (e: React.MouseEvent) =>
    openContextMenu(e, [
      { label: t('git.history.copyId'), run: () => void navigator.clipboard.writeText(item.oid) },
      { label: t('git.history.copySummary'), run: () => void navigator.clipboard.writeText(item.summary) },
    ]);
  const title = [item.summary, item.body, '', `${item.short} · ${item.author} <${item.email}>`, fullDate(item.time, lang)]
    .filter((line, index) => index !== 1 || line)
    .join('\n');
  return (
    <li className="git-log__item">
      <button
        type="button"
        className={`git-log__row${open ? ' is-open' : ''}`}
        aria-expanded={open}
        title={title}
        onClick={() => useGit.getState().toggleCommit(item.oid)}
        onContextMenu={menu}
      >
        {open ? <ChevronDown size={13} className="git-log__chevron" /> : <ChevronRight size={13} className="git-log__chevron" />}
        <span className="git-log__main">
          <span className="git-log__summary">{item.summary || t('git.history.noMessage')}</span>
          <span className="git-log__meta">
            {item.author} · {relativeTime(item.time, lang)}
            {item.local && (
              <span className="git-log__local" title={t('git.history.localHint')}>
                <ArrowUp size={11} />
                {t('git.history.local')}
              </span>
            )}
          </span>
        </span>
      </button>
      {open &&
        (files ? (
          <ul className="git-list git-list--nested">
            {files.files.map((file) => {
              const shown = repo ? shownPath(repo, file) : file.path;
              const dir = dirName(shown);
              return (
                <li
                  key={file.path}
                  className={`git-file git-file--${file.change}`}
                  role="button"
                  tabIndex={0}
                  title={file.original ? `${file.path}\n${t('git.renamedFrom', { path: file.original })}` : file.path}
                  onClick={() => openCommitChange(item, files.parent, file)}
                  onDoubleClick={() => openCommitChange(item, files.parent, file, false)}
                  onKeyDown={(e) => e.key === 'Enter' && openCommitChange(item, files.parent, file, false)}
                >
                  <span className="git-file__name">{baseName(shown)}</span>
                  {dir && dir !== shown && <span className="git-file__dir">{dir}</span>}
                  <ChangeLetter change={file.change} />
                </li>
              );
            })}
            {files.files.length === 0 && <li className="git-muted git-pad">{t('git.history.noFiles')}</li>}
          </ul>
        ) : (
          <p className="git-muted git-pad">
            <Spinner size={12} /> {t('common.loading')}
          </p>
        ))}
    </li>
  );
}

function History() {
  const log = useGit((s) => s.log);
  const loading = useGit((s) => s.logLoading);
  const error = useGit((s) => s.logError);

  useEffect(() => {
    const state = useGit.getState();
    if (!state.log && !state.logLoading) void loadLog(true);
  }, []);

  if (!log) {
    return (
      <div className="sidebar__body">
        <Empty>
          {error ? (
            <>
              <p>{errorText(error)}</p>
              <Button small onClick={() => void loadLog(true)}>
                {t('git.retry')}
              </Button>
            </>
          ) : (
            <Spinner />
          )}
        </Empty>
      </div>
    );
  }
  return (
    <div className="sidebar__body">
      {log.commits.length === 0 ? (
        <Empty>{t('git.history.empty')}</Empty>
      ) : (
        <ul className="git-log">
          {log.commits.map((item) => (
            <CommitItem key={item.oid} item={item} />
          ))}
        </ul>
      )}
      {log.more && (
        <div className="git-pad git-more">
          <Button small disabled={loading} onClick={() => void loadLog(false)}>
            {loading && <Spinner size={12} />}
            {t('git.history.more')}
          </Button>
        </div>
      )}
    </div>
  );
}

// Estados sem repositório --------------------------------------------------

function ErrorState({ error }: { error: IpcError }) {
  const path = typeof error.data?.path === 'string' ? error.data.path : null;
  let body: ReactNode;
  switch (error.code) {
    case 'git_missing':
      body = (
        <>
          <p>{t('git.missing.title')}</p>
          <p className="git-muted">{t('git.missing.hint')}</p>
          <Button variant="primary" icon={<Download size={14} />} onClick={() => void openUrl(GIT_DOWNLOAD).catch(showError)}>
            {t('git.missing.download')}
          </Button>
        </>
      );
      break;
    case 'git_dubious_ownership': {
      const command = `git config --global --add safe.directory "${path ?? ''}"`;
      body = (
        <>
          <p>{errorText(error)}</p>
          <p className="git-muted">{t('git.dubious.hint')}</p>
          <code className="git-command">{command}</code>
          <Button
            small
            icon={<Copy size={13} />}
            onClick={() =>
              void navigator.clipboard.writeText(command).then(() => useToasts.getState().push({ kind: 'info', title: t('common.copied') }))
            }
          >
            {t('git.copyCommand')}
          </Button>
        </>
      );
      break;
    }
    default:
      body = (
        <>
          <p>{errorText(error)}</p>
          {errorText(error) !== error.message && <p className="git-muted git-detail">{error.message}</p>}
        </>
      );
  }
  return (
    <div className="sidebar__body">
      <Empty icon={<TriangleAlert size={20} />}>
        {body}
        <Button small icon={<RefreshCw size={13} />} onClick={() => void refreshGit()}>
          {t('git.retry')}
        </Button>
      </Empty>
    </div>
  );
}

function NoRepository() {
  const busy = useGit((s) => s.busy);
  return (
    <div className="sidebar__body">
      <Empty icon={<FolderGit2 size={22} />}>
        <p>{t('git.noRepo.title')}</p>
        <p className="git-muted">{t('git.noRepo.hint')}</p>
        <Button variant="primary" disabled={!!busy} onClick={() => void init()}>
          {busy === 'init' && <Spinner size={13} />}
          {t('git.noRepo.create')}
        </Button>
      </Empty>
    </div>
  );
}

// A vista --------------------------------------------------------------------

export function GitPanel() {
  useT();
  const snapshot = useProject((s) => s.snapshot);
  const status = useGit((s) => s.status);
  const error = useGit((s) => s.error);
  const loaded = useGit((s) => s.loaded);
  const loading = useGit((s) => s.loading);
  const tab = useGit((s) => s.tab);
  const count = useGit((s) => s.status?.repository?.files.length ?? 0);

  // Abrir a vista relê: o que mudou por fora sem o vigia ver (um repositório
  // acima da pasta do projeto) aparece.
  useEffect(() => {
    void refreshGit();
  }, []);

  const actions = (
    <ViewActions>
      <IconButton label={t('common.refresh')} disabled={loading} onClick={() => void refreshGit()}>
        {loading ? <Spinner size={13} /> : <RefreshCw size={14} />}
      </IconButton>
    </ViewActions>
  );

  if (!snapshot) {
    return (
      <div className="sidebar__body">
        <Empty icon={<FolderGit2 size={22} />}>{t('git.noProject')}</Empty>
      </div>
    );
  }
  if (error) {
    return (
      <>
        {actions}
        <ErrorState error={error} />
      </>
    );
  }
  if (!status) {
    return (
      <>
        {actions}
        <div className="sidebar__body">
          <Empty>{loaded ? <p>{t('git.unavailable')}</p> : <Spinner />}</Empty>
        </div>
      </>
    );
  }
  const repo = status.repository;
  if (!repo) {
    return (
      <>
        {actions}
        <NoRepository />
      </>
    );
  }
  return (
    <>
      {actions}
      <Head repo={repo} />
      <div className="sidebar__toolbar">
        <div className="segmented segmented--full" role="tablist">
          <button
            type="button"
            role="tab"
            aria-selected={tab === 'changes'}
            className={tab === 'changes' ? 'is-active' : ''}
            onClick={() => useGit.getState().setTab('changes')}
          >
            {t('git.tab.changes')}
            {count > 0 && <span className="git-tab__count">{repo.truncated ? `${count}+` : count}</span>}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={tab === 'history'}
            className={tab === 'history' ? 'is-active' : ''}
            onClick={() => useGit.getState().setTab('history')}
          >
            {t('git.tab.history')}
          </button>
        </div>
      </div>
      {tab === 'changes' ? <Changes repo={repo} /> : <History />}
    </>
  );
}
