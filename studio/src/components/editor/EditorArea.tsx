// A área central: até três grupos lado a lado, cada um com a
// sua barra de abas e o conteúdo da aba ativa. O editor de texto de um grupo
// fica montado enquanto o grupo tiver aba de arquivo, só escondido quando a
// aba ativa é uma vista, para trocar de aba sem recriar o Monaco.
//
// As abas se arrastam com o ponteiro (como a árvore, sidebar/dnd.ts): numa
// barra de abas, para a posição; no conteúdo de um grupo, para o fim dele;
// na faixa da direita do conteúdo, para um grupo novo, se couber.

import { Columns2, X } from 'lucide-react';
import { Fragment, useEffect, useRef } from 'react';
import { Group, Panel, Separator } from 'react-resizable-panels';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { create } from 'zustand';

import { setEditorVisible } from '../../editor/host';
import { t, useT } from '../../i18n';
import { useApp } from '../../state/app';
import { fileTabId, MAX_GROUPS, useEditor, type EditorGroup, type Tab } from '../../state/editor';
import { useLayout } from '../../state/layout';
import { useProject } from '../../state/project';
import { showError } from '../../state/toasts';
import { baseName, relativeTo } from '../../util/paths';
import { IconButton, openContextMenu, type MenuItem } from '../common';
import { chromeMenu } from '../layout/layoutMenus';
import { RunControls } from '../layout/RunControls';
import { AboutView } from '../views/AboutView';
import { CompareView } from '../views/CompareView';
import { DiffView } from '../views/DiffView';
import { ProcessorView } from '../views/ProcessorView';
import { ReportView } from '../views/ReportView';
import { SchematicView } from '../views/SchematicView';
import { SettingsView } from '../views/SettingsView';
import { BoardView } from '../views/BoardView';
import { VirtualBoardView } from '../views/VirtualBoardView';
import { SynthesisView } from '../views/SynthesisView';
import { ToolchainView } from '../views/ToolchainView';
import { Chat } from '../assistant/Chat';
import { LearnView } from '../views/LearnView';
import { WaveView } from '../views/WaveView';
import { WelcomeView } from '../views/WelcomeView';
import { MonacoHost } from './MonacoHost';
import { tabIcon } from './tabIcons';

/** O título de uma aba, no idioma atual. */
export function tabTitle(tab: Tab): string {
  switch (tab.kind) {
    case 'file':
      return baseName(tab.path ?? '');
    case 'settings':
      return t('settings.title');
    case 'toolchain':
      return t('toolchain.title');
    case 'schematic':
      return t('schematic.title');
    case 'synthesis':
      return t('synthesis.title');
    case 'board':
      return t('board.title');
    case 'vboard':
      return t('vboard.title');
    case 'report':
      return tab.data?.id ?? t('reports.title');
    case 'compare':
      return t('compare.title', { current: tab.data?.id ?? '', baseline: tab.data?.against ?? '' }).trim();
    case 'processor':
      return t('processor.title', { name: tab.data?.name ?? '' });
    case 'about':
      return t('about.tab');
    case 'wave':
      return baseName(tab.data?.path ?? '');
    case 'learn':
      return t('learn.tab');
    case 'assistant':
      return t('assistant.tab');
    case 'diff':
      return tab.data?.rev
        ? t('git.diff.tabCommit', { name: baseName(tab.data?.repo ?? ''), short: tab.data?.short ?? tab.data.rev.slice(0, 7) })
        : t('git.diff.tabChanges', { name: baseName(tab.data?.repo ?? '') });
  }
}

// Arrastar abas -------------------------------------------------------------

/** Onde a aba arrastada cai: numa posição da barra de um grupo, no fim de um
 * grupo, ou num grupo novo à direita de `group`. */
type TabDrop = { group: string; index: number } | { group: string; split: true };

interface TabDragState {
  tab: Tab | null;
  from: string | null;
  drop: TabDrop | null;
  x: number;
  y: number;
}

const useTabDrag = create<TabDragState>(() => ({ tab: null, from: null, drop: null, x: 0, y: 0 }));

/** A faixa da direita do conteúdo de um grupo que abre um grupo novo. */
const SPLIT_ZONE = 0.25;

