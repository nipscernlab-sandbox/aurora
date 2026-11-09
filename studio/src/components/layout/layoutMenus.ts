// Os menus do layout: o de uma vista (mover para outra região, esconder), o
// de uma região (as vistas dela), o da barra de atividades e os das barras de
// ferramentas e de status (os itens delas). Todos terminam com o submenu
// Aparência, com as barras e as regiões: de qualquer parte da janela que
// esteja à vista se chega de volta às que foram escondidas.

import { action, isEnabled, runAction, TOOLBAR_STYLE_ACTIONS, toolbarItemAvailable } from '../../actions';
import { t, type Key } from '../../i18n';
import { useLayout, type RegionId, type ViewId } from '../../state/layout';
import { availableStatusItems, isViewAvailable, REGION_IDS, regionOf, TOOLBAR_ITEMS, TOOLBAR_STYLES, VIEW_INFO } from '../../state/layoutModel';
import type { MenuItem } from '../common';

/** O item de menu de uma ação de actions.ts, com a marca se ela liga e
 * desliga e o nome curto dos menus. */
export function actionItem(id: string): MenuItem {
  const a = action(id);
  return {
    label: t(a.menuLabel ?? a.label),
    keys: a.keys,
    checked: a.checked?.(),
    disabled: !isEnabled(a),
    run: () => runAction(id),
  };
}

const SEPARATOR: MenuItem = { separator: true };

/** As barras e as regiões, cada uma com a marca de visível. */
export const APPEARANCE_ENTRIES = [
  'toggleMenuBar',
  'toggleToolbar',
  'toggleActivityBar',
  'activityBarRight',
  'toggleStatusBar',
  '-',
  'toggleSidebar',
  'toggleRightSidebar',
  'togglePanel',
  'togglePanelPosition',
  'maximizePanel',
] as const;

/** Os layouts com nome: trocar, salvar, restaurar, personalizar. */
export const LAYOUT_ENTRIES = ['selectLayout', 'saveLayout', 'saveLayoutAs', 'restoreLayout', 'resetLayout', '-', 'customizeLayout'] as const;

export function layoutItems(): MenuItem[] {
  return LAYOUT_ENTRIES.map((id) => (id === '-' ? SEPARATOR : actionItem(id)));
}

export function appearanceItems(): MenuItem[] {
  return APPEARANCE_ENTRIES.map((id) => (id === '-' ? SEPARATOR : actionItem(id)));
}

function appearanceSubmenu(): MenuItem {
  return { label: t('menu.appearance'), submenu: appearanceItems() };
}

const MOVE_LABELS: Record<RegionId, Key> = {
  left: 'layout.moveTo.left',
  right: 'layout.moveTo.right',
  panel: 'layout.moveTo.panel',
};

/** "Mover para" as outras duas regiões. */
export function moveItems(view: ViewId): MenuItem[] {
  const from = regionOf(useLayout.getState().live, view);
  return REGION_IDS.filter((region) => region !== from).map((region) => ({
    label: t(MOVE_LABELS[region]),
    run: () => useLayout.getState().moveView(view, region),
  }));
}

/** O menu de uma aba de vista ou de um ícone da barra de atividades. */
export function viewMenu(view: ViewId): MenuItem[] {
  return [
    ...moveItems(view),
    { label: t('layout.hideView', { view: t(VIEW_INFO[view].label) }), run: () => useLayout.getState().setViewHidden(view, true) },
    SEPARATOR,
    appearanceSubmenu(),
  ];
}

/** As vistas de uma região, marcadas as que aparecem: desmarcar esconde.
 * As de recurso experimental desligado ficam de fora. */
function viewChecklist(region: RegionId): MenuItem[] {
  const { live } = useLayout.getState();
  return live.regions[region].views.filter(isViewAvailable).map((view) => {
    const shown = !live.hidden.views.includes(view);
    return {
      label: t(VIEW_INFO[view].label),
      checked: shown,
      run: () => useLayout.getState().setViewHidden(view, shown),
    };
  });
}

const HIDE_REGION: Record<RegionId, Key> = {
  left: 'layout.hide.left',
  right: 'layout.hide.right',
  panel: 'panel.hide',
};

/** O menu do cabeçalho de uma região, fora das abas. */
export function regionMenu(region: RegionId): MenuItem[] {
  const items = viewChecklist(region);
  if (items.length) items.push(SEPARATOR);
  if (region === 'panel') items.push(actionItem('togglePanelPosition'), actionItem('maximizePanel'));
  items.push({ label: t(HIDE_REGION[region]), run: () => useLayout.getState().setRegionVisible(region, false) });
  items.push(SEPARATOR, appearanceSubmenu());
  return items;
}

/** O menu da barra de atividades, fora dos ícones. */
export function activityBarMenu(side: 'left' | 'right'): MenuItem[] {
  const items = viewChecklist(side);
  if (items.length) items.push(SEPARATOR);
  items.push(actionItem('activityBarRight'), { ...actionItem('toggleActivityBar'), label: t('layout.hideActivityBar'), checked: undefined });
  items.push(SEPARATOR, appearanceSubmenu());
  return items;
}

/** Os itens de uma barra, marcados os que aparecem: desmarcar esconde. */
function itemChecklist(bar: 'toolbar' | 'statusbar', list: { id: string; label: Key }[]): MenuItem[] {
  const hidden = useLayout.getState().live.hidden[bar];
  return list.map((item) => {
    const shown = !hidden.includes(item.id);
    return {
      label: t(item.label),
      checked: shown,
      run: () => useLayout.getState().setItemHidden(bar, item.id, shown),
    };
  });
}

/** O menu da barra de ferramentas, em qualquer modelo: os itens dele, os
 * modelos (layoutModel.ts, TOOLBAR_STYLES), com a marca no do layout em uso,
 * e as barras. */
export function toolbarMenu(): MenuItem[] {
  const style = useLayout.getState().live.bars.toolbarStyle;
  const items = TOOLBAR_ITEMS[style].filter((item) => toolbarItemAvailable(item.id));
  return [
    ...itemChecklist('toolbar', items),
    SEPARATOR,
    { label: t('layout.toolbarModel'), submenu: TOOLBAR_STYLES.map((style) => actionItem(TOOLBAR_STYLE_ACTIONS[style])) },
    actionItem('toggleToolbar'),
    SEPARATOR,
    appearanceSubmenu(),
  ];
}

export function statusBarMenu(): MenuItem[] {
  return [...itemChecklist('statusbar', availableStatusItems()), SEPARATOR, actionItem('toggleStatusBar'), SEPARATOR, appearanceSubmenu()];
}

/** O menu da barra de menus e o da barra de abas do editor: só as barras e
 * as regiões, sem submenu. */
export function chromeMenu(): MenuItem[] {
  return appearanceItems();
}
