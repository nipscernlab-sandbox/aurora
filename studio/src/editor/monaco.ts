// O Monaco: workers, linguagens, temas, um modelo de texto por arquivo e os
// marcadores dos diagnósticos da AURORA.
//
// Há uma instância só do editor (MonacoHost); cada arquivo aberto é um
// modelo, identificado pelo caminho. Trocar de aba troca o modelo e
// restaura a posição do cursor e da rolagem.

import * as monaco from 'monaco-editor';
import EditorWorker from 'monaco-editor/editor/editor.worker?worker';
import JsonWorker from 'monaco-editor/language/json/json.worker?worker';

import type { Diagnostic } from '../ipc/aurora-types';
import { alpha, tokenRule, uiTokens, type Theme } from '../themes';
import { asmTokenRules, registerAsm } from './languages/asm';
import { cmmTokenRules, registerCmm } from './languages/cmm';

export { monaco };

// Workers do Monaco, empacotados pelo Vite. Só o JSON tem serviço de
// linguagem próprio aqui (o .spf é JSON); o resto usa o worker do editor.
self.MonacoEnvironment = {
  getWorker(_id: string, label: string) {
    if (label === 'json') return new JsonWorker();
    return new EditorWorker();
  },
};

registerCmm(monaco);
registerAsm(monaco);

// A gramática de Verilog e SystemVerilog do Monaco declara module/endmodule e
// begin/end como pares. Casar um com o outro serve; colori-los como
// colchetes, não: a cor do par (a dos operadores, na maioria dos temas)
// cobria a de palavra reservada. Só os símbolos ganham cor de par. O Monaco
// junta as configurações campo a campo, e a da gramática, carregada depois,
// não tem este campo.
for (const id of ['verilog', 'systemverilog']) {
  monaco.languages.setLanguageConfiguration(id, {
    colorizedBracketPairs: [
      ['(', ')'],
      ['[', ']'],
      ['{', '}'],
    ],
  });
}

// O .spf é JSON (formato da AURORA). O Core aceita comentários e vírgula
// sobrando na leitura, então o editor não os marca como erro.
monaco.json.jsonDefaults.setDiagnosticsOptions({
  validate: true,
  allowComments: true,
  trailingCommas: 'ignore',
});

const EXTENSION_LANGUAGE: Record<string, string> = {
  cmm: 'cmm',
  asm: 'sapho-asm',
  v: 'verilog',
  vh: 'verilog',
  vt: 'verilog',
  sv: 'systemverilog',
  svh: 'systemverilog',
  c: 'cpp',
  cpp: 'cpp',
  h: 'cpp',
  hpp: 'cpp',
  py: 'python',
  json: 'json',
  spf: 'json',
  md: 'markdown',
  sh: 'shell',
  bash: 'shell',
  ps1: 'powershell',
  ys: 'tcl',
  tcl: 'tcl',
  xml: 'xml',
  yml: 'yaml',
  yaml: 'yaml',
  ini: 'ini',
  toml: 'ini',
};

/** A linguagem do Monaco para um arquivo, pela extensão. */
export function languageForPath(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? '';
  const ext = name.includes('.') ? name.split('.').pop()!.toLowerCase() : '';
  return EXTENSION_LANGUAGE[ext] ?? 'plaintext';
}

/** O nome legível da linguagem, para a barra de status. */
export function languageLabel(languageId: string): string {
  const labels: Record<string, string> = {
    cmm: 'C±',
    'sapho-asm': 'SAPHO ASM',
    verilog: 'Verilog',
    systemverilog: 'SystemVerilog',
    cpp: 'C/C++',
    python: 'Python',
    json: 'JSON',
    markdown: 'Markdown',
    plaintext: 'Texto',
    shell: 'Shell',
    tcl: 'Tcl',
  };
  return labels[languageId] ?? languageId;
}

// Temas ------------------------------------------------------------------

