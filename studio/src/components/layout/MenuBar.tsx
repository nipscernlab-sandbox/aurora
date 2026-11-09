// A barra de menus: Arquivo, Editar, Exibir, Projeto, Fluxo, Ferramentas,
// Ajuda. Cada item é uma ação de actions.ts; o atalho aparece ao lado, e a
// marca nas que ligam e desligam. O Exibir tem submenus (Consoles,
// Aparência): solto, ele não caberia na altura mínima da janela. Os projetos
// recentes do Arquivo também ficam num submenu, Abrir recente. As ações de
// recurso experimental desligado não aparecem, nem o separador delas.
//
// Os menus (`Menus`) ficam na barra de título integrada (TitleBar.tsx) ou,
// sem ela, numa linha própria (`MenuBar`).

import { useEffect, useRef, useState, type MouseEvent as ReactMouseEvent } from 'react';

import { action, isAvailable } from '../../actions';
import { useT, type Key } from '../../i18n';
import { useApp } from '../../state/app';
import { useExperimental } from '../../state/experimental';
import { useJobs } from '../../state/jobs';
import { useLayout } from '../../state/layout';
import { useProject } from '../../state/project';
import { AuroraMark, MenuList, openContextMenu, type MenuItem } from '../common';
import { actionItem, APPEARANCE_ENTRIES, chromeMenu, LAYOUT_ENTRIES } from './layoutMenus';
import { RunControls } from './RunControls';

/** Uma ação, um separador ou um submenu (um nível só). */
type Entry = string | { submenu: Key; entries: readonly string[] };

const CONSOLE_ENTRIES = [
  'viewConsoleInfo',
  '-',
  'viewConsoleCmm',
  'viewConsoleAsm',
  'viewConsoleVerilog',
  'viewConsoleWave',
  'viewConsoleRTLViewer',
  'viewConsoleFpga',
];

const MENUS: { label: Key; entries: Entry[] }[] = [
  {
    label: 'menu.file',
    entries: ['newProject', 'openProject', 'recent', 'closeProject', 'convertLayout', '-', 'newFile', 'newFolder', 'openFile', '-', 'save', 'saveAll', '-', 'closeTab', 'reopenTab', '-', 'revealProject', 'settings', '-', 'quit'],
  },
  { label: 'menu.edit', entries: ['undo', 'redo', '-', 'find', 'replace', 'findInFiles', '-', 'goToLine', 'formatDocument', '-', 'toggleVim'] },
  {
    label: 'menu.view',
    entries: [
      'commandPalette',
      'quickOpen',
      '-',
      'viewExplorer',
      'viewFlow',
      'viewSearch',
      'viewReports',
      'viewLearn',
      'viewGit',
      'viewProfile',
      'viewAssistant',
      'assistantTab',
      { submenu: 'menu.consoles', entries: CONSOLE_ENTRIES },
      'toggleTerminal',
      'showProblems',
      '-',
      { submenu: 'menu.appearance', entries: APPEARANCE_ENTRIES },
      { submenu: 'menu.layout', entries: LAYOUT_ENTRIES },
      'toggleZen',
      '-',
      'splitEditor',
      'closeEditorGroup',
      '-',
      'zoomIn',
      'zoomOut',
      'zoomReset',
      '-',
      'selectTheme',
      'toggleTheme',
    ],
  },
  {
    label: 'menu.project',
    entries: ['addVerilog', 'newVerilog', 'newTestbench', 'newCocotb', 'newProcessor', 'removeProcessor', '-', 'chooseTop', 'chooseTestbench', '-', 'refreshProject'],
  },
  {
    label: 'menu.flow',
    entries: ['build', 'check', 'lint', '-', 'simulate', 'fastSim', 'openWave', 'waveSignals', '-', 'synthesize', 'showSchematic', 'showStatistics', 'viewHierarchy', '-', 'showBoard', 'showVirtualBoard', 'fpgaBuildProgram', 'fpgaBuild', 'fpgaProgram', '-', 'fullFlow', 'cancel', '-', 'useIcarus', 'useVerilator'],
  },
  {
    label: 'menu.tools',
    entries: [
      'toolchain',
      'installComponents',
      'checkUpdates',
      '-',
      'history',
      'lastReport',
      'compareReports',
      'cleanReports',
      '-',
      { submenu: 'menu.learn', entries: ['viewLearn', 'learnCheck', 'learnHint', 'learnWave', 'learnNext'] },
    ],
  },
  { label: 'menu.help', entries: ['auroraDocs', 'saphoManual', '-', 'startTour', 'shortcuts', 'about'] },
];

/** Sem separador no começo, no fim nem dois seguidos: as ações que não
 * existem nesta sessão podiam deixar dois juntos. */
