// A janela inteira: a barra de título integrada (TitleBar.tsx) ou a barra de
// menus, barra de ferramentas, a área de trabalho (Workbench.tsx: barra de
// atividades, barras laterais, editor e painel) e barra de status. Aqui
// também ficam as ligações globais: atalhos, vigia de arquivos, arrastar e
// soltar, fechar com arquivos não salvos.

import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, type CSSProperties } from 'react';

import { chordActions, handleShortcut, runAction } from './actions';
import { ContextMenuHost } from './components/common';
import { Dialogs } from './components/dialogs/Dialogs';
import { MenuBar } from './components/layout/MenuBar';
import { StatusBar } from './components/layout/StatusBar';
import { TitleBar } from './components/layout/TitleBar';
import { Toolbar } from './components/layout/Toolbar';
import { Workbench } from './components/layout/Workbench';
import { watchZen, ZenHud, zenWidth } from './components/layout/Zen';
import { externalDrop, externalLeave, externalOver } from './components/sidebar/dnd';
import { Toasts } from './components/Toasts';
import { Tour } from './components/tour/Tour';
import { recentStepDirs, setConsoleTheme, setLinkHandler } from './console/consoles';
import { setShellTheme } from './console/shell';
import { bindEditorChord } from './editor/monaco';
import { t, useLang } from './i18n';
import { api } from './ipc/api';
import { useApp } from './state/app';
import { confirm } from './state/dialogs';
import { useEditor } from './state/editor';
import { useExperimental } from './state/experimental';
import { refreshGitSoon } from './state/git';
import { loadAccount } from './state/github';
import { useHierarchy } from './state/hierarchy';
import { useJobs } from './state/jobs';
import { takeFreshStart, useLayout } from './state/layout';
import { useLearn } from './state/learn';
import { useProject } from './state/project';
import { activeLayout } from './state/savedLayouts';
import { rememberSplashLook, splashRemaining, splashStatus } from './state/splash';
import { applyTitleBar, useTitleBar, watchWindow } from './state/titleBar';
import { showError } from './state/toasts';
import { useTour } from './state/tour';
import { startUpdateChecks } from './state/updates';
import { watchWideTabs } from './state/wideTabs';
import type { Theme } from './themes';
import { extension, isAbsolutePath, normalizeDots, resolveFrom } from './util/paths';

/**
 * Mostra a janela, uma vez, e fecha o splash. Ela nasce escondida
 * (`visible: false` em tauri.conf.json), com o splash na frente (a janela
 * `splash`, public/splash.html), e só aparece com o Studio pronto: a
 * interface desenhada no tema e no zoom das preferências, a AURORA achada e o
 * último projeto reaberto, ou depois de REVEAL_WAIT_MS, o que vier antes. O
 * splash fica pelo menos SPLASH_MIN_MS na tela, contados de quando ele
 * aparece: o desenho do logo e uma volta inteira do sinal pelos fios. Se a
 * interface não chegar aqui, o backend mostra a janela e fecha o splash
 * sozinho depois de um tempo (`REVEAL_FALLBACK`, em lib.rs).
 */
let revealed = false;
const REVEAL_WAIT_MS = 8000;
const SPLASH_MIN_MS = 3200;
/** Quanto a abertura espera a AURORA ser achado (a tela inicial e a barra de
 * status já nascem com o bundle). */
const TOOLCHAIN_WAIT_MS = 4000;
function reveal() {
  if (revealed) return;
  revealed = true;
  splashStatus(t('splash.ready'));
  // A janela principal aparece antes de o splash fechar: não há um instante
  // sem janela nenhuma.
  window.setTimeout(
    () =>
      void getCurrentWindow()
        .show()
        .catch(() => undefined)
        .then(() => api.app.closeSplash())
        .catch(() => undefined),
    splashRemaining(SPLASH_MIN_MS),
  );
}

/** Espera `ready` ficar verdadeiro, no máximo `ms`. */
function waitFor(ready: () => boolean, ms: number): Promise<void> {
  const started = Date.now();
  return new Promise((resolve) => {
    const check = () => (ready() || Date.now() - started >= ms ? resolve() : void window.setTimeout(check, 50));
    check();
  });
}