/** `selection` com metade da opacidade, para a seleção sem foco. */
function halfAlpha(color: string): string {
  const opacity = color.length === 9 ? Number.parseInt(color.slice(7), 16) / 255 : 1;
  return alpha(color, opacity / 2);
}

/**
 * O tema do Monaco que sai de um tema do Studio (themes/): as regras gerais
 * de token pelos papéis da sintaxe, as das gramáticas C± e asm, as do
 * próprio tema e as cores do editor e dos widgets pela interface.
 *
 * O `inherit` traz as regras do vs/vs-dark que não temos (markdown, css);
 * por isso as regras gerais cobrem também os tokens mais específicos do
 * tema base que as linguagens daqui usam (`number.hex`, `metatag.xml`,
 * `string.key.json`), senão eles ficariam com as cores do VS Code.
 */
function monacoTheme(theme: Theme): monaco.editor.IStandaloneThemeData {
  const { ui, syntax: s, editor: e } = theme;
  // As cores da interface com os padrões preenchidos (`active`, `hover`...).
  const tokens = uiTokens(theme);
  const dark = theme.scheme === 'dark';
  const style = { comment: 'italic', directive: 'bold', ...s.style };
  const brackets = e.brackets ?? [s.operator];
  const bracketColors: Record<string, string> = {};
  for (let level = 1; level <= 6; level++) {
    bracketColors[`editorBracketHighlight.foreground${level}`] = brackets[(level - 1) % brackets.length]!;
  }
  return {
    base: dark ? 'vs-dark' : 'vs',
    inherit: true,
    rules: [
      tokenRule('', s.fg),
      tokenRule('comment', s.comment, style.comment),
      tokenRule('keyword', s.keyword, style.keyword),
      tokenRule('keyword.flow', s.control, style.keyword),
      tokenRule('keyword.directive', s.directive, style.directive),
      tokenRule('keyword.json', s.constant),
      tokenRule('type', s.type, style.type),
      tokenRule('predefined', s.func, style.func),
      tokenRule('number', s.number),
      tokenRule('number.hex', s.number),
      tokenRule('string', s.string),
      tokenRule('string.key.json', s.variable),
      tokenRule('string.value.json', s.string),
      tokenRule('regexp', s.constant),
      tokenRule('constant', s.constant, style.constant),
      tokenRule('variable', s.variable),
      tokenRule('variable.predefined', s.constant, style.constant),
      tokenRule('variable.parameter', s.variable),
      tokenRule('key', s.variable),
      tokenRule('attribute.name', s.variable),
      tokenRule('attribute.value', s.string),
      tokenRule('tag', s.tag),
      tokenRule('metatag', s.tag),
      tokenRule('metatag.html', s.tag),
      tokenRule('metatag.xml', s.tag),
      tokenRule('annotation', s.directive),
      tokenRule('operator', s.operator),
      tokenRule('delimiter', s.delimiter),
      tokenRule('delimiter.html', s.delimiter),
      tokenRule('delimiter.xml', s.delimiter),
      ...cmmTokenRules(s),
      ...asmTokenRules(s),
      ...(theme.rules ?? []),
    ],
    colors: {
      'editor.background': ui.bgEditor,
      'editor.foreground': s.fg,
      'editorGutter.background': ui.bgEditor,
      'minimap.background': ui.bgEditor,
      'editorLineNumber.foreground': e.lineNumber,
      'editorLineNumber.activeForeground': e.lineNumberActive,
      'editor.lineHighlightBackground': e.lineHighlight,
      'editor.lineHighlightBorder': e.lineHighlightBorder ?? '#00000000',
      'editor.selectionBackground': e.selection,
      'editor.inactiveSelectionBackground': e.selectionInactive ?? halfAlpha(e.selection),
      'editor.findMatchBackground': e.findMatch,
      'editor.findMatchHighlightBackground': e.findMatchHighlight,
      'editorCursor.foreground': e.cursor,
      'editorIndentGuide.background1': e.indentGuide,
      'editorIndentGuide.activeBackground1': e.indentGuideActive,
      'editorBracketMatch.background': e.bracketMatch ?? ui.bg4,
      'editorBracketMatch.border': e.bracketMatchBorder ?? ui.text3,
      'editorWidget.background': ui.bg2,
      'editorWidget.border': ui.borderStrong,
      'editorSuggestWidget.background': ui.bg2,
      'editorSuggestWidget.border': ui.borderStrong,
      'editorSuggestWidget.selectedBackground': alpha(ui.brandFill, dark ? 0.4 : 0.15),
      'editorSuggestWidget.highlightForeground': ui.brandText,
      'editorHoverWidget.background': ui.bg2,
      'editorHoverWidget.border': ui.borderStrong,
      'input.background': ui.bg1,
      'input.border': ui.borderStrong,
      'input.foreground': ui.text0,
      'scrollbarSlider.background': dark ? '#FFFFFF1A' : '#0000001A',
      'scrollbarSlider.hoverBackground': dark ? '#FFFFFF2A' : '#0000002A',
      'scrollbarSlider.activeBackground': dark ? '#FFFFFF33' : '#00000033',
      'editorError.foreground': ui.error,
      'editorWarning.foreground': ui.warn,
      'editorInfo.foreground': ui.info,
      focusBorder: ui.brand,
      // O menu do botão direito, como os menus do Studio (.menu, app.css).
      'menu.background': ui.bg2,
      'menu.foreground': ui.text0,
      'menu.border': ui.borderStrong,
      'menu.selectionBackground': tokens.active!,
      'menu.selectionForeground': ui.text0,
      'menu.separatorBackground': ui.border,
      // Sem `brackets` no tema, todos os níveis na cor dos operadores.
      ...bracketColors,
      'editorBracketHighlight.unexpectedBracket.foreground': ui.error,
    },
  };
}

