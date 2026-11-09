// Preferências > Layout da janela: o layout em uso e o arranjo da janela
// agora. As mudanças valem na hora; "Salvar" as grava no layout em uso, como
// no resto do Studio.
//
// Para não pesar: do layout em uso, só o que serve agora fica à vista (Salvar
// e Restaurar aparecem quando a janela está diferente dele; o resto vai no
// menu ⋯); o modelo da barra de ferramentas se escolhe por um desenho de
// cada um, e a lista de itens embaixo é a do modelo escolhido; as vistas
// aparecem agrupadas por região, como na janela, cada uma com o olho e o
// menu dela; e os itens das barras ficam recolhidos. As vistas e os itens de
// recurso experimental desligado não aparecem.

import { Eye, EyeOff, MoreHorizontal } from 'lucide-react';
import type { MouseEvent } from 'react';

import { runAction, TOOLBAR_STYLE_ACTIONS, toolbarItemAvailable } from '../../actions';
import { useT, type Key } from '../../i18n';
import { useApp } from '../../state/app';
import { useLayout, type RegionId, type ViewId } from '../../state/layout';
import {
  availableStatusItems,
  isViewAvailable,
  REGION_IDS,
  REGION_LABELS,
  TOOLBAR_ITEMS,
  TOOLBAR_STYLES,
  VIEW_INFO,
  type ActivityBarSide,
  type PanelPosition,
  type ToolbarStyle,
} from '../../state/layoutModel';
import {
  applyLayout,
  deleteLayout,
  namedLayouts,
  renameLayout,
  resetLayout,
  restoreLayout,
  saveLayout,
  saveLayoutAs,
  useLayoutStatus,
} from '../../state/savedLayouts';
import { Button, Checkbox, IconButton, openContextMenu, Segmented, Switch, type MenuItem } from '../common';
import { moveItems } from '../layout/layoutMenus';
import { VIEW_ICONS } from '../layout/viewCatalog';
import { SettingRow, SettingsGroup } from './settingsParts';

/** Abre um menu logo abaixo do botão que o pediu. */
function menuBelow(e: MouseEvent<HTMLElement>, items: MenuItem[]) {
  const rect = e.currentTarget.getBoundingClientRect();
  openContextMenu({ clientX: rect.left, clientY: rect.bottom + 4, preventDefault: () => e.preventDefault() }, items);
}

/** Uma vista da lista. `before` e `after` são as vizinhas na lista, que pode
 * pular vistas da região (as de recurso experimental desligado): subir põe a
 * vista antes de `before`, e descer, depois de `after`. */
function ViewItem({ view, region, before, after }: { view: ViewId; region: RegionId; before: ViewId | null; after: ViewId | null }) {
  const t = useT();
  const hidden = useLayout((s) => s.live.hidden.views.includes(view));
  const name = t(VIEW_INFO[view].label);
  const Icon = VIEW_ICONS[view];
  const moveNextTo = (neighbor: ViewId, offset: 0 | 1) => {
    // A posição conta sem a própria vista, como `moveView` espera.
    const rest = useLayout.getState().live.regions[region].views.filter((v) => v !== view);
    useLayout.getState().moveView(view, region, rest.indexOf(neighbor) + offset);
  };
  return (
    <li className={`settings-view${hidden ? ' is-hidden' : ''}`}>
      <Icon size={14} strokeWidth={1.7} className="settings-view__icon" aria-hidden />
      <span className="settings-view__name">{name}</span>
      <IconButton
        label={t(hidden ? 'layout.showView' : 'layout.hideView', { view: name })}
        aria-pressed={!hidden}
        onClick={() => useLayout.getState().setViewHidden(view, !hidden)}
      >
        {hidden ? <EyeOff size={14} /> : <Eye size={14} />}
      </IconButton>
      <IconButton
        label={t('settings.layout.viewMenu', { view: name })}
        onClick={(e) =>
          menuBelow(e, [
            ...moveItems(view),
            { separator: true },
            { label: t('settings.layout.up'), disabled: !before, run: () => before && moveNextTo(before, 0) },
            { label: t('settings.layout.down'), disabled: !after, run: () => after && moveNextTo(after, 1) },
          ])
        }
      >
        <MoreHorizontal size={14} />
      </IconButton>
    </li>
  );
}