function tidy(items: MenuItem[]): MenuItem[] {
  const out: MenuItem[] = [];
  for (const item of items) {
    if (item.separator && (out.length === 0 || out[out.length - 1].separator)) continue;
    out.push(item);
  }
  while (out.length > 0 && out[out.length - 1].separator) out.pop();
  return out;
}

function useItems(entries: Entry[]): MenuItem[] {
  const t = useT();
  const recent = useApp((s) => s.recent);
  // Redesenha quando muda o que habilita as ações ou o que elas marcam.
  useApp((s) => s.settings);
  useApp((s) => s.toolchain);
  useJobs((s) => s.running);
  useProject((s) => s.snapshot);
  useLayout((s) => s.live);
  useLayout((s) => s.panelMaximized);
  useExperimental((s) => s.enabled);

  const build = (list: readonly Entry[]): MenuItem[] => {
    const items: MenuItem[] = [];
    for (const entry of list) {
      if (entry === '-') {
        items.push({ separator: true });
      } else if (typeof entry === 'object') {
        items.push({ label: t(entry.submenu), submenu: build(entry.entries) });
      } else if (entry === 'recent') {
        // Os recentes num submenu, que abre ao lado com o mouse em cima.
        const existing = recent.filter((r) => r.exists).slice(0, 8);
        items.push({
          label: t('menu.openRecent'),
          submenu:
            existing.length === 0
              ? [{ label: t('menu.noRecent'), disabled: true }]
              : existing.map((r) => ({ label: r.name, run: () => void useProject.getState().open(r.spf) })),
        });
      } else if (isAvailable(action(entry))) {
        items.push(actionItem(entry));
      }
    }
    return tidy(items);
  };
  return build(entries);
}

function Menu({
  label,
  entries,
  open,
  onOpen,
  onHover,
  onClose,
}: {
  label: Key;
  entries: Entry[];
  open: boolean;
  onOpen: () => void;
  onHover: () => void;
  onClose: () => void;
}) {
  const t = useT();
  const items = useItems(entries);
  return (
    // Na barra de título integrada, um clique no menu aberto (num separador,
    // na borda) não arrasta a janela.
    <div className="menubar__menu" data-tauri-drag-region="false">
      <button
        type="button"
        className={`menubar__item${open ? ' is-open' : ''}`}
        onClick={() => (open ? onClose() : onOpen())}
        onMouseEnter={onHover}
      >
        {t(label)}
      </button>
      {open && (
        <div className="menu menu--dropdown" role="menu">
          <MenuList items={items} onDone={onClose} />
        </div>
      )}
    </div>
  );
}

/** Botão direito no vazio da barra: o menu de aparência (layoutMenus.ts). */
export function chromeContextMenu(event: ReactMouseEvent<HTMLElement>) {
  if (event.target instanceof Element && event.target.closest('button, [role="menu"]')) return;
  openContextMenu(event, chromeMenu());
}

/** Os menus, um aberto de cada vez: na barra de menus e na barra de título
 * integrada (TitleBar.tsx). Com um aberto, passar o mouse noutro o abre. */
export function Menus() {
  const [open, setOpen] = useState<number | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (open === null) return;
    const close = (e: MouseEvent) => {
      if (ref.current && e.target instanceof Node && ref.current.contains(e.target)) return;
      setOpen(null);
    };
    const key = (e: KeyboardEvent) => e.key === 'Escape' && setOpen(null);
    const blur = () => setOpen(null);
    window.addEventListener('mousedown', close, true);
    window.addEventListener('keydown', key, true);
    window.addEventListener('blur', blur);
    return () => {
      window.removeEventListener('mousedown', close, true);
      window.removeEventListener('keydown', key, true);
      window.removeEventListener('blur', blur);
    };
  }, [open]);

  return (
    <div className="menubar__menus" ref={ref}>
      {MENUS.map((menu, index) => (
        <Menu
          key={menu.label}
          label={menu.label}
          entries={menu.entries}
          open={open === index}
          onOpen={() => setOpen(index)}
          onHover={() => open !== null && setOpen(index)}
          onClose={() => setOpen(null)}
        />
      ))}
    </div>
  );
}

/** A barra de menus numa linha própria, quando a barra de título integrada
 * não está à vista: em tela cheia, ou com a janela ainda com a moldura do
 * sistema (state/titleBar.ts). Os controles de execução ficam no fim dela;
 * com os menus escondidos, ela aparece só com eles. */
export function MenuBar({ menus, run }: { menus: boolean; run: boolean }) {
  return (
    <div className="menubar" onContextMenu={chromeContextMenu}>
      <AuroraMark className="menubar__logo" />
      {menus && <Menus />}
      {run && <RunControls variant="titlebar" />}
    </div>
  );
}