const definedThemes = new Set<string>();

/** Passa o Monaco (todas as instâncias) para o tema. Cada tema é definido no
 * Monaco na primeira vez que é usado. */
export function setMonacoTheme(theme: Theme): void {
  const name = `aurora-${theme.id}`;
  if (!definedThemes.has(name)) {
    monaco.editor.defineTheme(name, monacoTheme(theme));
    definedThemes.add(name);
  }
  monaco.editor.setTheme(name);
}

// Atalhos de duas etapas -------------------------------------------------

/** O código do Monaco de uma combinação como `Ctrl+K` ou `Z`. Só letras e
 * dígitos, que é o que os atalhos de duas etapas do Studio usam. */
function monacoKey(combo: string): number {
  let code = 0;
  for (const part of combo.split('+')) {
    if (part === 'Ctrl') code |= monaco.KeyMod.CtrlCmd;
    else if (part === 'Shift') code |= monaco.KeyMod.Shift;
    else if (part === 'Alt') code |= monaco.KeyMod.Alt;
    else if (/^[A-Z]$/.test(part)) code |= monaco.KeyCode[`Key${part}` as keyof typeof monaco.KeyCode];
    else if (/^\d$/.test(part)) code |= monaco.KeyCode[`Digit${part}` as keyof typeof monaco.KeyCode];
    else throw new Error(`Tecla sem equivalente no Monaco: ${part}`);
  }
  return code;
}

/**
 * Ensina ao Monaco um atalho de duas etapas do Studio (`Ctrl+K Z`). Com o
 * foco no editor, quem reconhece as teclas é o Monaco, que já tem atalhos
 * começando por Ctrl+K (Ctrl+K Ctrl+C comenta a linha); registrado aqui, o
 * do Studio convive com os dele. Fora do editor, quem cuida é o
 * handleShortcut de actions.ts.
 */
