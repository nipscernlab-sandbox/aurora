// O explorador, em três modos:
//
// - Fontes: a vista lógica do projeto, como o "Sources" do Vivado e o modo
//   Files da AURORA. Módulos (com o topo marcado e o Verilog gerado de cada
//   processador), testbenches (com o simulado marcado e o testbench gerado de
//   cada processador), processadores SAPHO com o programa, as memórias, as
//   entradas, as saídas e os intermediários do YANC separados, e os .v da
//   pasta que ainda não estão no .spf. Arrastar um Verilog para Módulos ou
//   Testbenches muda o papel dele.
// - Hierarquia: a árvore de instâncias depois da elaboração pelo Icarus
//   (state/hierarchy.ts), do design e de cada testbench.
// - Arquivos: a árvore da pasta, com criar, renomear, mandar para a lixeira,
//   arrastar para mover e soltar arquivos do sistema para copiar.
//
// Nas três, as setas andam pela árvore (direita abre, esquerda fecha ou
// sobe), Enter abre, F2 renomeia e Delete manda para a lixeira. Com o
// controle de versão ligado (state/git.ts), cada arquivo mudado tem a letra
// da mudança e o nome na cor dela, e cada pasta com mudança, um ponto.

import { revealItemInDir } from '@tauri-apps/plugin-opener';
import {
  Activity,
  Boxes,
  ChevronDown,
  ChevronRight,
  ChevronsDownUp,
  ChevronsUpDown,
  CircleAlert,
  Cpu,
  Eye,
  EyeOff,
  FilePlus,
  Folder,
  FolderOpen,
  FolderPlus,
  ListPlus,
  Package,
  Plus,
  RefreshCw,
  SquareFunction,
} from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';

import {
  addVerilogFiles,
  newFileIn,
  newFolderIn,
  openWave,
  removeProcessor,
  runBuild,
  runCheck,
  runFastSimulation,
  runSimulation,
  runSynthesis,
} from '../../actions';
import { errorText, t, useLang, useT, type Key } from '../../i18n';
import { api } from '../../ipc/api';
import type { DirEntry, Elaboration, ModuleInstance, ProcessorStatus, ProjectFile } from '../../ipc/types';
import { confirm, openDialog, prompt } from '../../state/dialogs';
import { useEditor } from '../../state/editor';
import { useGitMark } from '../../state/git';
import { useHierarchy } from '../../state/hierarchy';
import { useJobs } from '../../state/jobs';
import { useLayout, type ExplorerMode } from '../../state/layout';
import { useProject } from '../../state/project';
import { guarded, showError } from '../../state/toasts';
import { baseName, dirName, extension, isInside, joinPath, relativeTo, relativeUp, samePath } from '../../util/paths';
import { Badge, Empty, FoldContext, IconButton, openContextMenu, Section, Spinner, useFoldable, type MenuItem } from '../common';
import { fileIcon } from '../editor/tabIcons';
import { beginDrag, movePath, reorder, sameTarget, useDrag, type DragSource, type DropSection } from './dnd';
import { ViewActions } from '../layout/ViewActions';
import { ChangeLetter } from './GitMark';
import { MissingSpfNotice } from './MissingSpfNotice';

// Uma linha da árvore ----------------------------------------------------

interface RowProps {
  depth: number;
  icon: ReactNode;
  label: ReactNode;
  /** Texto depois do rótulo, apagado (o módulo de uma instância). */
  detail?: ReactNode;
  title?: string;
  badges?: ReactNode;
  expanded?: boolean;
  selected?: boolean;
  muted?: boolean;
  /** O arquivo da linha (`data-path`), para revelar e para os testes. */
  path?: string;
  /** O arquivo da marca do git, quando a linha não tem `path` (uma instância
   * da hierarquia, pelo arquivo que a define). */
  gitPath?: string;
  /** A linha é de uma pasta: a marca do git é um ponto, se há mudança
   * dentro dela. */
  dir?: boolean;
  /** A pasta que recebe o que for solto na linha (`data-drop-dir`). */
  dropDir?: string;
  dropActive?: boolean;
  /** O arquivo das fontes antes do qual o que for solto na linha entra
   * (`data-drop-order`), e se ele é o destino agora. */
  dropOrder?: string;
  dropBefore?: boolean;
  onClick?: () => void;
  onDoubleClick?: () => void;
  onContextMenu?: (e: React.MouseEvent) => void;
  onPointerDown?: (e: React.PointerEvent) => void;
}

