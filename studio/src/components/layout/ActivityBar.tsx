// A barra de atividades, à esquerda ou à direita da janela: em cima, as
// vistas da barra lateral do mesmo lado (a que o layout pôs ali); embaixo, as
// vistas marcadas para ficar lá (o Perfil, como as Contas do VS Code),
// ferramentas e preferências. Clicar na vista que já está na frente esconde
// a barra lateral, como no VS Code.

import { Settings, Wrench } from 'lucide-react';

import { useT } from '../../i18n';
import { useApp } from '../../state/app';
import { useEditor } from '../../state/editor';
import { useLayout } from '../../state/layout';
import { VIEW_INFO, visibleViews, type ViewId } from '../../state/layoutModel';
import { openContextMenu } from '../common';
import { activityBarMenu, viewMenu } from './layoutMenus';
import { viewTitle } from './Region';
import { ViewBadge, ViewIcon } from './viewCatalog';

export function ActivityBar({ side }: { side: 'left' | 'right' }) {
  const t = useT();
  const live = useLayout((s) => s.live);
  const toolchainMissing = useApp((s) => s.toolchain !== null && !s.toolchain.found);
  const region = live.regions[side];
  const views = visibleViews(live, side);

  const item = (view: ViewId) => (
    <button
      key={view}
      type="button"
      className={`activitybar__item${region.visible && region.active === view ? ' is-active' : ''}`}
      title={viewTitle(t, view)}
      aria-label={t(VIEW_INFO[view].label)}
      data-view-tab={view}
      onClick={() => useLayout.getState().toggleView(view)}
      onContextMenu={(e) => {
        e.stopPropagation();
        openContextMenu(e, viewMenu(view));
      }}
    >
      <ViewIcon view={view} size={20} strokeWidth={1.6} />
      <ViewBadge view={view} compact />
    </button>
  );

  return (
    <nav className={`activitybar activitybar--${side}`} onContextMenu={(e) => openContextMenu(e, activityBarMenu(side))}>
      {views.filter((view) => !VIEW_INFO[view].bottom).map(item)}
      <div className="activitybar__spacer" />
      {views.filter((view) => VIEW_INFO[view].bottom).map(item)}
      <button
        type="button"
        className="activitybar__item"
        title={t('sidebar.toolchain')}
        aria-label={t('sidebar.toolchain')}
        onClick={() => useEditor.getState().openView('toolchain')}
      >
        <Wrench size={19} strokeWidth={1.6} />
        {toolchainMissing && <span className="activitybar__alert" aria-hidden />}
      </button>
      <button
        type="button"
        className="activitybar__item"
        title={`${t('sidebar.settings')} (Ctrl+,)`}
        aria-label={t('sidebar.settings')}
        onClick={() => useEditor.getState().openView('settings')}
      >
        <Settings size={19} strokeWidth={1.6} />
      </button>
    </nav>
  );
}