function dropAt(x: number, y: number): TabDrop | null {
  const element = document.elementFromPoint(x, y);
  const groupNode = element?.closest<HTMLElement>('[data-editor-group]');
  const group = groupNode?.dataset.editorGroup;
  if (!groupNode || !group) return null;
  const bar = element?.closest<HTMLElement>('.tabbar');
  if (bar) {
    const tabs = [...bar.querySelectorAll<HTMLElement>('.tab')];
    const index = tabs.findIndex((tab) => {
      const r = tab.getBoundingClientRect();
      return x < r.left + r.width / 2;
    });
    return { group, index: index < 0 ? tabs.length : index };
  }
  const body = groupNode.querySelector<HTMLElement>('.editor-area__body');
  const r = body?.getBoundingClientRect();
  if (r && x > r.right - r.width * SPLIT_ZONE && useEditor.getState().groups.length < MAX_GROUPS) return { group, split: true };
  return { group, index: Number.MAX_SAFE_INTEGER };
}

function beginTabDrag(event: React.PointerEvent, tab: Tab, from: string) {
  if (event.button !== 0 || (event.target as Element).closest('.tab__close')) return;
  const startX = event.clientX;
  const startY = event.clientY;
  let active = false;
  const move = (e: PointerEvent) => {
    if (!active) {
      if (Math.abs(e.clientX - startX) < 5 && Math.abs(e.clientY - startY) < 5) return;
      active = true;
      document.body.classList.add('is-dragging');
    }
    useTabDrag.setState({ tab, from, drop: dropAt(e.clientX, e.clientY), x: e.clientX, y: e.clientY });
  };
  const finish = (apply: boolean) => {
    window.removeEventListener('pointermove', move);
    window.removeEventListener('pointerup', up);
    window.removeEventListener('keydown', key, true);
    document.body.classList.remove('is-dragging');
    const { drop } = useTabDrag.getState();
    useTabDrag.setState({ tab: null, from: null, drop: null });
    if (!active || !apply || !drop) return;
    const editor = useEditor.getState();
    if ('split' in drop) {
      // Para a direita do grupo debaixo do ponteiro: se for o de origem, é
      // o mesmo que "mover para a direita".
      if (drop.group !== from) editor.dropTab(tab.id, from, drop.group, Number.MAX_SAFE_INTEGER);
      editor.moveTo(tab.id, drop.group, 'right');
    } else {
      editor.dropTab(tab.id, from, drop.group, drop.index);
    }
  };
  const up = () => finish(true);
  const key = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      finish(false);
    }
  };
  window.addEventListener('pointermove', move);
  window.addEventListener('pointerup', up);
  window.addEventListener('keydown', key, true);
}

/** O rótulo que segue o ponteiro enquanto uma aba é arrastada. */
export function TabDragGhost() {
  const tab = useTabDrag((s) => s.tab);
  const x = useTabDrag((s) => s.x);
  const y = useTabDrag((s) => s.y);
  if (!tab) return null;
  return (
    <div className="drag-ghost" style={{ left: x + 12, top: y + 12 }}>
      {tabTitle(tab)}
    </div>
  );
}

// Barra de abas --------------------------------------------------------------

function tabMenu(tab: Tab, group: EditorGroup, root: string | undefined): MenuItem[] {
  const editor = useEditor.getState();
  const index = editor.groups.findIndex((g) => g.id === group.id);
  const count = editor.groups.length;
  const canRight = index < count - 1 || count < MAX_GROUPS;
  const items: MenuItem[] = [
    { label: t('common.close'), keys: 'Ctrl+W', run: () => void editor.closeTab(tab.id, group.id) },
    { label: t('tab.closeOthers'), run: () => void editor.closeOthers(tab.id, group.id) },
    { label: t('tab.closeAll'), run: () => void editor.closeAll() },
    { separator: true },
    { label: t('tab.splitRight'), keys: 'Ctrl+\\', disabled: !canRight, run: () => editor.split(tab.id, 'right') },
    { label: t('tab.moveRight'), disabled: !canRight || (count === 1 && group.tabIds.length === 1), run: () => editor.moveTo(tab.id, group.id, 'right') },
    { label: t('tab.moveLeft'), disabled: index === 0, run: () => editor.moveTo(tab.id, group.id, 'left') },
  ];
  if (count > 1) items.push({ label: t('tab.closeGroup'), run: () => void editor.closeGroup(group.id) });
  if (tab.path) {
    items.push(
      { separator: true },
      { label: t('common.copyPath'), run: () => void navigator.clipboard.writeText(tab.path!) },
      { label: t('common.copyRelativePath'), run: () => void navigator.clipboard.writeText(relativeTo(tab.path!, root)) },
      { label: t('common.reveal'), run: () => void revealItemInDir(tab.path!).catch(showError) },
    );
  }
  return items;
}