function Row({
  depth,
  icon,
  label,
  detail,
  title,
  badges,
  expanded,
  selected,
  muted,
  path,
  gitPath,
  dir,
  dropDir,
  dropActive,
  dropOrder,
  dropBefore,
  onClick,
  onDoubleClick,
  onContextMenu,
  onPointerDown,
}: RowProps) {
  const git = useGitMark(gitPath ?? path, !!dir);
  return (
    <div
      className={`tree__row${selected ? ' is-selected' : ''}${muted ? ' is-muted' : ''}${dropActive ? ' is-drop-target' : ''}${dropBefore ? ' is-drop-before' : ''}${git && git !== 'dir' ? ` tree__row--git-${git}` : ''}`}
      style={{ paddingLeft: 8 + depth * 14, ['--depth' as string]: depth }}
      title={title}
      role="treeitem"
      aria-expanded={expanded}
      aria-level={depth + 1}
      tabIndex={0}
      data-path={path}
      data-drop-dir={dropDir}
      data-drop-order={dropOrder}
      onClick={onClick}
      onDoubleClick={onDoubleClick}
      onContextMenu={onContextMenu}
      onPointerDown={onPointerDown}
      onKeyDown={(e) => {
        if (e.key !== 'Enter') return;
        // Ctrl+Enter num arquivo: abre no grupo ao lado, como no VS Code.
        if ((e.ctrlKey || e.metaKey) && path && expanded === undefined) openAside(path);
        else (onDoubleClick ?? onClick)?.();
      }}
    >
      <span className="tree__twisty">
        {expanded === undefined ? null : expanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
      </span>
      <span className="tree__icon">{icon}</span>
      <span className="tree__label">
        {label}
        {detail && <span className="tree__detail">{detail}</span>}
      </span>
      {(badges || git) && (
        <span className="tree__badges">
          {badges}
          {git === 'dir' ? <span className="tree__git-dot" aria-hidden /> : git ? <ChangeLetter change={git} /> : null}
        </span>
      )}
    </div>
  );
}

/** As setas do teclado numa árvore: cima e baixo andam, direita abre (ou
 * entra), esquerda fecha (ou sobe para o pai). */
function treeKeys(e: React.KeyboardEvent<HTMLDivElement>) {
  if (!['ArrowDown', 'ArrowUp', 'ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(e.key)) return;
  const rows = Array.from(e.currentTarget.querySelectorAll<HTMLElement>('[role="treeitem"]'));
  const current = document.activeElement as HTMLElement | null;
  const index = current ? rows.indexOf(current) : -1;
  if (index < 0) return;
  e.preventDefault();
  const row = rows[index];
  const expanded = row.getAttribute('aria-expanded');
  const level = Number(row.getAttribute('aria-level') ?? 1);
  switch (e.key) {
    case 'ArrowDown':
      rows[Math.min(index + 1, rows.length - 1)].focus();
      break;
    case 'ArrowUp':
      rows[Math.max(index - 1, 0)].focus();
      break;
    case 'Home':
      rows[0].focus();
      break;
    case 'End':
      rows[rows.length - 1].focus();
      break;
    case 'ArrowRight':
      if (expanded === 'false') row.click();
      else if (expanded === 'true') rows[index + 1]?.focus();
      break;
    case 'ArrowLeft':
      if (expanded === 'true') row.click();
      else {
        for (let i = index - 1; i >= 0; i--) {
          if (Number(rows[i].getAttribute('aria-level') ?? 1) < level) {
            rows[i].focus();
            break;
          }
        }
      }
      break;
  }
}

function FileIcon({ path }: { path: string }) {
  const Icon = fileIcon(path);
  return <Icon size={14} />;
}

function useActivePath(): string | null {
  return useEditor((s) => s.tabs.find((tab) => tab.id === s.activeId)?.path ?? null);
}

function openPreview(path: string, line?: number | null) {
  void useEditor.getState().openFile(path, { preview: true, ...(line ? { line } : {}) });
}

function openPinned(path: string, line?: number | null) {
  void useEditor.getState().openFile(path, line ? { line } : undefined);
}

/** Abre no grupo do editor à direita (cria um, se couber). */
function openAside(path: string) {
  void useEditor.getState().openFile(path, { side: true });
}

/** "Abrir" e "Abrir ao lado", o começo do menu de um arquivo. */
function openItems(path: string): MenuItem[] {
  return [
    { label: t('common.open'), run: () => openPinned(path) },
    { label: t('explorer.openAside'), keys: 'Ctrl+Enter', run: () => openAside(path) },
  ];
}

function commonFileItems(path: string, root: string): MenuItem[] {
  return [
    { label: t('common.copyPath'), run: () => void navigator.clipboard.writeText(path) },
    { label: t('common.copyRelativePath'), run: () => void navigator.clipboard.writeText(relativeTo(path, root)) },
    { label: t('common.reveal'), run: () => void revealItemInDir(path).catch(showError) },
  ];
}

async function afterProjectChange() {
  await useProject.getState().refresh();
}

/** O arquivo pode ser o topo. A lista vem do backend, com a regra do Core:
 * qualquer Verilog, inclusive o gerado de um processador, menos nome de
 * testbench (`tb_<nome>.v`, `<nome>_tb.v`). */
function canBeTop(path: string): boolean {
  return useProject.getState().snapshot?.top_candidates.some((c) => samePath(c, path)) ?? false;
}

/** Escolhe o topo pelo arquivo (registrando-o, se preciso) e, num arquivo
 * com vários módulos, pelo módulo pedido. */
async function setTopFile(path: string, module?: string) {
  if ((await guarded(() => api.project.setTop(path))) === undefined) return;
  await afterProjectChange();
  if (module && useProject.getState().snapshot?.top_module !== module) {
    await guarded(() => api.project.setTop(module));
    await afterProjectChange();
  }
}

function topItem(path: string, isTop: boolean, module?: string): MenuItem {
  return { label: t('explorer.setTop'), disabled: isTop || !canBeTop(path), run: () => void setTopFile(path, module) };
}

/** O processador que gerou este Verilog, se foi um build. */
function generator(path: string): ProcessorStatus | undefined {
  return useProject.getState().snapshot?.processors.find((p) => samePath(p.generated.verilog, path));
}

/** Uma subpasta lógica da árvore, que abre e fecha. */
function Group({
  depth,
  label,
  count,
  defaultOpen = true,
  muted,
  children,
}: {
  depth: number;
  label: string;
  count?: number;
  defaultOpen?: boolean;
  muted?: boolean;
  children: ReactNode;
}) {
  const [open, setOpen] = useFoldable(defaultOpen);
  return (
    <>
      <Row
        depth={depth}
        icon={open ? <FolderOpen size={14} /> : <Folder size={14} />}
        label={label}
        badges={count !== undefined ? <span className="tree__count">{count}</span> : undefined}
        expanded={open}
        muted={muted}
        onClick={() => setOpen(!open)}
      />
      {open && children}
    </>
  );
}

/** O destino do arrasto sobre esta seção: ela mesma, ou uma linha dela
 * quando o que vem é de fora da seção (aí soltar troca o papel, e não a
 * ordem). */
function useSectionDrop(section: DropSection): boolean {
  return useDrag(
    (s) =>
      (s.source !== null || s.external) &&
      s.allowed &&
      (sameTarget(s.target, { kind: 'section', section }) ||
        (s.target?.kind === 'order' && s.target.section === section && s.source?.section !== section)),
  );
}

// Fontes -------------------------------------------------------------------

function VerilogRow({ file, kind }: { file: ProjectFile; kind: 'module' | 'testbench' }) {
  const snapshot = useProject((s) => s.snapshot)!;
  const active = useActivePath();
  const isTop = kind === 'module' && samePath(file.path, snapshot.top_level);
  const isSim = kind === 'testbench' && samePath(file.path, snapshot.selected_testbench);
  const section: DropSection = kind === 'module' ? 'modules' : 'testbenches';
  // Um arquivo da mesma seção arrastado sobre esta linha entra antes dela.
  const dropBefore = useDrag(
    (s) =>
      s.allowed &&
      s.source?.section === section &&
      s.target?.kind === 'order' &&
      samePath(s.target.path, file.path),
  );
  // Os vizinhos na lista, para subir e descer pelo menu.
  const list = kind === 'module' ? snapshot.synthesizable : snapshot.testbenches;
  const index = list.findIndex((f) => samePath(f.path, file.path));
  const previous = index > 0 ? list[index - 1] : undefined;
  const next = index >= 0 && index + 1 < list.length ? list[index + 1] : undefined;

  const menu = (e: React.MouseEvent) => {
    const items: MenuItem[] = [...openItems(file.path), { separator: true }];
    if (kind === 'module') {
      items.push(
        topItem(file.path, isTop),
        {
          label: t('explorer.makeTestbench'),
          run: async () => {
            await guarded(() => api.project.addVerilog(file.path, true));
            await afterProjectChange();
          },
        },
      );
    } else {
      // Só marca: quem simula é a Wave (F8) ou a Rápida (F9), quando o
      // usuário pedir.
      items.push({
        label: t('explorer.setTestbench'),
        disabled: isSim,
        run: async () => {
          await guarded(() => api.project.setTestbench(file.path));
          await afterProjectChange();
        },
      });
      // A escolha de sinais é a do testbench simulado; um cocotb grava tudo.
      if (isSim && extension(file.path) !== 'py') {
        items.push({ label: t('action.waveSignals'), run: () => openDialog({ kind: 'waveSignals' }) });
      }
      // Um testbench sem nome de testbench também pode virar o topo (muda de
      // lista); o nome tb_x.v ou x_tb.v, não.
      if (canBeTop(file.path)) items.push(topItem(file.path, false));
    }
    // O check elabora Verilog; um testbench cocotb (.py) só roda simulado.
    if (extension(file.path) !== 'py') {
      items.push({ label: t('explorer.checkFile'), run: () => void runCheck(false, file.path) });
    }
    items.push(
      { separator: true },
      {
        label: t('explorer.moveUp'),
        disabled: !previous,
        run: () => previous && void reorder(file.path, { kind: 'before', path: previous.path }),
      },
      {
        label: t('explorer.moveDown'),
        disabled: !next,
        run: () => next && void reorder(file.path, { kind: 'after', path: next.path }),
      },
      { separator: true },
      {
        label: t('explorer.removeFromProject'),
        run: async () => {
          const answer = await confirm({
            title: t('common.remove'),
            message: t('dialog.confirmRemove', { name: baseName(file.path) }),
            buttons: [
              { label: t('common.remove'), value: 'yes', danger: true },
              { label: t('common.cancel'), value: 'no' },
            ],
          });
          if (answer !== 'yes') return;
          await guarded(() => api.project.removeVerilog(file.path));
          await afterProjectChange();
        },
      },
      { separator: true },
      ...commonFileItems(file.path, snapshot.root),
    );
    openContextMenu(e, items);
  };

  // O Verilog gerado de um processador, registrado (por exemplo, como topo).
  const builtBy = kind === 'module' ? generator(file.path) : undefined;
  // Fora da pasta do projeto (o HITS usa `../../rtl/`), o caminho absoluto
  // não cabe na linha: vai o nome, com a pasta relativa ao lado.
  const outside = !isInside(file.path, snapshot.root);
  return (
    <Row
      depth={1}
      icon={builtBy ? <Cpu size={14} /> : <FileIcon path={file.path} />}
      label={outside ? baseName(file.path) : relativeTo(file.path, snapshot.root)}
      detail={outside ? dirName(relativeUp(file.path, snapshot.root)) : undefined}
      title={builtBy ? `${file.path}\n${t('explorer.generatedBy', { name: builtBy.name })}` : file.path}
      path={file.path}
      selected={samePath(active, file.path)}
      badges={
        <>
          {builtBy && <Badge>{t('explorer.generatedBadge')}</Badge>}
          {isTop && (
            <Badge tone="accent" title={snapshot.top_module_error?.message}>
              {snapshot.top_module ? `${t('explorer.topBadge')}: ${snapshot.top_module}` : t('explorer.topBadge')}
            </Badge>
          )}
          {isSim && <Badge tone="accent">{t('explorer.simBadge')}</Badge>}
        </>
      }
      onClick={() => openPreview(file.path)}
      onDoubleClick={() => openPinned(file.path)}
      onContextMenu={menu}
      dropOrder={file.path}
      dropBefore={dropBefore}
      onPointerDown={(e) => beginDrag(e, { path: file.path, isDir: false, movable: false, verilog: true, section })}
    />
  );
}

/** Marca o testbench que o build gerou como o simulado. Ele ainda não está no
 * `.spf`: entra como testbench, como no "Adicionar como testbench" da aba
 * Arquivos, e só então é marcado. */
async function markGeneratedTestbench(path: string) {
  if ((await guarded(() => api.project.addVerilog(path, true))) === undefined) return;
  await guarded(() => api.project.setTestbench(path));
  await afterProjectChange();
}

/** O Verilog ou o testbench que o build de um processador gerou. Fica na
 * seção do papel dele, marcado como gerado. */
function GeneratedRow({ processor, path, kind }: { processor: ProcessorStatus; path: string; kind: 'module' | 'testbench' }) {
  const snapshot = useProject((s) => s.snapshot)!;
  const active = useActivePath();
  const name = processor.name;
  const menu = (e: React.MouseEvent) =>
    openContextMenu(e, [
      ...openItems(path),
      { separator: true },
      ...(kind === 'module'
        ? [topItem(path, false), { separator: true }]
        : [{ label: t('explorer.setTestbench'), run: () => void markGeneratedTestbench(path) }, { separator: true }]),
      { label: t('toolbar.build'), keys: 'F6', run: () => void runBuild(name) },
      ...(kind === 'testbench'
        ? [
            { label: t('action.simulate'), run: () => void runSimulation(true, name) },
            { label: t('action.fastSim'), run: () => void runFastSimulation(name) },
            { label: t('action.openWave'), disabled: !processor.waveform, run: () => void openWave(name) },
          ]
        : [{ label: t('action.synthesize'), run: () => void runSynthesis(name) }]),
      { separator: true },
      ...commonFileItems(path, snapshot.root),
    ]);
  return (
    <Row
      depth={1}
      icon={<Cpu size={14} />}
      label={baseName(path)}
      detail={relativeTo(dirName(path), snapshot.root)}
      title={`${relativeTo(path, snapshot.root)}\n${t('explorer.generatedBy', { name })}`}
      path={path}
      selected={samePath(active, path)}
      badges={<Badge title={t('explorer.generatedBy', { name })}>{t('explorer.generatedBadge')}</Badge>}
      onClick={() => openPreview(path)}
      onDoubleClick={() => openPinned(path)}
      onContextMenu={menu}
    />
  );
}

function ProcessorNode({ processor }: { processor: ProcessorStatus }) {
  const snapshot = useProject((s) => s.snapshot)!;
  const target = useProject((s) => s.target);
  const active = useActivePath();
  const [open, setOpen] = useFoldable(true);
  const root = snapshot.root;
  const name = processor.name;
  const generated = processor.generated;

  const menu = (e: React.MouseEvent) =>
    openContextMenu(e, [
      { label: t('toolbar.build'), keys: 'F6', run: () => void runBuild(name) },
      { label: t('action.simulate'), run: () => void runSimulation(true, name) },
      { label: t('action.fastSim'), run: () => void runFastSimulation(name) },
      { label: t('action.openWave'), disabled: !processor.waveform, run: () => void openWave(name) },
      { label: t('action.synthesize'), run: () => void runSynthesis(name) },
      { separator: true },
      { label: t('explorer.configure'), run: () => useEditor.getState().openView('processor', { name }) },
      { label: t('explorer.newInput'), run: () => openDialog({ kind: 'newInput', processor: name }) },
      { separator: true },
      { label: t('common.reveal'), run: () => void revealItemInDir(processor.dir).catch(showError) },
      { separator: true },
      // Não no meio de uma operação, que pode estar usando o processador.
      { label: t('action.removeProcessor'), danger: true, disabled: !!useJobs.getState().running, run: () => void removeProcessor(name) },
    ]);

  const fileRow = (path: string, depth: number, options: { label?: string; badge?: string; muted?: boolean } = {}) => (
    <Row
      key={path}
      depth={depth}
      icon={<FileIcon path={path} />}
      label={options.label ?? baseName(path)}
      title={relativeTo(path, root)}
      path={path}
      muted={options.muted}
      selected={samePath(active, path)}
      badges={options.badge ? <Badge>{options.badge}</Badge> : undefined}
      onClick={() => openPreview(path)}
      onDoubleClick={() => openPinned(path)}
      onContextMenu={(e) => openContextMenu(e, commonFileItems(path, root))}
    />
  );
  const gen = t('explorer.generatedBadge');

  return (
    <>
      <Row
        depth={1}
        icon={<Cpu size={14} />}
        label={name}
        expanded={open}
        selected={target === name}
        badges={
          <>
            <Badge>{processor.language === 'cpp' ? 'C' : 'C±'}</Badge>
            <Badge tone={processor.built ? 'ok' : 'muted'}>{processor.built ? t('explorer.built') : t('explorer.notBuilt')}</Badge>
          </>
        }
        onClick={() => setOpen(!open)}
        onDoubleClick={() => useEditor.getState().openView('processor', { name })}
        onContextMenu={menu}
      />
      {open && (
        <>
          <Group depth={2} label={t('explorer.program')}>
            {fileRow(processor.source, 3)}
            {generated.assembly && fileRow(generated.assembly, 3, { badge: gen })}
          </Group>
          {generated.memories.length > 0 && (
            <Group depth={2} label={t('explorer.memories')} count={generated.memories.length}>
              {generated.memories.map((path) => fileRow(path, 3))}
            </Group>
          )}
          <Group depth={2} label={t('explorer.simulation')}>
            {processor.inputs.length === 0 && processor.outputs.length === 0 && !processor.waveform && (
              <p className="tree__empty" style={{ paddingLeft: 8 + 3 * 14 }}>
                {t('explorer.noSimulationFiles')}
              </p>
            )}
            {processor.inputs.map((input) => fileRow(input.path, 3, { label: `${t('explorer.input')} ${input.port}: ${baseName(input.path)}` }))}
            {processor.outputs.map((output) => fileRow(output.path, 3, { label: `${t('explorer.output')} ${output.port}: ${baseName(output.path)}` }))}
            {processor.waveform && (
              <Row
                depth={3}
                icon={<Activity size={14} />}
                label={baseName(processor.waveform)}
                title={relativeTo(processor.waveform, root)}
                onDoubleClick={() => void openWave(name)}
                onContextMenu={(e) =>
                  openContextMenu(e, [{ label: t('action.openWave'), run: () => void openWave(name) }, { separator: true }, ...commonFileItems(processor.waveform!, root)])
                }
              />
            )}
          </Group>
          {generated.intermediates.length > 0 && (
            <Group depth={2} label={t('explorer.intermediates')} count={generated.intermediates.length} defaultOpen={false} muted>
              {generated.intermediates.map((path) => fileRow(path, 3, { muted: true }))}
            </Group>
          )}
        </>
      )}
    </>
  );
}

/** O + de um arquivo fora do projeto: registra no .spf, como o "Adicionar ao
 * projeto" do menu de contexto, que decide se é módulo ou testbench. Só
 * aparece com o ponteiro ou o foco na linha (app.css). O clique, a tecla e
 * o arrastar ficam no botão e não chegam à linha, que abriria o arquivo ou
 * começaria a arrastá-lo. */
function AddToProjectButton({ path }: { path: string }) {
  return (
    <IconButton
      label={t('explorer.addToProject')}
      className="icon-btn--small tree__add"
      onClick={(e) => {
        e.stopPropagation();
        void addVerilogFiles([path]);
      }}
      onDoubleClick={(e) => e.stopPropagation()}
      onPointerDown={(e) => e.stopPropagation()}
      onKeyDown={(e) => e.stopPropagation()}
    >
      <Plus size={14} />
    </IconButton>
  );
}

function SourcesView() {
  useT();
  const snapshot = useProject((s) => s.snapshot)!;
  const root = snapshot.root;
  const modulesDrop = useSectionDrop('modules');
  const testbenchesDrop = useSectionDrop('testbenches');
  // O gerado que já está no .spf (escolhido como topo, por exemplo) aparece
  // uma vez só, na linha do registrado.
  const registered = (path: string | null, files: ProjectFile[]) => files.some((f) => samePath(f.path, path));
  const generatedVerilog = snapshot.processors.filter(
    (p) => p.generated.verilog && !registered(p.generated.verilog, snapshot.synthesizable),
  );
  const generatedTestbenches = snapshot.processors.filter(
    (p) => p.generated.testbench && !registered(p.generated.testbench, snapshot.testbenches),
  );
  const moduleCount = snapshot.synthesizable.length + generatedVerilog.length;
  const testbenchCount = snapshot.testbenches.length + generatedTestbenches.length;

  return (
    <div className="tree" role="tree" onKeyDown={treeKeys}>
      <Section
        title={t('explorer.modules')}
        count={moduleCount}
        drop="modules"
        dropActive={modulesDrop}
        actions={
          <IconButton label={t('action.newVerilog')} onClick={() => openDialog({ kind: 'newVerilog', testbench: false })}>
            <Plus size={14} />
          </IconButton>
        }
      >
        {moduleCount === 0 && <p className="tree__empty">{t('explorer.emptyModules')}</p>}
        {snapshot.synthesizable.map((file) => (
          <VerilogRow key={file.path} file={file} kind="module" />
        ))}
        {generatedVerilog.map((p) => (
          <GeneratedRow key={p.generated.verilog} processor={p} path={p.generated.verilog!} kind="module" />
        ))}
      </Section>

      <Section
        title={t('explorer.testbenches')}
        count={testbenchCount}
        drop="testbenches"
        dropActive={testbenchesDrop}
        actions={
          <IconButton label={t('action.newTestbench')} onClick={() => openDialog({ kind: 'newVerilog', testbench: true })}>
            <Plus size={14} />
          </IconButton>
        }
      >
        {testbenchCount === 0 && <p className="tree__empty">{t('explorer.emptyTestbenches')}</p>}
        {snapshot.testbenches.map((file) => (
          <VerilogRow key={file.path} file={file} kind="testbench" />
        ))}
        {generatedTestbenches.map((p) => (
          <GeneratedRow key={p.generated.testbench} processor={p} path={p.generated.testbench!} kind="testbench" />
        ))}
      </Section>

      <Section
        title={t('explorer.processors')}
        count={snapshot.processors.length}
        actions={
          <IconButton label={t('action.newProcessor')} onClick={() => openDialog({ kind: 'newProcessor' })}>
            <Plus size={14} />
          </IconButton>
        }
      >
        {snapshot.processors.length === 0 && <p className="tree__empty">{t('explorer.emptyProcessors')}</p>}
        {snapshot.processors.map((processor) => (
          <ProcessorNode key={processor.name} processor={processor} />
        ))}
      </Section>

      {snapshot.unregistered.length > 0 && (
        <Section
          title={t('explorer.unregistered')}
          count={snapshot.unregistered.length}
          defaultOpen={false}
          actions={
            // Não o + dos outros cabeçalhos, que cria arquivo novo: este
            // registra os que já existem.
            <IconButton label={t('explorer.addAllToProject')} onClick={() => void addVerilogFiles(snapshot.unregistered)}>
              <ListPlus size={14} />
            </IconButton>
          }
        >
          {snapshot.unregistered.map((path) => (
            <Row
              key={path}
              depth={1}
              icon={<FileIcon path={path} />}
              label={relativeTo(path, root)}
              title={t('explorer.unregisteredHint')}
              path={path}
              muted
              badges={<AddToProjectButton path={path} />}
              onClick={() => openPreview(path)}
              onDoubleClick={() => openPinned(path)}
              onPointerDown={(e) => beginDrag(e, { path, isDir: false, movable: false, verilog: true })}
              onContextMenu={(e) =>
                openContextMenu(e, [
                  { label: t('explorer.addToProject'), run: () => void addVerilogFiles([path]) },
                  ...(extension(path) === 'py' ? [] : [topItem(path, false)]),
                  {
                    label: t('explorer.addAsTestbench'),
                    run: async () => {
                      await guarded(() => api.project.addVerilog(path, true));
                      await afterProjectChange();
                    },
                  },
                  { separator: true },
                  ...commonFileItems(path, root),
                ])
              }
            />
          ))}
        </Section>
      )}
      <p className="tree__hint">{t('explorer.dragHint')}</p>
    </div>
  );
}

// Hierarquia -----------------------------------------------------------------

function InstanceNode({
  node,
  depth,
  path,
  expanded,
  toggle,
  root,
  processors,
  topModule,
}: {
  node: ModuleInstance;
  depth: number;
  path: string;
  expanded: Set<string>;
  toggle: (path: string) => void;
  root: string;
  processors: Set<string>;
  topModule: string | null;
}) {
  const active = useActivePath();
  const hasChildren = node.children.length > 0;
  // Abertas: as raízes e o primeiro nível; a biblioteca SAPHO, fechada.
  const defaultOpen = depth < 2 && !node.library;
  const open = hasChildren && expanded.has(path) !== defaultOpen;
  const isProcessor = processors.has(node.module) && !node.library;
  const icon = isProcessor ? <Cpu size={14} /> : node.library ? <Package size={14} /> : <SquareFunction size={14} />;
  const isTop = depth === 0 && !node.library && node.module === topModule;
  const file = node.file;

  const menu = (e: React.MouseEvent) =>
    openContextMenu(e, [
      { label: t('hierarchy.goToDefinition'), disabled: !file, run: () => file && openPinned(file, node.line) },
      {
        label: t('hierarchy.goToInstance'),
        disabled: !node.instance_file,
        run: () => node.instance_file && openPinned(node.instance_file, node.instance_line),
      },
      { separator: true },
      { label: t('hierarchy.copyPath'), run: () => void navigator.clipboard.writeText(path) },
      // Qualquer módulo do projeto pode virar o topo, pelo arquivo que o
      // define; a biblioteca SAPHO e os testbenches pelo nome, não.
      ...(file && !node.library ? [{ separator: true }, topItem(file, isTop, node.module)] : []),
    ]);

  return (
    <>
      <Row
        depth={depth}
        icon={icon}
        label={node.name}
        detail={node.name === node.module ? undefined : node.module}
        title={[path, file ? `${relativeTo(file, root)}${node.line ? `:${node.line}` : ''}` : ''].filter(Boolean).join('\n')}
        badges={node.library ? <Badge>SAPHO</Badge> : isTop ? <Badge tone="accent">{t('explorer.topBadge')}</Badge> : undefined}
        gitPath={file ?? undefined}
        expanded={hasChildren ? open : undefined}
        muted={node.library}
        selected={!!file && samePath(active, file)}
        onClick={() => {
          if (hasChildren) toggle(path);
          if (file) openPreview(file, node.line);
        }}
        onDoubleClick={() => file && openPinned(file, node.line)}
        onContextMenu={menu}
      />
      {open &&
        node.children.map((child) => (
          <InstanceNode
            key={child.name}
            node={child}
            depth={depth + 1}
            path={`${path}.${child.name}`}
            expanded={expanded}
            toggle={toggle}
            root={root}
            processors={processors}
            topModule={topModule}
          />
        ))}
    </>
  );
}

function ElaborationTree({
  elaboration,
  expanded,
  toggle,
}: {
  elaboration: Elaboration;
  expanded: Set<string>;
  toggle: (path: string) => void;
}) {
  const snapshot = useProject((s) => s.snapshot)!;
  const processors = useMemo(() => new Set(snapshot.processors.map((p) => p.name)), [snapshot.processors]);
  if (elaboration.status !== 'succeeded') {
    // O erro do iverilog, com arquivo e linha; clicar abre o arquivo.
    const shown = elaboration.diagnostics.filter((d) => d.severity === 'error' || d.severity === 'unknown');
    const title = elaboration.processor
      ? t('hierarchy.processorTestbench', { name: elaboration.processor })
      : elaboration.testbench
        ? baseName(elaboration.testbench)
        : t('hierarchy.design');
    return (
      <div className="tree__error">
        <div className="tree__error-title">
          <CircleAlert size={13} />
          {title}: {t('hierarchy.failed')}
        </div>
        {shown.length === 0 && <pre>{t(`console.status.${elaboration.status}` as Key)}</pre>}
        {shown.slice(0, 8).map((d, i) => (
          <button
            key={i}
            type="button"
            className="tree__diagnostic"
            disabled={!d.file}
            onClick={() => d.file && openPinned(d.file, d.line)}
          >
            {d.file ? `${relativeTo(d.file, snapshot.root)}${d.line ? `:${d.line}` : ''}: ` : ''}
            {d.message}
          </button>
        ))}
      </div>
    );
  }
  return (
    <>
      {elaboration.roots.map((node) => (
        <InstanceNode
          key={node.name}
          node={node}
          depth={0}
          path={node.name}
          expanded={expanded}
          toggle={toggle}
          root={snapshot.root}
          processors={processors}
          topModule={snapshot.top_module}
        />
      ))}
    </>
  );
}

function HierarchyView({ expanded, toggle }: { expanded: Set<string>; toggle: (path: string) => void }) {
  const t = useT();
  const lang = useLang((s) => s.lang);
  const { data, at, loading, error, stale } = useHierarchy();
  const running = useJobs((s) => s.running !== null);

  // Na primeira vez que a vista abre, elabora.
  useEffect(() => {
    const state = useHierarchy.getState();
    if (!state.data && !state.loading && !state.error && !useJobs.getState().running) void state.refresh();
  }, []);

  if (!data) {
    return (
      <Empty>
        {loading ? (
          <p>
            <Spinner /> {t('hierarchy.elaborating')}
          </p>
        ) : error?.code === 'empty_project' ? (
          // Projeto sem arquivo Verilog não é erro: é o começo.
          <>
            <p>{errorText(error)}</p>
            <button type="button" className="btn btn--sm" onClick={() => void addVerilogFiles()}>
              {t('action.addVerilog')}
            </button>
          </>
        ) : error ? (
          <p className="tree__error-text">{error.message}</p>
        ) : (
          <p>{t('hierarchy.empty')}</p>
        )}
        {!loading && error?.code !== 'empty_project' && (
          <button type="button" className="btn btn--sm" disabled={running} onClick={() => void useHierarchy.getState().refresh()}>
            {t('hierarchy.elaborate')}
          </button>
        )}
      </Empty>
    );
  }

  const when = new Date(at).toLocaleTimeString(lang === 'en' ? 'en' : 'pt-BR');
  return (
    <div className="tree" role="tree" onKeyDown={treeKeys}>
      <p className={`tree__status${stale ? ' is-stale' : ''}`}>
        {loading ? <Spinner /> : null}
        {stale ? t('hierarchy.stale') : t('hierarchy.elaborated', { when, ms: data.duration_ms })}
      </p>
      <Section title={t('hierarchy.design')} count={data.design?.roots.length ?? 0}>
        {data.design ? (
          <ElaborationTree elaboration={data.design} expanded={expanded} toggle={toggle} />
        ) : (
          <p className="tree__empty">{t('hierarchy.noDesign')}</p>
        )}
      </Section>
      <Section title={t('hierarchy.simulation')} count={data.testbenches.length}>
        {data.testbenches.length === 0 && <p className="tree__empty">{t('hierarchy.noTestbench')}</p>}
        {data.testbenches.map((elaboration) => (
          <ElaborationTree
            key={elaboration.processor ?? elaboration.testbench ?? ''}
            elaboration={elaboration}
            expanded={expanded}
            toggle={toggle}
          />
        ))}
      </Section>
    </div>
  );
}

// Arquivos -----------------------------------------------------------------

function DirNode({
  entry,
  depth,
  expanded,
  toggle,
  version,
}: {
  entry: DirEntry;
  depth: number;
  expanded: Set<string>;
  toggle: (path: string) => void;
  version: number;
}) {
  const snapshot = useProject((s) => s.snapshot)!;
  const spfMissing = useProject((s) => s.spfMissing);
  const active = useActivePath();
  const showHidden = useLayout((s) => s.showHidden);
  const [children, setChildren] = useState<DirEntry[] | null>(null);
  const isOpen = entry.is_dir && expanded.has(entry.path);
  const dropDir = entry.is_dir ? entry.path : dirName(entry.path);
  const dropActive = useDrag((s) => s.allowed && entry.is_dir && sameTarget(s.target, { kind: 'dir', path: entry.path }));

  useEffect(() => {
    if (!isOpen) return;
    let cancelled = false;
    api.fs
      .readDir(entry.path)
      .then((list) => !cancelled && setChildren(list))
      .catch(() => !cancelled && setChildren([]));
    return () => {
      cancelled = true;
    };
  }, [isOpen, entry.path, version]);

  const root = snapshot.root;
  // Sem o .spf, o que ele dizia (topo, simulado, registrado) não vale mais, e
  // o menu fica só com as operações de arquivo.
  const project = !spfMissing;
  const isTop = project && samePath(entry.path, snapshot.top_level);
  const isSim = project && samePath(entry.path, snapshot.selected_testbench);
  const registered =
    project &&
    !entry.is_dir &&
    (snapshot.synthesizable.some((f) => samePath(f.path, entry.path)) || snapshot.testbenches.some((f) => samePath(f.path, entry.path)));

  // Renomear passa pelo Core, como mover: um .v registrado continua no .spf.
  const rename = async () => {
    const name = await prompt({ title: t('dialog.rename.title'), initial: entry.name, selectStem: !entry.is_dir });
    if (!name || name === entry.name) return;
    await movePath(entry.path, joinPath(dirName(entry.path), name));
  };
  const trash = async () => {
    const answer = await confirm({
      title: t('common.delete'),
      message: t('dialog.confirmDelete', { name: entry.name }),
      buttons: [
        { label: t('common.delete'), value: 'yes', danger: true },
        { label: t('common.cancel'), value: 'no' },
      ],
    });
    if (answer !== 'yes') return;
    if ((await guarded(() => api.fs.trash(entry.path))) === undefined) return;
    for (const tab of useEditor.getState().tabs) {
      if (tab.path && isInside(tab.path, entry.path)) {
        useEditor.getState().setDirty(tab.path, false);
        await useEditor.getState().closeTab(tab.id);
      }
    }
    useProject.getState().bumpTree();
  };

  const menu = (e: React.MouseEvent) => {
    const dir = entry.is_dir ? entry.path : dirName(entry.path);
    const verilog = project && ['v', 'sv'].includes(extension(entry.path));
    // Um .py entra no projeto como testbench cocotb; topo, nunca.
    const cocotb = project && !entry.is_dir && extension(entry.path) === 'py';
    openContextMenu(e, [
      ...(entry.is_dir ? [] : openItems(entry.path)),
      { label: t('action.newFile'), run: () => void newFileIn(dir) },
      { label: t('action.newFolder'), run: () => void newFolderIn(dir) },
      ...(verilog
        ? [
            { separator: true },
            ...(registered ? [] : [{ label: t('explorer.addToProject'), run: () => void addVerilogFiles([entry.path]) }]),
            {
              label: t('explorer.setTop'),
              disabled: isTop || entry.testbench_name,
              run: () => void setTopFile(entry.path),
            },
          ]
        : []),
      ...(cocotb && !registered
        ? [{ separator: true }, { label: t('explorer.addToProject'), run: () => void addVerilogFiles([entry.path]) }]
        : []),
      { separator: true },
      { label: t('common.rename'), keys: 'F2', run: () => void rename() },
      { label: t('common.delete'), keys: 'Del', danger: true, run: () => void trash() },
      { separator: true },
      ...commonFileItems(entry.path, root),
    ]);
  };

  const source: DragSource = { path: entry.path, isDir: entry.is_dir, movable: true, verilog: false };
  return (
    <>
      <div
        onKeyDown={(e) => {
          if (e.key === 'F2') void rename();
          if (e.key === 'Delete') void trash();
        }}
      >
        <Row
          depth={depth}
          icon={entry.is_dir ? isOpen ? <FolderOpen size={14} /> : <Folder size={14} /> : <FileIcon path={entry.path} />}
          label={entry.name}
          title={relativeTo(entry.path, root)}
          path={entry.path}
          dir={entry.is_dir}
          dropDir={dropDir}
          dropActive={dropActive}
          expanded={entry.is_dir ? isOpen : undefined}
          selected={samePath(active, entry.path)}
          muted={entry.hidden}
          badges={
            isTop ? (
              <Badge tone="accent">{t('explorer.topBadge')}</Badge>
            ) : isSim ? (
              <Badge tone="accent">{t('explorer.simBadge')}</Badge>
            ) : registered ? (
              <span className="tree__dot" title={t('explorer.inProject')} />
            ) : undefined
          }
          onClick={() => (entry.is_dir ? toggle(entry.path) : openPreview(entry.path))}
          onDoubleClick={() => !entry.is_dir && openPinned(entry.path)}
          onContextMenu={menu}
          onPointerDown={(e) => beginDrag(e, source)}
        />
      </div>
      {isOpen &&
        children
          ?.filter((child) => showHidden || !child.hidden)
          .map((child) => (
            <DirNode key={child.path} entry={child} depth={depth + 1} expanded={expanded} toggle={toggle} version={version} />
          ))}
    </>
  );
}

function FilesView({
  expanded,
  toggle,
  expand,
}: {
  expanded: Set<string>;
  toggle: (path: string) => void;
  expand: (paths: string[]) => void;
}) {
  const snapshot = useProject((s) => s.snapshot)!;
  const version = useProject((s) => s.treeVersion);
  const showHidden = useLayout((s) => s.showHidden);
  const active = useActivePath();
  const [entries, setEntries] = useState<DirEntry[]>([]);
  const rootDrop = useDrag((s) => s.allowed && sameTarget(s.target, { kind: 'dir', path: snapshot.root }));
  const container = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    api.fs
      .readDir(snapshot.root)
      .then((list) => !cancelled && setEntries(list))
      .catch(showError);
    return () => {
      cancelled = true;
    };
  }, [snapshot.root, version]);

  // O arquivo ativo aparece na árvore: as pastas dele abrem e a linha rola
  // para a vista, como o "revelar" do VS Code.
  useEffect(() => {
    if (!active || !isInside(active, snapshot.root)) return;
    const ancestors: string[] = [];
    for (let dir = dirName(active); dir.length > snapshot.root.length && isInside(dir, snapshot.root); dir = dirName(dir)) {
      ancestors.push(dir);
    }
    expand(ancestors);
    const timer = window.setTimeout(() => {
      const row = container.current?.querySelector<HTMLElement>(`[data-path="${CSS.escape(active)}"]`);
      row?.scrollIntoView({ block: 'nearest' });
    }, 120);
    return () => window.clearTimeout(timer);
  }, [active, snapshot.root, expand]);

  return (
    <div
      ref={container}
      className={`tree tree--files${rootDrop ? ' is-drop-target' : ''}`}
      role="tree"
      data-drop-dir={snapshot.root}
      onKeyDown={treeKeys}
    >
      {entries
        .filter((entry) => showHidden || !entry.hidden)
        .map((entry) => (
          <DirNode key={entry.path} entry={entry} depth={0} expanded={expanded} toggle={toggle} version={version} />
        ))}
      <div className="tree__spacer" />
    </div>
  );
}

/** O rótulo que segue o ponteiro durante um arrasto dentro da janela. */
function DragGhost() {
  const source = useDrag((s) => s.source);
  const x = useDrag((s) => s.x);
  const y = useDrag((s) => s.y);
  const allowed = useDrag((s) => s.allowed);
  if (!source) return null;
  return (
    <div className={`drag-ghost${allowed ? '' : ' is-denied'}`} style={{ left: x + 12, top: y + 8 }}>
      {baseName(source.path)}
    </div>
  );
}

// O painel -----------------------------------------------------------------

const EXPANDED_KEY = 'aurora-ide:expanded:';

/** Quantas pastas o "expandir tudo" da árvore de arquivos abre, no máximo. */
const EXPAND_ALL_LIMIT = 400;

/** As pastas do projeto, em largura, para o "expandir tudo" da árvore de
 * arquivos. As ocultas só entram se estiverem à mostra. */
async function allDirs(root: string, showHidden: boolean): Promise<string[]> {
  const found: string[] = [];
  const queue = [root];
  while (queue.length > 0 && found.length < EXPAND_ALL_LIMIT) {
    const dir = queue.shift()!;
    const entries = await api.fs.readDir(dir).catch(() => [] as DirEntry[]);
    for (const entry of entries) {
      if (!entry.is_dir || (entry.hidden && !showHidden)) continue;
      found.push(entry.path);
      queue.push(entry.path);
    }
  }
  return found.slice(0, EXPAND_ALL_LIMIT);
}

/** Os nós da hierarquia que o "recolher tudo" (`open` falso) ou o "expandir
 * tudo" põem contra o padrão de cada um (ver InstanceNode). */
function hierarchyFlips(elaborations: Elaboration[], open: boolean): Set<string> {
  const flips = new Set<string>();
  const walk = (node: ModuleInstance, path: string, depth: number) => {
    if (node.children.length === 0) return;
    const defaultOpen = depth < 2 && !node.library;
    if (defaultOpen !== open) flips.add(path);
    for (const child of node.children) walk(child, `${path}.${child.name}`, depth + 1);
  };
  for (const elaboration of elaborations) for (const root of elaboration.roots) walk(root, root.name, 0);
  return flips;
}

function loadExpanded(spf: string | undefined): Set<string> {
  if (!spf) return new Set();
  try {
    return new Set(JSON.parse(localStorage.getItem(EXPANDED_KEY + spf) ?? '[]') as string[]);
  } catch {
    return new Set();
  }
}

const FOLDS: Record<ExplorerMode, { version: number; open: boolean }> = {
  sources: { version: 0, open: true },
  hierarchy: { version: 0, open: true },
  files: { version: 0, open: true },
};

export function Explorer() {
  const t = useT();
  const snapshot = useProject((s) => s.snapshot);
  const spfMissing = useProject((s) => s.spfMissing);
  const mode = useLayout((s) => s.explorerMode);
  const showHidden = useLayout((s) => s.showHidden);
  const hierarchyLoading = useHierarchy((s) => s.loading);
  const running = useJobs((s) => s.running !== null);
  const spf = snapshot?.spf;
  // As pastas abertas da árvore de arquivos, lembradas por projeto.
  const [expanded, setExpanded] = useState<Set<string>>(() => loadExpanded(spf));
  // Os nós da hierarquia que o usuário abriu ou fechou, contra o padrão.
  const [flipped, setFlipped] = useState<Set<string>>(new Set());
  // O último "recolher tudo" ou "expandir tudo" de cada modo (FoldContext).
  const [folds, setFolds] = useState<Record<ExplorerMode, { version: number; open: boolean }>>(FOLDS);

  useEffect(() => {
    setExpanded(loadExpanded(spf));
    setFlipped(new Set());
    setFolds(FOLDS);
  }, [spf]);

  useEffect(() => {
    if (!spf) return;
    try {
      localStorage.setItem(EXPANDED_KEY + spf, JSON.stringify([...expanded]));
    } catch {
      // Só não lembra.
    }
  }, [expanded, spf]);

  const toggle = useCallback((path: string) => {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }, []);
  const expand = useCallback((paths: string[]) => {
    setExpanded((current) => (paths.every((p) => current.has(p)) ? current : new Set([...current, ...paths])));
  }, []);
  const flip = useCallback((path: string) => {
    setFlipped((current) => {
      const next = new Set(current);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }, []);

  if (!snapshot) {
    return (
      <div className="sidebar__body">
        <Empty>
          <p>{t('explorer.noProject')}</p>
        </Empty>
      </div>
    );
  }

  const fold = folds[mode];
  // Na árvore de arquivos, o botão segue as pastas abertas: sem nenhuma,
  // expande; com alguma, recolhe.
  const folded = mode === 'files' ? expanded.size === 0 : !fold.open;
  // O botão alterna: depois de recolher, expande, e vice-versa.
  const foldAll = async (open: boolean) => {
    setFolds((current) => ({ ...current, [mode]: { version: current[mode].version + 1, open } }));
    if (mode === 'files') {
      setExpanded(open ? new Set(await allDirs(snapshot.root, showHidden)) : new Set());
    } else if (mode === 'hierarchy') {
      const data = useHierarchy.getState().data;
      const elaborations = data ? [...(data.design ? [data.design] : []), ...data.testbenches] : [];
      setFlipped(hierarchyFlips(elaborations, open));
    }
  };

  const modes: { id: typeof mode; label: string }[] = [
    { id: 'sources', label: t('explorer.sources') },
    { id: 'hierarchy', label: t('explorer.hierarchy') },
    { id: 'files', label: t('explorer.files') },
  ];

  return (
    <>
      <ViewActions>
        {mode === 'files' && (
          <>
            <IconButton label={t('action.newFile')} onClick={() => void newFileIn()}>
              <FilePlus size={15} />
            </IconButton>
            <IconButton label={t('action.newFolder')} onClick={() => void newFolderIn()}>
              <FolderPlus size={15} />
            </IconButton>
            <IconButton label={t('explorer.showHidden')} active={showHidden} onClick={() => useLayout.getState().setShowHidden(!showHidden)}>
              {showHidden ? <Eye size={15} /> : <EyeOff size={15} />}
            </IconButton>
          </>
        )}
        {mode === 'sources' && !spfMissing && (
          <IconButton label={t('action.addVerilog')} onClick={() => void addVerilogFiles()}>
            <FilePlus size={15} />
          </IconButton>
        )}
        {mode === 'hierarchy' ? (
          <IconButton
            label={t('hierarchy.elaborate')}
            disabled={hierarchyLoading || running || spfMissing}
            onClick={() => void useHierarchy.getState().refresh()}
          >
            {hierarchyLoading ? <Spinner /> : <Boxes size={15} />}
          </IconButton>
        ) : (
          <IconButton
            label={t('action.refreshProject')}
            onClick={() => {
              useProject.getState().bumpTree();
              void useProject.getState().refresh();
            }}
          >
            <RefreshCw size={14} />
          </IconButton>
        )}
        <IconButton label={folded ? t('explorer.expandAll') : t('explorer.collapseAll')} onClick={() => void foldAll(folded)}>
          {folded ? <ChevronsUpDown size={15} /> : <ChevronsDownUp size={15} />}
        </IconButton>
      </ViewActions>
      <div className="sidebar__toolbar">
        <div className="segmented segmented--full" role="tablist">
          {modes.map((m) => (
            <button
              key={m.id}
              type="button"
              role="tab"
              aria-selected={mode === m.id}
              className={mode === m.id ? 'is-active' : ''}
              onClick={() => useLayout.getState().setExplorerMode(m.id)}
            >
              {m.label}
            </button>
          ))}
        </div>
      </div>
      <div className="sidebar__body">
        {/* Sem o .spf, as fontes e a hierarquia (que vêm dele) saem; a árvore
            de arquivos continua, embaixo do aviso. */}
        {spfMissing && <MissingSpfNotice />}
        <FoldContext.Provider value={fold}>
          {mode === 'sources' && !spfMissing && <SourcesView />}
          {mode === 'hierarchy' && !spfMissing && <HierarchyView expanded={flipped} toggle={flip} />}
          {mode === 'files' && <FilesView expanded={expanded} toggle={toggle} expand={expand} />}
        </FoldContext.Provider>
      </div>
      <DragGhost />
    </>
  );
}