/** Um modelo da barra de ferramentas: um desenho da janela com o lugar dos
 * botões do fluxo marcado, o nome e o que ele faz. Escolher um modelo mostra
 * a barra, se estava escondida. */
function ToolbarModel({ style, selected }: { style: ToolbarStyle; selected: boolean }) {
  const t = useT();
  return (
    <button
      type="button"
      role="radio"
      aria-checked={selected}
      className={`toolbar-model${selected ? ' is-selected' : ''}`}
      onClick={() => runAction(TOOLBAR_STYLE_ACTIONS[style])}
    >
      <span className={`toolbar-sketch toolbar-sketch--${style}`} aria-hidden>
        <span className="toolbar-sketch__title">
          <span className="toolbar-sketch__menus" />
          {style === 'titlebar' && <span className="toolbar-sketch__run" />}
        </span>
        {style === 'classic' && (
          <span className="toolbar-sketch__tools">
            <span className="toolbar-sketch__run" />
          </span>
        )}
        <span className="toolbar-sketch__body">
          <span className="toolbar-sketch__side" />
          <span className="toolbar-sketch__editor">
            <span className="toolbar-sketch__tabs">{style === 'editor' && <span className="toolbar-sketch__run" />}</span>
          </span>
        </span>
      </span>
      <span className="toolbar-model__name">{t(`layout.toolbar.${style}`)}</span>
      <span className="toolbar-model__hint">{t(`layout.toolbar.${style}.hint`)}</span>
    </button>
  );
}

/** Os itens de uma barra, recolhidos: a lista só abre quando se quer mexer. */
function BarItems({ bar, title, items }: { bar: 'toolbar' | 'statusbar'; title: string; items: { id: string; label: Key }[] }) {
  const t = useT();
  const hidden = useLayout((s) => s.live.hidden[bar]);
  const shown = items.filter((item) => !hidden.includes(item.id)).length;
  return (
    <details className="settings-details">
      <summary>
        <span className="settings-details__title">{title}</span>
        <span className="settings-details__count">{t('settings.layout.itemsShown', { shown, total: items.length })}</span>
      </summary>
      <div className="settings-details__body">
        {items.map((item) => (
          <Checkbox
            key={item.id}
            checked={!hidden.includes(item.id)}
            onChange={(value) => useLayout.getState().setItemHidden(bar, item.id, !value)}
            label={t(item.label)}
          />
        ))}
      </div>
    </details>
  );
}