function TabBar({ group, count }: { group: EditorGroup; count: number }) {
  useT();
  const all = useEditor((s) => s.tabs);
  const root = useProject((s) => s.snapshot?.root);
  const drop = useTabDrag((s) => (s.drop && !('split' in s.drop) && s.drop.group === group.id ? s.drop.index : null));
  const strip = useRef<HTMLDivElement>(null);
  const tabs = group.tabIds.map((id) => all.find((tab) => tab.id === id)).filter((tab): tab is Tab => !!tab);
  // No modelo `editor` da barra de ferramentas, o Executar fica no canto das
  // abas do grupo em uso; o arquivo da frente dele pode mudar a etapa.
  const run = useLayout((s) => !s.zen && s.live.bars.toolbar && s.live.bars.toolbarStyle === 'editor');
  const focused = useEditor((s) => s.activeGroup === group.id);

  // A aba ativa entra na vista rolando só a barra de abas. O `scrollIntoView`
  // rolava também os ancestrais, até a raiz da janela: com muitas abas, a
  // interface inteira deslizava para o lado junto com elas.
  useEffect(() => {
    const bar = strip.current;
    const tab = bar?.querySelector<HTMLElement>('.tab.is-active');
    if (!bar || !tab) return;
    const left = tab.offsetLeft;
    const right = left + tab.offsetWidth;
    if (left < bar.scrollLeft) bar.scrollLeft = left;
    else if (right > bar.scrollLeft + bar.clientWidth) bar.scrollLeft = right - bar.clientWidth;
  }, [group.activeId, tabs.length]);

  return (
    <div
      className="editor-group__header"
      // Fora das abas, as barras e as regiões da janela: a barra de abas nunca
      // some, então daqui sempre se volta ao que o layout escondeu.
      onContextMenu={(e) => {
        if (!(e.target as HTMLElement).closest('.tab')) openContextMenu(e, chromeMenu());
      }}
    >
      <div className="tabbar" ref={strip} role="tablist">
        {tabs.map((tab, index) => {
          const Icon = tabIcon(tab);
          const active = tab.id === group.activeId;
          const marker = drop === index ? ' is-drop-before' : drop !== null && index === tabs.length - 1 && drop >= tabs.length ? ' is-drop-after' : '';
          return (
            <div
              key={tab.id}
              role="tab"
              aria-selected={active}
              className={`tab${active ? ' is-active' : ''}${tab.preview ? ' is-preview' : ''}${tab.dirty ? ' is-dirty' : ''}${marker}`}
              title={tab.path ? relativeTo(tab.path, root) : tabTitle(tab)}
              onPointerDown={(e) => beginTabDrag(e, tab, group.id)}
              onMouseDown={(e) => {
                if (e.button === 1) {
                  e.preventDefault();
                  void useEditor.getState().closeTab(tab.id, group.id);
                }
              }}
              onClick={() => useEditor.getState().activate(tab.id, group.id)}
              onDoubleClick={() => useEditor.getState().pin(tab.id)}
              onContextMenu={(e) => openContextMenu(e, tabMenu(tab, group, root))}
            >
              <Icon size={14} className="tab__icon" />
              <span className="tab__title">{tabTitle(tab)}</span>
              <button
                type="button"
                className="tab__close"
                aria-label={t('common.close')}
                onClick={(e) => {
                  e.stopPropagation();
                  void useEditor.getState().closeTab(tab.id, group.id);
                }}
              >
                <span className="tab__dirty" aria-hidden />
                <X size={13} className="tab__x" />
              </button>
            </div>
          );
        })}
      </div>
      <div className="editor-group__actions">
        {run && focused && <RunControls variant="editor" path={tabs.find((tab) => tab.id === group.activeId)?.path ?? null} />}
        <IconButton
          label={`${t('tab.splitRight')} (Ctrl+\\)`}
          disabled={count >= MAX_GROUPS || !group.activeId}
          onClick={() => useEditor.getState().split(group.activeId ?? undefined, 'right')}
        >
          <Columns2 size={14} />
        </IconButton>
        {count > 1 && (
          <IconButton label={t('tab.closeGroup')} onClick={() => void useEditor.getState().closeGroup(group.id)}>
            <X size={14} />
          </IconButton>
        )}
      </div>
    </div>
  );
}

function ViewContent({ tab }: { tab: Tab }) {
  switch (tab.kind) {
    case 'settings':
      return <SettingsView />;
    case 'toolchain':
      return <ToolchainView checkToken={tab.data?.check ?? null} />;
    case 'schematic':
      return <SchematicView />;
    case 'synthesis':
      return <SynthesisView />;
    case 'board':
      return <BoardView />;
    case 'vboard':
      return <VirtualBoardView />;
    case 'report':
      return <ReportView id={tab.data?.id ?? ''} />;
    case 'compare':
      return <CompareView id={tab.data?.id ?? null} against={tab.data?.against ?? null} />;
    case 'processor':
      return <ProcessorView name={tab.data?.name ?? ''} />;
    case 'about':
      return <AboutView />;
    case 'wave':
      return <WaveView path={tab.data?.path ?? ''} />;
    case 'learn':
      return <LearnView />;
    case 'assistant':
      return <Chat mode="tab" />;
    case 'diff':
      return <DiffView tab={tab} />;
    default:
      return null;
  }
}

