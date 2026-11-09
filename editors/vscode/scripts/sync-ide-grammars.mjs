// Copia as gramáticas do C± e do assembly da AURORA IDE
// (src/editor/languages/cmm.ts e asm.ts do repositório
// nipscernlab-sandbox/aurora-ide) para test/fixtures/aurora-ide/, onde o
// teste de gramática (test/unit/grammar.test.ts) as compara com as da
// extensão. Ficam como .txt porque o tsconfig compila tudo em test/.
//
// Uso: npm run sync:ide-grammars -- <clone do aurora-ide>
//      (ou AURORA_IDE_SRC=<clone do aurora-ide> npm run sync:ide-grammars)
//
// Rode quando a IDE mudar uma das duas gramáticas e confira o diff.

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const source = process.argv[2] ?? process.env.AURORA_IDE_SRC;
if (!source) {
  console.error('Uso: npm run sync:ide-grammars -- <clone do aurora-ide> (ou defina AURORA_IDE_SRC)');
  process.exit(2);
}
const ide = resolve(source);
const version = JSON.parse(readFileSync(join(ide, 'package.json'), 'utf8')).version;
const out = join(import.meta.dirname, '..', 'test', 'fixtures', 'aurora-ide');
mkdirSync(out, { recursive: true });

for (const file of ['cmm.ts', 'asm.ts']) {
  const text = readFileSync(join(ide, 'src', 'editor', 'languages', file), 'utf8');
  const header =
    `// Cópia de src/editor/languages/${file} da AURORA IDE ${version}` +
    ' (nipscernlab-sandbox/aurora-ide), feita por scripts/sync-ide-grammars.mjs.' +
    ' Não edite à mão.\n';
  writeFileSync(join(out, `${file}.txt`), header + text);
  console.log(`test/fixtures/aurora-ide/${file}.txt (AURORA IDE ${version})`);
}
