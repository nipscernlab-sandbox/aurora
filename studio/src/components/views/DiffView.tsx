// A aba de diff do controle de versão: o editor de diff do Monaco com duas
// versões de um arquivo.
//
// - Mudanças do disco: à esquerda, o arquivo no último commit (HEAD); à
//   direita, o próprio arquivo, com o mesmo modelo do editor (state/editor.ts):
//   editar aqui edita o arquivo, e Ctrl+S grava. As setas da margem desfazem
//   um trecho.
// - Um commit do histórico: o arquivo no pai e no commit, só leitura.
//
// O lado que vem do git mora num modelo com esquema próprio (`aurora-git:`),
// para não colidir com o modelo do arquivo (`file:`) nem receber os
// marcadores dos diagnósticos (editor/monaco.ts). Um arquivo binário ou
// grande demais aparece como aviso, sem diff.

import { Columns2, FileText, RefreshCw, Rows2 } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';

import { getModel, holdModel, languageForPath, monaco, setMonacoTheme } from '../../editor/monaco';
import { errorText, useT, type Key } from '../../i18n';
import { api } from '../../ipc/api';
import type { FileVersion, Revision } from '../../ipc/aurora-types';
import type { IpcError } from '../../ipc/types';
import { useApp } from '../../state/app';
import { diffSpecOf, useEditor, type Tab } from '../../state/editor';
import { useGit } from '../../state/git';
import { learnSession } from '../../state/learnSession';
import { Button, Empty, IconButton, Spinner } from '../common';

const MISSING: FileVersion = { exists: false, size: 0, binary: false, too_large: false, text: null };

/** Lado a lado ou em linha, lembrado para as próximas abas de diff. */
const INLINE_KEY = 'aurora-ide:diff-inline';

function loadInline(): boolean {
  try {
    return localStorage.getItem(INLINE_KEY) === '1';
  } catch {
    return false;
  }
}

function saveInline(value: boolean): void {
  try {
    localStorage.setItem(INLINE_KEY, value ? '1' : '0');
  } catch {
    // Só não lembra.
  }
}

let modelCount = 0;

/** Uma URI nova para um lado que vem do git: duas abas do mesmo arquivo não
 * dividem modelo. A extensão fica no fim, para o Monaco achar a linguagem. */
function gitUri(path: string): monaco.Uri {
  return monaco.Uri.from({ scheme: 'aurora-git', path: `/${++modelCount}/${path}` });
}

interface Sides {
  original: FileVersion;
  /** `null`: o lado novo é o arquivo no disco (o modelo do editor). */
  modified: FileVersion | null;
}

function setText(model: monaco.editor.ITextModel, text: string): void {
  if (model.getValue() !== text) model.setValue(text);
}