function FileNotice({ path }: { path: string }) {
  const doc = useEditor((s) => s.docs[path]);
  if (!doc) return null;
  return (
    <div className="file-notice">
      <p>{doc.tooLarge ? t('editor.tooLarge') : t('editor.binary')}</p>
      <button type="button" className="btn btn--default" onClick={() => void revealItemInDir(path).catch(showError)}>
        {t('common.reveal')}
      </button>
    </div>
  );
}

/** Um grupo: a barra de abas e o conteúdo da aba ativa dele. */
function EditorGroupView({ group, count }: { group: EditorGroup; count: number }) {
  const all = useEditor((s) => s.tabs);
  const docs = useEditor((s) => s.docs);
  const focused = useEditor((s) => s.activeGroup === group.id);
  const splitting = useTabDrag((s) => !!s.drop && 'split' in s.drop && s.drop.group === group.id);
  const hovered = useTabDrag((s) => !!s.drop && !('split' in s.drop) && s.drop.group === group.id && s.drop.index === Number.MAX_SAFE_INTEGER);
  const active = all.find((tab) => tab.id === group.activeId) ?? null;
  const lastFile = useRef<string | null>(null);
  // No zen as abas somem, a não ser que a preferência peça; Ctrl+P e a
  // paleta continuam trocando de arquivo.
  const zen = useLayout((s) => s.zen);
  const zenTabs = useApp((s) => s.settings?.zen?.show_tabs ?? false);
  const hideTabs = zen && !zenTabs;

  if (active?.kind === 'file' && active.path) lastFile.current = active.path;
  if (lastFile.current && !group.tabIds.includes(fileTabId(lastFile.current))) {
    lastFile.current = all.find((tab) => tab.kind === 'file' && group.tabIds.includes(tab.id))?.path ?? null;
  }
  const editorPath = lastFile.current;
  const editorDoc = editorPath ? docs[editorPath] : undefined;
  const showEditor = active?.kind === 'file' && !!editorDoc && !editorDoc.binary && !editorDoc.tooLarge;

  useEffect(() => setEditorVisible(group.id, showEditor), [group.id, showEditor]);

  return (
    <div
      className={`editor-group${count > 1 ? ' is-split' : ''}${focused ? ' is-focused' : ''}`}
      data-editor-group={group.id}
      // Clicar numa vista do grupo também o torna o ativo.
      onPointerDownCapture={() => useEditor.getState().focusGroup(group.id)}
    >
      {group.tabIds.length > 0 && !hideTabs && <TabBar group={group} count={count} />}
      <div className="editor-area__body">
        {editorPath && editorDoc && !editorDoc.binary && !editorDoc.tooLarge && (
          <div className="editor-area__editor" hidden={!showEditor}>
            <MonacoHost path={editorPath} group={group.id} />
          </div>
        )}
        {active?.kind === 'file' && active.path && !showEditor && <FileNotice path={active.path} />}
        {/* As abas de onda ficam montadas enquanto existem, escondidas quando
            não são a ativa: o cliente do Surfer guarda o zoom, o cursor e os
            sinais acrescentados, que se perdiam a cada troca de aba. */}
        {group.tabIds
          .map((id) => all.find((tab) => tab.id === id))
          .filter((tab): tab is Tab => tab?.kind === 'wave')
          .map((tab) => (
            <div className="editor-area__view" key={tab.id} hidden={tab.id !== active?.id}>
              <ViewContent tab={tab} />
            </div>
          ))}
        {active && active.kind !== 'file' && active.kind !== 'wave' && (
          <div className="editor-area__view" key={active.id}>
            <ViewContent tab={active} />
          </div>
        )}
        {!active && <WelcomeView />}
        {hovered && <div className="editor-group__drop" />}
        {splitting && <div className="editor-group__drop editor-group__drop--split" />}
      </div>
    </div>
  );
}

export function EditorArea() {
  const groups = useEditor((s) => s.groups);
  return (
    <div className="editor-area">
      <Group orientation="horizontal" className="editor-area__groups">
        {groups.map((group, index) => (
          <Fragment key={group.id}>
            {index > 0 && <Separator className="resize-handle resize-handle--vertical" />}
            <Panel id={group.id} minSize={160}>
              <EditorGroupView group={group} count={groups.length} />
            </Panel>
          </Fragment>
        ))}
      </Group>
      <TabDragGhost />
    </div>
  );
}