export function bindEditorChord(keys: string, run: () => void): void {
  const [first, second] = keys.split(' ');
  if (!first || !second) throw new Error(`Atalho sem duas etapas: ${keys}`);
  const command = `aurora.chord.${keys}`;
  monaco.editor.registerCommand(command, () => run());
  monaco.editor.addKeybindingRule({ keybinding: monaco.KeyMod.chord(monacoKey(first), monacoKey(second)), command });
}

// Modelos ----------------------------------------------------------------

export function uriFor(path: string): monaco.Uri {
  return monaco.Uri.file(path);
}

export function getModel(path: string): monaco.editor.ITextModel | null {
  return monaco.editor.getModel(uriFor(path));
}

/** O modelo do arquivo, criado com `content` se ainda não existe. */
export function ensureModel(path: string, content: string): monaco.editor.ITextModel {
  const existing = getModel(path);
  if (existing) return existing;
  const model = monaco.editor.createModel(content, languageForPath(path), uriFor(path));
  applyMarkers(model);
  return model;
}

/** Quem mostra o modelo de um arquivo fora do MonacoHost, por caminho. */
const holders = new Map<string, Set<() => void>>();

/**
 * Registra quem mostra o modelo de `path` fora do MonacoHost (o editor de
 * diff da aba de diff, components/views/DiffView.tsx): `release` roda antes
 * de o modelo ser descartado, para largar dele a tempo. O editor de diff do
 * Monaco reclama (um erro no console) de um modelo descartado com ele ainda
 * preso. Devolve a função que desfaz o registro.
 */
export function holdModel(path: string, release: () => void): () => void {
  const key = uriFor(path).toString();
  const set = holders.get(key) ?? new Set();
  set.add(release);
  holders.set(key, set);
  return () => {
    set.delete(release);
    if (set.size === 0 && holders.get(key) === set) holders.delete(key);
  };
}

export function disposeModel(path: string): void {
  const key = uriFor(path).toString();
  for (const release of [...(holders.get(key) ?? [])]) release();
  getModel(path)?.dispose();
}

// Diagnósticos -----------------------------------------------------------

const OWNER = 'aurora';
let markersByFile = new Map<string, monaco.editor.IMarkerData[]>();

function key(path: string): string {
  return path.replace(/\\/g, '/');
}

function severity(diagnostic: Diagnostic): monaco.MarkerSeverity {
  switch (diagnostic.severity) {
    case 'error':
      return monaco.MarkerSeverity.Error;
    case 'warning':
      return monaco.MarkerSeverity.Warning;
    default:
      return monaco.MarkerSeverity.Info;
  }
}

function applyMarkers(model: monaco.editor.ITextModel): void {
  const markers = markersByFile.get(key(model.uri.fsPath)) ?? [];
  const lines = model.getLineCount();
  monaco.editor.setModelMarkers(
    model,
    OWNER,
    markers.map((m) => {
      const line = Math.min(Math.max(m.startLineNumber, 1), lines);
      return { ...m, startLineNumber: line, endLineNumber: line, endColumn: model.getLineMaxColumn(line) };
    }),
  );
}

/** Troca os marcadores da AURORA pelos diagnósticos da última operação. Só os
 * que têm arquivo e linha viram marcador; o resto fica no painel Problemas. */
export function setDiagnostics(diagnostics: Diagnostic[]): void {
  const next = new Map<string, monaco.editor.IMarkerData[]>();
  for (const d of diagnostics) {
    if (!d.file || !d.line) continue;
    const list = next.get(key(d.file)) ?? [];
    list.push({
      severity: severity(d),
      message: `${d.message} (${d.tool})`,
      source: d.tool,
      startLineNumber: d.line,
      startColumn: d.column ?? 1,
      endLineNumber: d.line,
      endColumn: d.column ? d.column + 1 : 1,
    });
    next.set(key(d.file), list);
  }
  markersByFile = next;
  for (const model of monaco.editor.getModels()) {
    if (model.uri.scheme === 'file') applyMarkers(model);
  }
}