/** O que roda uma vez, ao abrir a janela. */
async function start(): Promise<() => void> {
  const cleanups: (() => void)[] = [];

  // Os recursos experimentais antes do último projeto: a sessão dele não
  // reabre a aba de um recurso desligado (state/experimental.ts).
  const [settings] = await Promise.all([useApp.getState().init(), useExperimental.getState().load()]);
  // A barra de título integrada antes de a janela aparecer: no Windows e no
  // Linux, a moldura do sistema sai (state/titleBar.ts).
  const os = useApp.getState().info?.os;
  if (os) await applyTitleBar(os);
  cleanups.push(await watchWindow());
  // Um `aurora experimental enable|disable` dado num terminal fora do Studio
  // vale quando a janela volta a ter o foco. O projeto também é relido: a
  // pasta dele movida ou renomeada no Explorer não dá aviso à vigia, que
  // acompanha a pasta para onde ela for.
  cleanups.push(
    await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (!focused) return;
      void useExperimental.getState().load();
      void useProject.getState().refresh();
      // Um commit, uma troca de ramo ou um login feitos fora da AURORA.
      refreshGitSoon(0);
      void loadAccount();
    }),
  );
  // Sem a janela guardada no localStorage (a primeira abertura, ou o
  // armazenamento da WebView limpo), ela abre no layout em uso. A janela
  // ainda está escondida: o primeiro quadro já sai nele.
  if (takeFreshStart() && settings) useLayout.getState().applyBody(activeLayout(settings).body);
  const applyTheme = (theme: Theme) => {
    setConsoleTheme(theme);
    setShellTheme(theme);
    rememberSplashLook(theme, useApp.getState().info?.version);
  };
  applyTheme(useApp.getState().theme);
  cleanups.push(useApp.subscribe((s, prev) => s.theme !== prev.theme && applyTheme(s.theme)));
  // O splash da próxima abertura fala o idioma de agora.
  cleanups.push(useLang.subscribe(() => rememberSplashLook(useApp.getState().theme, useApp.getState().info?.version)));

  // Os atalhos de duas etapas (Ctrl+K Z) também valem com o foco no editor.
  for (const a of chordActions()) bindEditorChord(a.keys!, () => runAction(a.id));
  cleanups.push(watchZen());
  cleanups.push(watchWideTabs());

  await getCurrentWebview()
    .setZoom(useLayout.getState().zoom)
    .catch(() => undefined);
  // Tema e zoom aplicados. A janela espera a AURORA ser achado e o último
  // projeto reabrir, para o primeiro quadro já ser o de trabalho, mas no
  // máximo REVEAL_WAIT_MS: um projeto grande não segura a abertura (o
  // splash cobre a espera).
  const revealTimer = window.setTimeout(reveal, REVEAL_WAIT_MS);
  cleanups.push(() => window.clearTimeout(revealTimer));
  splashStatus(t('splash.toolchain'));
  await waitFor(() => useApp.getState().toolchain !== null, TOOLCHAIN_WAIT_MS);

  // A pasta de exercícios do aurora learn, se houver uma lembrada: a vista
  // Exercícios e a aba do enunciado a usam mesmo sem a vista à mostra.
  void useLearn.getState().load();

  // Links arquivo:linha dos consoles abrem no editor.
  setLinkHandler((path, line, column) => void openLinkedFile(path, line, column));

  // O que mudou no disco: árvore, retrato do projeto, abas abertas e o
  // estado do git. Do .git, o vigia só avisa do HEAD, do índice e das
  // referências (watcher.rs), que são só do controle de versão.
  let refreshTimer = 0;
  const unlistenFs = await api.events.onFsChanged((event) => {
    refreshGitSoon();
    const paths = event.paths.filter((p) => !/[\\/]\.git([\\/]|$)/.test(p));
    if (paths.length === 0) return;
    useProject.getState().bumpTree();
    void useEditor.getState().onDiskChange(paths);
    useLearn.getState().onDiskChange(paths);
    if (paths.some((p) => ['v', 'sv', 'vh', 'svh'].includes(extension(p)))) useHierarchy.getState().markStale();
    window.clearTimeout(refreshTimer);
    refreshTimer = window.setTimeout(() => void useProject.getState().refresh(), 400);
  });
  cleanups.push(unlistenFs);

  // Arrastar arquivos do sistema: um .spf abre o projeto; no explorador, a
  // pasta debaixo do ponteiro recebe uma cópia, e Módulos ou Testbenches
  // registram o Verilog (components/sidebar/dnd.ts); no resto da janela, os
  // arquivos abrem no editor.
  const unlistenDrop = await getCurrentWebview().onDragDropEvent((event) => {
    const payload = event.payload;
    if (payload.type === 'over') {
      externalOver(payload.position);
      return;
    }
    if (payload.type === 'leave') {
      externalLeave();
      return;
    }
    if (payload.type !== 'drop') return;
    const paths = payload.paths;
    const spf = paths.find((p) => extension(p) === 'spf');
    if (spf) {
      externalLeave();
      void useProject.getState().open(spf);
      return;
    }
    void externalDrop(paths, payload.position).then((handled) => {
      if (!handled) for (const p of paths) void useEditor.getState().openFile(p);
    });
  });
  cleanups.push(unlistenDrop);

  // Fechar a janela com arquivos não salvos pergunta antes. Durante uma
  // atualização ou instalação da AURORA, não fecha: o `aurora` que troca os
  // arquivos morreria junto, com a instalação pela metade.
  const window_ = getCurrentWindow();
  const unlistenClose = await window_.onCloseRequested(async (event) => {
    const running = useJobs.getState().running;
    if (running?.flow === 'update' || running?.flow === 'install') {
      event.preventDefault();
      await confirm({
        title: t('dialog.busyClose.title'),
        message: t(running.flow === 'update' ? 'dialog.busyClose.update' : 'dialog.busyClose.install'),
        buttons: [{ label: t('common.ok'), value: 'ok', primary: true }],
      });
      return;
    }
    const snapshot = useProject.getState().snapshot;
    if (snapshot) useEditor.getState().persistSession(snapshot.spf);
    const dirty = useEditor.getState().tabs.filter((tab) => tab.dirty);
    if (dirty.length === 0) return;
    event.preventDefault();
    const answer = await confirm({
      title: t('dialog.unsaved.title'),
      message: t('dialog.unsavedMany.message', { count: dirty.length }),
      buttons: [
        { label: t('action.saveAll'), value: 'save', primary: true },
        { label: t('common.discard'), value: 'discard', danger: true },
        { label: t('common.cancel'), value: 'cancel' },
      ],
    });
    if (answer === 'save' && !(await useEditor.getState().saveAll())) return;
    if (answer === 'save' || answer === 'discard') await window_.destroy();
  });
  cleanups.push(unlistenClose);

  // Reabre o último projeto, como a AURORA. Sem ele, o editor vazio mostra
  // o começo (WelcomeView.tsx).
  const last = useApp.getState().recent[0] ?? (await api.app.recent().catch(() => []))[0];
  if (settings?.restore_last_project && last?.exists) {
    splashStatus(t('splash.project', { name: last.name }));
    await useProject.getState().open(last.spf);
  }
  // Um quadro para o React desenhar o projeto antes de a janela aparecer.
  window.setTimeout(reveal, 0);
  // Studio recém-instalado: o tour pela interface, com o tema, o idioma e o
  // layout padrão (state/tour.ts).
  if (settings && !settings.tour_done) useTour.getState().start();

  // AURORA mais nova publicado: procura logo depois de abrir e a cada 12
  // horas, e oferece a atualização (state/updates.ts).
  cleanups.push(startUpdateChecks());

  return () => cleanups.forEach((cleanup) => cleanup());
}

