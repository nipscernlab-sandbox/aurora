// Uma região da janela: a barra lateral esquerda, a direita ou o painel. Ela
// mostra uma das vistas que o layout pôs nela, e o cabeçalho tem dois jeitos:
//
// - título (`title`): a barra lateral do lado da barra de atividades, que é
//   quem escolhe a vista; o cabeçalho mostra o nome dela e o do projeto,
//   como antes dos layouts;
// - abas (`tabs`): o painel e a barra lateral do outro lado. No painel as
//   abas têm o nome da vista; numa barra lateral, só o ícone, porque onze
//   abas com texto não cabem nela.
//
// Os botões da vista (ViewActions.tsx) vêm antes dos da região.

import { ChevronDown, Maximize2, Minimize2, X } from 'lucide-react';
import { useRef, useState } from 'react';

import { action } from '../../actions';
import { useT, type Key } from '../../i18n';
import { useLayout, type RegionId, type ViewId } from '../../state/layout';
import { VIEW_INFO, visibleViews } from '../../state/layoutModel';
import { useProject } from '../../state/project';
import { Empty, IconButton, openContextMenu } from '../common';
import { regionMenu, viewMenu } from './layoutMenus';
import { ViewBadge, ViewContent, ViewIcon, viewKind } from './viewCatalog';
import { ActionsSlot } from './ViewActions';

export type RegionMode = 'title' | 'tabs';

const HIDE_LABELS: Record<RegionId, Key> = {
  left: 'layout.hide.left',
  right: 'layout.hide.right',
  panel: 'panel.hide',
};

/** O nome da vista com o atalho que a revela, para a dica. */
export function viewTitle(t: (key: Key) => string, view: ViewId): string {
  const keys = action(VIEW_INFO[view].action).keys;
  return keys ? `${t(VIEW_INFO[view].label)} (${keys})` : t(VIEW_INFO[view].label);
}

function ViewTab({ region, view, active }: { region: RegionId; view: ViewId; active: boolean }) {
  const t = useT();
  const select = () => useLayout.getState().revealView(view, { explicit: true });
  const menu = (e: React.MouseEvent) => {
    e.stopPropagation();
    openContextMenu(e, viewMenu(view));
  };
  if (region === 'panel') {
    return (
      <button
        type="button"
        role="tab"
        aria-selected={active}
        className={`panel__tab${active ? ' is-active' : ''}`}
        data-view-tab={view}
        onClick={select}
        onContextMenu={menu}
      >
        {t(VIEW_INFO[view].label)}
        <ViewBadge view={view} />
      </button>
    );
  }
  return (
    <button
      type="button"
      role="tab"
      aria-selected={active}
      aria-label={t(VIEW_INFO[view].label)}
      title={viewTitle(t, view)}
      className={`panel__tab panel__tab--icon${active ? ' is-active' : ''}`}
      data-view-tab={view}
      onClick={select}
      onContextMenu={menu}
    >
      <ViewIcon view={view} size={16} strokeWidth={1.7} />
      <ViewBadge view={view} compact />
    </button>
  );
}

export function Region({ region, mode }: { region: RegionId; mode: RegionMode }) {
  const t = useT();
  const live = useLayout((s) => s.live);
  const maximized = useLayout((s) => s.panelMaximized);
  const project = useProject((s) => s.snapshot?.name);
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  const views = visibleViews(live, region);
  const active = live.regions[region].active;
  const side = region !== 'panel';
  // A vista que entra no lugar de outra, com a região já aberta, entra com
  // um esmaecer (app.css); a da região que acabou de abrir, não: abrir e
  // fechar regiões é na hora.
  const shownView = useRef(active);
  const switched = useRef(false);
  if (shownView.current !== active) {
    switched.current = shownView.current != null;
    shownView.current = active;
  }
  const menu = (e: React.MouseEvent) => openContextMenu(e, regionMenu(region));

  const header =
    mode === 'title' ? (
      <header className="sidebar__header" onContextMenu={menu}>
        <span className="sidebar__title">{active ? t(VIEW_INFO[active].label) : ''}</span>
        {project && <span className="sidebar__project">{project}</span>}
        <div className="sidebar__actions" ref={setSlot} />
      </header>
    ) : (
      <div className="panel__header" onContextMenu={menu}>
        <div className="panel__tabs" role="tablist">
          {views.map((view) => (
            <ViewTab key={view} region={region} view={view} active={view === active} />
          ))}
        </div>
        <div className="panel__actions">
          <div className="region__slot" ref={setSlot} />
          {region === 'panel' && (
            <IconButton label={t('panel.maximize')} onClick={() => useLayout.getState().setPanelMaximized(!maximized)}>
              {maximized ? <Minimize2 size={14} /> : <Maximize2 size={14} />}
            </IconButton>
          )}
          <IconButton label={t(HIDE_LABELS[region])} onClick={() => useLayout.getState().setRegionVisible(region, false)}>
            {side ? <X size={14} /> : <ChevronDown size={15} />}
          </IconButton>
        </div>
      </div>
    );

  return (
    <aside className={`region region--${region} ${side ? 'sidebar' : 'panel'}`} data-region={region}>
      {header}
      <ActionsSlot.Provider value={slot}>
        <div className="region__body">
          {active ? (
            <div
              key={active}
              className={`region__view region__view--${viewKind(active)}${switched.current ? ' region__view--troca' : ''}`}
              data-view={active}
            >
              <ViewContent view={active} />
            </div>
          ) : (
            <Empty>{t('layout.emptyRegion')}</Empty>
          )}
        </div>
      </ActionsSlot.Provider>
    </aside>
  );
}
