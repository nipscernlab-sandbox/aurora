// A barra de status: o projeto (o ramo do git, topo, testbench, alvo) à
// esquerda, a operação rodando ou o último resultado no meio, e à direita os
// problemas, o cursor, a linguagem, o simulador, o bundle e os botões das
// regiões (a barra lateral esquerda, o painel e a direita).
//
// Cada item pode ser escondido pelo layout (o menu de contexto da barra, ou
// Preferências > Layout da janela). A espera de um atalho de duas etapas e a
// linha do Vim aparecem sempre: sem elas, o teclado parece não responder. O
// aviso de AURORA mais nova também, até a atualização.

import {
  CircleAlert,
  CircleArrowUp,
  GitBranch,
  PanelBottom,
  PanelBottomDashed,
  PanelLeft,
  PanelLeftDashed,
  PanelRight,
  PanelRightDashed,
  RotateCw,
  SquareSplitHorizontal,
  TriangleAlert,
  type LucideIcon,
} from 'lucide-react';
import { useEffect, useState } from 'react';

import { action, runAction, useChord } from '../../actions';
import { languageLabel } from '../../editor/monaco';
import { setVimStatusNode } from '../../editor/vim';
import { formatDuration, useT, type Key } from '../../i18n';
import { useApp } from '../../state/app';
import { useEditor } from '../../state/editor';
import { useFeature } from '../../state/experimental';
import { useGit } from '../../state/git';
import { lastFlowKey, phaseKey, useJobs } from '../../state/jobs';
import { useLayout } from '../../state/layout';
import { useProject } from '../../state/project';
import { canUpdate, finishPending, offerUpdate, useUpdates } from '../../state/updates';
import { baseName } from '../../util/paths';
import { Kbd, openContextMenu, Spinner } from '../common';
import { statusBarMenu } from './layoutMenus';

export function Elapsed({ since }: { since: number }) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 500);
    return () => window.clearInterval(timer);
  }, []);
  return <span className="mono">{formatDuration(Math.max(0, now - since))}</span>;
}

/** Mostrar e esconder a barra lateral esquerda, o painel e a direita, como
 * os botões do canto da barra de título do VS Code. O ícone tracejado e
 * apagado é o da região escondida; o do painel acompanha a posição dele. */
function RegionToggles() {
  const t = useT();
  // Redesenha quando uma região aparece ou some (`checked` lê o estado).
  useLayout((s) => s.live.regions);
  const panelRight = useLayout((s) => s.live.panelPosition === 'right');
  const toggles: { id: string; on: LucideIcon; off: LucideIcon }[] = [
    { id: 'toggleSidebar', on: PanelLeft, off: PanelLeftDashed },
    panelRight
      ? { id: 'togglePanel', on: SquareSplitHorizontal, off: SquareSplitHorizontal }
      : { id: 'togglePanel', on: PanelBottom, off: PanelBottomDashed },
    { id: 'toggleRightSidebar', on: PanelRight, off: PanelRightDashed },
  ];
  return (
    <div className="statusbar__regions">
      {toggles.map(({ id, on, off }) => {
        const a = action(id);
        const shown = !!a.checked?.();
        const Icon = shown ? on : off;
        return (
          <button
            key={id}
            type="button"
            className={`statusbar__button statusbar__region${shown ? ' is-on' : ''}`}
            aria-pressed={shown}
            aria-label={t(a.label)}
            title={`${t(a.label)}${a.keys ? ` (${a.keys})` : ''}`}
            onClick={() => runAction(id)}
          >
            <Icon size={14} />
          </button>
        );
      })}
    </div>
  );
}

/** O ramo do git do projeto, com os commits por enviar (↑) e por trazer
 * (↓). O clique mostra a vista Controle de versão. Só com o recurso ligado e
 * o projeto num repositório. */
function GitItem() {
  const t = useT();
  const on = useFeature('git');
  const branch = useGit((s) => s.status?.repository?.branch ?? null);
  const syncing = useGit((s) => s.busy === 'push' || s.busy === 'pull' || s.busy === 'fetch' || s.busy === 'remote' || s.busy === 'switch');
  if (!on || !branch) return null;
  const name = branch.name ?? `(${branch.commit?.slice(0, 7) ?? t('git.branch.noCommit')})`;
  const counts = [branch.ahead ? `↑${branch.ahead}` : '', branch.behind ? `↓${branch.behind}` : ''].filter(Boolean).join(' ');
  const title = [
    branch.name ? t('git.branch.current', { branch: branch.name }) : t('git.branch.detached'),
    branch.upstream ? t('git.status.tracks', { upstream: branch.upstream, ahead: branch.ahead, behind: branch.behind }) : t('git.branch.unpublished'),
  ].join('\n');
  return (
    <button
      type="button"
      className="statusbar__item statusbar__button statusbar__git"
      title={title}
      onClick={() => useLayout.getState().revealView('git', { explicit: true })}
    >
      {syncing ? <Spinner size={12} /> : <GitBranch size={12} />}
      <span className="statusbar__git-name">{name}</span>
      {counts && <span className="mono">{counts}</span>}
    </button>
  );
}

/** AURORA mais nova publicado: o clique oferece a atualização. Depois dela,
 * se o Studio não conseguiu sair (um arquivo que não gravou), o clique
 * reinicia ou fecha. */