/** Abre o arquivo de um link `arquivo:linha` de um console. Um caminho
 * relativo vale a partir da raiz do projeto ou do CWD da ferramenta que o
 * escreveu (no Windows, o caminho de uma pasta com acento vai relativo ao
 * CWD): abre a primeira dessas pastas em que o arquivo existe. Sem nenhuma,
 * abre o da raiz, que diz que o arquivo não existe. */
async function openLinkedFile(path: string, line: number, column: number): Promise<void> {
  const root = useProject.getState().snapshot?.root;
  const open = (file: string) => useEditor.getState().openFile(normalizeDots(file), { line, column });
  if (!root || isAbsolutePath(path)) {
    await open(root ? resolveFrom(root, path) : path);
    return;
  }
  for (const base of [root, ...recentStepDirs()]) {
    const candidate = normalizeDots(resolveFrom(base, path));
    const stat = await api.fs.stat(candidate).catch(() => null);
    if (stat && !stat.is_dir) {
      await open(candidate);
      return;
    }
  }
  await open(resolveFrom(root, path));
}

export function App() {
  const zen = useLayout((s) => s.zen);
  const bars = useLayout((s) => s.live.bars);
  const menus = !zen && bars.menubar;
  // A barra de ferramentas do modelo do layout: a clássica numa linha
  // própria; a da barra de título (RunControls.tsx) no fim dela ou da barra
  // de menus, que sem os menus aparece só com ela. A do editor fica na barra
  // de abas (EditorArea.tsx).
  const toolbar = !zen && bars.toolbar;
  const run = toolbar && bars.toolbarStyle === 'titlebar';
  // A faixa integrada sai em tela cheia (o zen com tela cheia, a tela cheia
  // do macOS): não há janela para mover, e os menus, se à vista, voltam para
  // a barra de menus. Esconder a barra de menus no layout tira só os menus
  // da faixa: ela é a barra de título.
  const titleBar = useTitleBar((s) => s.integrated && !s.fullscreen);
  // Sem a barra de status, o indicador do zen fica no lugar dela: a linha do
  // Vim, a espera de um atalho e o resultado da operação continuam à vista.
  const hud = zen || !bars.statusbar;
  const zenShell = useLayout((s) => s.zenShell);
  // No zen, com um grupo só, o editor e a gaveta do shell ficam numa coluna
  // centralizada (Preferências > Editor > Modo zen).
  const centered = useApp((s) => s.settings?.zen?.center_layout ?? true);
  const fontSize = useApp((s) => s.settings?.editor.font_size ?? 13);
  const singleGroup = useEditor((s) => s.groups.length === 1);
  const zenCentered = zen && centered && singleGroup;

  useEffect(() => {
    window.addEventListener('keydown', handleShortcut, true);

    let stop: (() => void) | null = null;
    let cancelled = false;
    void start().then(
      (cleanup) => {
        if (cancelled) cleanup();
        else stop = cleanup;
        // Roteiro de fumaça, só na build de desenvolvimento (src/dev/smoke.ts).
        if (import.meta.env.DEV) void import('./dev/smoke').then((m) => m.runSmokeScript());
      },
      (error) => {
        // A abertura parou no meio: a janela aparece com o erro.
        reveal();
        showError(error);
      },
    );
    return () => {
      cancelled = true;
      window.removeEventListener('keydown', handleShortcut, true);
      stop?.();
    };
  }, []);

  const appClass = `app${zen ? ' app--zen' : ''}${hud ? ' app--hud' : ''}${zenCentered ? ' app--zen-centered' : ''}${zen && zenShell ? ' app--zen-shell' : ''}`;
  return (
    <div className={appClass} style={zenCentered ? ({ '--zen-width': `${zenWidth(fontSize)}px` } as CSSProperties) : undefined}>
      {titleBar ? <TitleBar menus={menus} run={run} /> : (menus || run) && <MenuBar menus={menus} run={run} />}
      {toolbar && bars.toolbarStyle === 'classic' && <Toolbar />}
      <Workbench />
      {hud ? <ZenHud /> : <StatusBar />}
      <Dialogs />
      <Tour />
      <ContextMenuHost />
      <Toasts />
    </div>
  );
}