export function DiffView({ tab }: { tab: Tab }) {
  const t = useT();
  const spec = diffSpecOf(tab);
  const worktree = !spec.rev;
  /** O arquivo do lado novo, editável; só nas mudanças do disco de um
   * arquivo que existe. */
  const path = worktree ? (tab.path ?? null) : null;
  const doc = useEditor((s) => (path ? s.docs[path] : undefined));
  const head = useGit((s) => s.status?.repository?.branch.commit ?? null);
  // Nas mudanças do disco, a mudança de agora (um commit feito depois de
  // abrir a aba a tira da lista); num commit, a da aba.
  const current = useGit((s) => (worktree ? (s.status?.repository?.files.find((f) => f.path === spec.repo) ?? null) : null));
  const change = worktree ? (current?.change ?? null) : spec.change;
  const before = worktree ? (current?.original ?? spec.repo) : (spec.original ?? spec.repo);
  const theme = useApp((s) => s.theme);
  const settings = useApp((s) => s.settings?.editor);
  const [inline, setInline] = useState(loadInline);
  const [sides, setSides] = useState<Sides | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [editor, setEditor] = useState<monaco.editor.IStandaloneDiffEditor | null>(null);
  const [attached, setAttached] = useState(0);
  const container = useRef<HTMLDivElement>(null);
  const live = useRef<monaco.editor.IStandaloneDiffEditor | null>(null);
  const models = useRef<{ original: monaco.editor.ITextModel; modified: monaco.editor.ITextModel } | null>(null);
  const revealed = useRef(false);
  const editable = useRef(path);
  editable.current = path;

  // As duas versões. Nas mudanças do disco, o lado antigo é relido quando o
  // último commit muda (um commit feito agora, uma troca de ramo).
  useEffect(() => {
    let alive = true;
    setError(null);
    const read = (revision: Revision, file: string) => api.git.readFile(revision, file);
    const load = async (): Promise<Sides> => {
      if (worktree) {
        const original = await read({ kind: 'head' }, before);
        return { original, modified: path ? null : MISSING };
      }
      const original = spec.parent ? await read({ kind: 'commit', oid: spec.parent }, before) : MISSING;
      const modified = spec.change === 'deleted' ? MISSING : await read({ kind: 'commit', oid: spec.rev! }, spec.repo);
      return { original, modified };
    };
    load().then(
      (loaded) => alive && setSides(loaded),
      (failure: IpcError) => alive && setError(failure),
    );
    return () => {
      alive = false;
    };
  }, [spec.repo, spec.rev, spec.parent, before, spec.change, path, worktree ? head : null, attempt]);

  const notice: Key | null = !sides
    ? null
    : sides.original.too_large || sides.modified?.too_large || doc?.tooLarge
      ? 'git.diff.tooLarge'
      : sides.original.binary || sides.modified?.binary || doc?.binary
        ? 'git.diff.binary'
        : null;
  const ready = sides !== null && notice === null && !error;

  // Os modelos, ligados ao editor. Este efeito vem antes do que cria o
  // editor: ao fechar a aba, os modelos saem do editor antes de ele sumir.
  useEffect(() => {
    // Um editor que acabou de sair (a aba trocou de arquivo) não recebe nada.
    if (!editor || live.current !== editor) return;
    const language = languageForPath(spec.repo);
    const original = monaco.editor.createModel('', language, gitUri(spec.repo));
    const own = [original];
    let modified = path ? getModel(path) : null;
    if (!modified) {
      modified = monaco.editor.createModel('', language, gitUri(spec.repo));
      own.push(modified);
    }
    editor.setModel({ original, modified });
    models.current = { original, modified };
    revealed.current = false;
    setAttached((n) => n + 1);
    // O modelo do arquivo pode sair antes desta aba (o editor o descarta
    // quando a última aba que o usa fecha): o diff larga dele antes.
    const release = path
      ? holdModel(path, () => {
          if (live.current === editor) editor.setModel(null);
          models.current = null;
        })
      : null;
    return () => {
      release?.();
      if (live.current === editor) editor.setModel(null);
      for (const model of own) model.dispose();
      models.current = null;
    };
  }, [editor, path, spec.repo]);

  // O editor de diff, criado quando há o que comparar.
  useEffect(() => {
    if (!ready || !container.current) return;
    const created = monaco.editor.createDiffEditor(container.current, {
      automaticLayout: true,
      fontFamily: "'JetBrains Mono Variable', 'JetBrains Mono', ui-monospace, monospace",
      fontLigatures: false,
      originalEditable: false,
      readOnly: true,
      renderSideBySide: !loadInline(),
      useInlineViewWhenSpaceIsLimited: true,
      ignoreTrimWhitespace: false,
      scrollBeyondLastLine: false,
      minimap: { enabled: false },
      fixedOverflowWidgets: true,
      padding: { top: 8 },
    });
    live.current = created;
    setEditor(created);
    // Gravar sozinho ao sair do editor, como no MonacoHost.
    const blur = created.getModifiedEditor().onDidBlurEditorText(() => {
      if (editable.current && useApp.getState().settings?.editor.auto_save && !learnSession.active()) {
        void useEditor.getState().save(tab.id);
      }
    });
    return () => {
      blur.dispose();
      created.setModel(null);
      created.dispose();
      if (live.current === created) live.current = null;
      setEditor(null);
    };
  }, [ready, tab.id]);

  // Os textos que vêm do git, nos modelos.
  useEffect(() => {
    const current = models.current;
    if (!current || !sides || !editor) return;
    setText(current.original, sides.original.text ?? '');
    if (sides.modified) setText(current.modified, sides.modified.text ?? '');
    if (!revealed.current) {
      revealed.current = true;
      const once = editor.onDidUpdateDiff(() => {
        once.dispose();
        editor.revealFirstDiff();
      });
    }
  }, [sides, attached, editor]);

  useEffect(() => {
    // Só o arquivo do disco se edita; num commit, os dois lados são só
    // leitura, sem as setas de desfazer da margem.
    editor?.updateOptions({
      readOnly: !path,
      renderMarginRevertIcon: !!path,
      renderGutterMenu: !!path,
      renderSideBySide: !inline,
      fontSize: settings?.font_size,
      wordWrap: settings?.word_wrap ? 'on' : 'off',
    });
  }, [editor, inline, settings, path]);

  useEffect(() => setMonacoTheme(theme), [theme]);

  const toggleInline = () => {
    saveInline(!inline);
    setInline(!inline);
  };

  const renamed = before !== spec.repo ? `${before} → ` : '';
  const caption = worktree ? t('git.diff.captionChanges') : t('git.diff.captionCommit', { short: spec.short ?? spec.rev?.slice(0, 7) ?? '' });

  return (
    <div className="diff-view">
      <div className="diff-view__header">
        <span className="diff-view__path mono" title={`${renamed}${spec.repo}`}>
          {renamed}
          {spec.repo}
        </span>
        <span className={`git-change git-change--${change ?? 'none'}`}>{change ? t(`git.change.${change}` as Key) : t('git.diff.unchanged')}</span>
        <span className="diff-view__caption">{caption}</span>
        <span className="diff-view__actions">
          <IconButton label={inline ? t('git.diff.sideBySide') : t('git.diff.inline')} onClick={toggleInline}>
            {inline ? <Columns2 size={14} /> : <Rows2 size={14} />}
          </IconButton>
          {path && (
            <IconButton label={t('git.openFile')} onClick={() => void useEditor.getState().openFile(path)}>
              <FileText size={14} />
            </IconButton>
          )}
        </span>
      </div>
      <div className="diff-view__body">
        {error ? (
          <Empty>
            <p>{errorText(error)}</p>
            <Button icon={<RefreshCw size={14} />} onClick={() => setAttempt((n) => n + 1)}>
              {t('git.retry')}
            </Button>
          </Empty>
        ) : !sides ? (
          <Empty>
            <Spinner />
          </Empty>
        ) : notice ? (
          <Empty>
            <p>{t(notice)}</p>
            {path && (
              <Button icon={<FileText size={14} />} onClick={() => void useEditor.getState().openFile(path)}>
                {t('git.openFile')}
              </Button>
            )}
          </Empty>
        ) : (
          <div className="diff-view__editor" ref={container} />
        )}
      </div>
    </div>
  );
}