function UpdateIndicator() {
  const t = useT();
  const report = useUpdates((s) => s.report);
  const pending = useUpdates((s) => s.pending);
  const updating = useJobs((s) => s.running?.flow === 'update');
  if (updating) return null;
  if (pending) {
    return (
      <button type="button" className="statusbar__item statusbar__button statusbar__update" onClick={finishPending}>
        <RotateCw size={12} />
        {t(pending.mode === 'restart' ? 'update.restartPending' : 'update.quitPending')}
      </button>
    );
  }
  if (!canUpdate(report)) return null;
  return (
    <button
      type="button"
      className="statusbar__item statusbar__button statusbar__update"
      title={t('update.availableHint', { installed: report.aurora.installed })}
      onClick={() => void offerUpdate()}
    >
      <CircleArrowUp size={12} />
      {t('update.available', { version: report.aurora.latest })}
    </button>
  );
}

export function StatusBar() {
  const t = useT();
  const snapshot = useProject((s) => s.snapshot);
  const spfMissing = useProject((s) => s.spfMissing);
  const rootMissing = useProject((s) => s.rootMissing);
  const target = useProject((s) => s.target);
  const running = useJobs((s) => s.running);
  const last = useJobs((s) => s.last);
  const problems = useJobs((s) => s.problems);
  const cursor = useEditor((s) => s.cursor);
  const languageId = useEditor((s) => s.languageId);
  const activeIsFile = useEditor((s) => s.tabs.find((tab) => tab.id === s.activeId)?.kind === 'file');
  const simulator = useApp((s) => s.settings?.simulator ?? 'icarus');
  const toolchain = useApp((s) => s.toolchain);
  const vim = useApp((s) => s.settings?.editor.vim_mode ?? false);
  const pending = useChord((s) => s.pending);
  const hidden = useLayout((s) => s.live.hidden.statusbar);
  const shown = (id: string) => !hidden.includes(id);

  const errors = problems.filter((p) => p.severity === 'error').length;
  const warnings = problems.filter((p) => p.severity === 'warning').length;

  return (
    <footer className="statusbar" onContextMenu={(e) => openContextMenu(e, statusBarMenu())}>
      <div className="statusbar__left">
        {snapshot ? (
          <>
            {shown('project') &&
              (spfMissing ? (
                <span
                  className="statusbar__item statusbar__project statusbar__project--missing"
                  title={
                    rootMissing
                      ? t('status.rootMissing', { path: snapshot.root })
                      : t('status.spfMissing', { file: baseName(snapshot.spf) })
                  }
                >
                  <TriangleAlert size={12} />
                  {snapshot.name}
                </span>
              ) : (
                <span className="statusbar__item statusbar__project">{snapshot.name}</span>
              ))}
            {shown('git') && <GitItem />}
            {shown('top') && (
              <span className="statusbar__item" title={snapshot.top_level ?? undefined}>
                {snapshot.top_module ? t('status.top', { name: snapshot.top_module }) : t('status.noTop')}
              </span>
            )}
            {shown('testbench') && (
              <span className="statusbar__item" title={snapshot.selected_testbench ?? undefined}>
                {snapshot.selected_testbench
                  ? t('status.testbench', { name: snapshot.testbench_module ?? baseName(snapshot.selected_testbench) })
                  : t('status.noTestbench')}
              </span>
            )}
            {target && shown('target') && (
              <span className="statusbar__item">
                {t('toolbar.target')}: {target}
              </span>
            )}
          </>
        ) : (
          shown('project') && <span className="statusbar__item">{t('status.noProject')}</span>
        )}
      </div>

      <div className="statusbar__center">
        {pending ? (
          <span className="statusbar__item">
            <Kbd keys={pending} />
            {t('status.chordPending')}
          </span>
        ) : !shown('operation') ? null : running ? (
          <span className="statusbar__item">
            <Spinner size={12} />
            {running.phase ? t(phaseKey(running.phase, running.statusKey)) : t('common.running')}
            <Elapsed since={running.startedAt} />
          </span>
        ) : last ? (
          <span className={`statusbar__item ${last.succeeded ? 'text-ok' : 'text-error'}`}>
            {last.succeeded
              ? t('status.lastOk', { flow: t(lastFlowKey(last)), time: formatDuration(last.durationMs) })
              : t('status.lastFailed', {
                  flow: t(lastFlowKey(last)),
                  status: last.status === 'error' ? t('console.status.failed') : t(`console.status.${last.status}` as Key),
                })}
          </span>
        ) : (
          <span className="statusbar__item muted">{t('status.idle')}</span>
        )}
      </div>

      <div className="statusbar__right">
        {shown('problems') && (
          <button type="button" className="statusbar__item statusbar__button" onClick={() => useLayout.getState().revealView('problems', { explicit: true })}>
            <CircleAlert size={12} /> {errors}
            <TriangleAlert size={12} /> {warnings}
          </button>
        )}
        {/* O modo do Vim e a linha de comando (`:w`, `/busca`), escritos pelo monaco-vim. */}
        <span ref={setVimStatusNode} className={`statusbar__item statusbar__vim${vim && activeIsFile ? '' : ' is-hidden'}`} />
        {activeIsFile && cursor && shown('cursor') && (
          <span className="statusbar__item">{t('status.cursor', { line: cursor.line, col: cursor.column })}</span>
        )}
        {activeIsFile && languageId && shown('language') && <span className="statusbar__item">{languageLabel(languageId)}</span>}
        {shown('simulator') && <span className="statusbar__item">{simulator === 'icarus' ? 'Icarus' : 'Verilator'}</span>}
        <UpdateIndicator />
        {shown('bundle') && (
          <button
            type="button"
            className={`statusbar__item statusbar__button${toolchain && !toolchain.found ? ' text-warn' : ''}`}
            onClick={() => useEditor.getState().openView('toolchain')}
          >
            {toolchain?.found ? t('status.bundle', { bundle: toolchain.bundle ?? '' }) : toolchain ? t('status.toolchainMissing') : ''}
          </button>
        )}
        {shown('regions') && <RegionToggles />}
      </div>
    </footer>
  );
}