export function LayoutSettings() {
  const t = useT();
  const settings = useApp((s) => s.settings);
  const live = useLayout((s) => s.live);
  const zen = useLayout((s) => s.zen);
  const { layout, dirty } = useLayoutStatus();
  const layouts = namedLayouts(settings);
  const state = useLayout.getState;

  const status = dirty ? t('settings.layout.modified') : layout.readOnly ? t('settings.layout.readOnly') : t('settings.layout.saved');
  const bar = (key: 'menubar' | 'toolbar' | 'statusbar', label: string) => (
    <SettingRow label={label}>{(id) => <Switch id={id} checked={live.bars[key]} onChange={(value) => state().setBar(key, value)} />}</SettingRow>
  );
  // As Preferências são uma aba larga: as regiões estão escondidas agora, e
  // as chaves mostram e mudam as que voltam ao sair delas.
  const wideRegions = useLayout((s) => s.wideSaved?.visible ?? null);
  const region = (id: RegionId) => (
    <SettingRow key={id} label={t(REGION_LABELS[id])}>
      {(control) => (
        <Switch
          id={control}
          checked={wideRegions ? wideRegions[id] : live.regions[id].visible}
          onChange={(value) => state().setLayoutRegionVisible(id, value)}
        />
      )}
    </SettingRow>
  );

  return (
    <>
      <p className="settings__intro">{t('settings.layout.hint')}</p>
      {zen && <p className="settings__note">{t('settings.layout.zen')}</p>}
      <fieldset className="settings-fieldset" disabled={zen}>
        <SettingsGroup>
          <SettingRow label={t('settings.layout.active')} hint={status} stack>
            {(id) => (
              <div className="settings-inline">
                <select id={id} className="select" value={layout.id} onChange={(e) => void applyLayout(e.target.value)}>
                  {layouts.map((candidate) => (
                    <option key={candidate.id} value={candidate.id}>
                      {candidate.preset ? `${candidate.name} (${t('layout.tag.preset')})` : candidate.name}
                    </option>
                  ))}
                </select>
                {dirty && (
                  <Button variant="primary" onClick={() => void saveLayout()}>
                    {layout.readOnly ? t('settings.layout.saveAs') : t('common.save')}
                  </Button>
                )}
                {dirty && <Button onClick={restoreLayout}>{t('settings.layout.restore')}</Button>}
                <IconButton
                  label={t('settings.layout.more')}
                  onClick={(e) =>
                    menuBelow(e, [
                      { label: t('settings.layout.saveAs'), run: () => void saveLayoutAs() },
                      { label: t('settings.layout.rename'), disabled: layout.readOnly, run: () => void renameLayout(layout.id) },
                      { label: t('settings.layout.delete'), disabled: layout.readOnly, danger: true, run: () => void deleteLayout(layout.id) },
                      { separator: true },
                      { label: t('action.resetLayout'), run: () => void resetLayout() },
                    ])
                  }
                >
                  <MoreHorizontal size={15} />
                </IconButton>
              </div>
            )}
          </SettingRow>
        </SettingsGroup>

        <SettingsGroup title={t('settings.layout.toolbar')} hint={t('settings.layout.toolbarHint')}>
          <div className="toolbar-models" role="radiogroup" aria-label={t('layout.toolbarModel')}>
            {TOOLBAR_STYLES.map((style) => (
              <ToolbarModel key={style} style={style} selected={live.bars.toolbarStyle === style} />
            ))}
          </div>
          {bar('toolbar', t('settings.layout.toolbarShow'))}
          <BarItems
            bar="toolbar"
            title={t('settings.layout.toolbarItems')}
            items={TOOLBAR_ITEMS[live.bars.toolbarStyle].filter((item) => toolbarItemAvailable(item.id))}
          />
        </SettingsGroup>

        <SettingsGroup title={t('settings.layout.bars')}>
          {bar('menubar', t('layout.bar.menubar'))}
          {bar('statusbar', t('layout.bar.statusbar'))}
          <SettingRow label={t('layout.bar.activitybar')}>
            {() => (
              <Segmented
                label={t('layout.bar.activitybar')}
                value={live.bars.activitybar}
                options={[
                  { value: 'left' as ActivityBarSide, label: t('settings.layout.side.left') },
                  { value: 'right' as ActivityBarSide, label: t('settings.layout.side.right') },
                  { value: 'hidden' as ActivityBarSide, label: t('settings.layout.side.hidden') },
                ]}
                onChange={(value) => state().setBar('activitybar', value)}
              />
            )}
          </SettingRow>
        </SettingsGroup>

        <SettingsGroup title={t('settings.layout.regions')} hint={wideRegions ? t('settings.layout.regionsWide') : undefined}>
          {REGION_IDS.map(region)}
          <SettingRow label={t('settings.layout.panelPosition')}>
            {() => (
              <Segmented
                label={t('settings.layout.panelPosition')}
                value={live.panelPosition}
                options={[
                  { value: 'bottom' as PanelPosition, label: t('settings.layout.panel.bottom') },
                  { value: 'right' as PanelPosition, label: t('settings.layout.panel.right') },
                ]}
                onChange={(value) => state().setPanelPosition(value)}
              />
            )}
          </SettingRow>
        </SettingsGroup>

        <SettingsGroup title={t('settings.layout.views')} hint={t('settings.layout.viewsHint')}>
          <div className="settings-views">
            {REGION_IDS.map((id) => {
              const views = live.regions[id].views.filter(isViewAvailable);
              return (
                <section key={id} className="settings-views__region" aria-label={t(REGION_LABELS[id])}>
                  <h4 className="settings-views__title">{t(REGION_LABELS[id])}</h4>
                  {views.length === 0 ? (
                    <p className="settings-views__empty">{t('settings.layout.noViews')}</p>
                  ) : (
                    <ul className="settings-views__list">
                      {views.map((view, index) => (
                        <ViewItem key={view} view={view} region={id} before={views[index - 1] ?? null} after={views[index + 1] ?? null} />
                      ))}
                    </ul>
                  )}
                </section>
              );
            })}
          </div>
        </SettingsGroup>

        <SettingsGroup title={t('settings.layout.items')} hint={t('settings.layout.itemsHint')}>
          <BarItems bar="statusbar" title={t('layout.bar.statusbar')} items={availableStatusItems()} />
        </SettingsGroup>
      </fieldset>
    </>
  );
}
