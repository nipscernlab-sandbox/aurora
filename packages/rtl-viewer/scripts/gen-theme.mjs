// Gera src/theme-standalone.css: as variáveis que o viewer usa, com os
// valores dos temas padrão do Studio (boreal e boreal-claro, de src/themes)
// e dos tokens dele (src/styles/tokens.css), lidos de um clone do
// repositório da AURORA IDE (nipscernlab-sandbox/aurora-ide). É o tema do
// HTML do `aurora synth --open`, que segue o claro ou o escuro do sistema; o
// desenho sai com as mesmas cores do Studio.
//
// Uso: npm run gen:theme -- <clone do aurora-ide>
//      (ou AURORA_IDE_SRC=<clone do aurora-ide> npm run gen:theme)
//
// Rode de novo quando o Studio mudar um desses temas e confira o diff.

import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { build } from 'esbuild';

const root = join(import.meta.dirname, '..');
const source = process.argv[2] ?? process.env.AURORA_IDE_SRC;
if (!source) {
  console.error('Uso: npm run gen:theme -- <clone do aurora-ide> (ou defina AURORA_IDE_SRC)');
  process.exit(2);
}
const studio = resolve(source);
const studioVersion = JSON.parse(readFileSync(join(studio, 'package.json'), 'utf8')).version;

// As variáveis que o viewer usa, lidas do CSS e do código dele.
const used = new Set();
for (const file of readdirSync(join(root, 'src')).filter((f) => /\.(css|tsx?)$/.test(f) && !f.startsWith('theme-'))) {
  for (const [, name] of readFileSync(join(root, 'src', file), 'utf8').matchAll(/var\(--([a-z0-9-]+)/g)) {
    used.add(name);
  }
}
// --c e --c-fill são do próprio CSS (a cor da família de cada célula).
used.delete('c');
used.delete('c-fill');

const bundle = join(tmpdir(), `aurora-rtl-themes-${process.pid}.mjs`);
await build({
  entryPoints: [join(studio, 'src', 'themes', 'index.ts')],
  outfile: bundle,
  bundle: true,
  format: 'esm',
  platform: 'neutral',
  logLevel: 'error',
});
const themes = await import(pathToFileURL(bundle).href);

function variables(id) {
  const tokens = themes.uiTokens(themes.themeById(id));
  return [...used]
    .filter((name) => name in tokens)
    .sort()
    .map((name) => `  --${name}: ${tokens[name]};`)
    .join('\n');
}

// Os tokens que não são cor (fontes, tamanhos, espaços, raios), do Studio.
const tokensCss = readFileSync(join(studio, 'src', 'styles', 'tokens.css'), 'utf8');
const sizes = [...tokensCss.matchAll(/^\s*--([a-z0-9-]+):\s*(.+);$/gm)]
  .filter(([, name]) => used.has(name))
  .map(([, name, value]) => `  --${name}: ${value};`)
  .join('\n');

const missing = [...used].filter((name) => !variables(themes.DEFAULT_THEME).includes(`--${name}:`) && !sizes.includes(`--${name}:`));
if (missing.length > 0) {
  console.error(`Sem valor no Studio para: ${missing.join(', ')}`);
  process.exit(1);
}

const css = `/*
 * GERADO por scripts/gen-theme.mjs a partir de src/themes (${themes.DEFAULT_LIGHT_THEME} e
 * ${themes.DEFAULT_THEME}) e src/styles/tokens.css da AURORA IDE ${studioVersion}
 * (nipscernlab-sandbox/aurora-ide). Não edite à mão: rode
 * \`npm run gen:theme -- <clone do aurora-ide>\`.
 *
 * O tema do viewer fora do Studio e do VS Code (o HTML do \`aurora synth
 * --open\`): o claro ou o escuro, como o sistema.
 */

:root {
  color-scheme: light dark;
${sizes}
${variables(themes.DEFAULT_LIGHT_THEME)}
}

@media (prefers-color-scheme: dark) {
  :root {
${variables(themes.DEFAULT_THEME).replace(/^/gm, '  ')}
  }
}

html,
body {
  height: 100%;
  margin: 0;
  background: var(--sch-canvas);
  color: var(--text-0);
  font-family: var(--font-ui, system-ui, sans-serif);
  font-size: var(--text-md, 13px);
}
`;
writeFileSync(join(root, 'src', 'theme-standalone.css'), css);
console.log(`Wrote src/theme-standalone.css (${used.size} variables)`);
