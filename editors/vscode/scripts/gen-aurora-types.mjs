// Gera src/aurora/aurora-types.ts a partir dos JSON Schemas da AURORA
// (aurora/docs/schema/*.json), que a própria AURORA gera dos tipos do Core.
// Assim a extensão usa o mesmo contrato da CLI. É o mesmo esquema do
// scripts/gen-aurora-types.mjs da AURORA IDE (nipscernlab-sandbox/aurora-ide).
//
// Uso: npm run gen:types                  (a AURORA deste repositório, em ../..)
//      AURORA_REPO=/outro/aurora npm run gen:types
//      npm run gen:types -- --check       (só confere; sai com 1 se o arquivo
//                                          gravado não é o que os schemas geram)
//
// Rode de novo sempre que a AURORA mudar um tipo público e confira o diff.

import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { compile } from 'json-schema-to-typescript';

const here = import.meta.dirname;
const repo = resolve(process.env.AURORA_REPO ?? join(here, '..', '..', '..'));
const dir = join(repo, 'docs', 'schema');
const out = join(here, '..', 'src', 'aurora', 'aurora-types.ts');
const check = process.argv.includes('--check');

const defs = {};
const roots = {};
for (const file of readdirSync(dir).filter((f) => f.endsWith('.json')).sort()) {
  const schema = JSON.parse(readFileSync(join(dir, file), 'utf8'));
  for (const [name, def] of Object.entries(schema.$defs ?? {})) {
    defs[name] ??= def;
  }
  const { $defs, $schema, ...root } = schema;
  // O tipo raiz de cada comando ganha o nome do título do schema
  // (StatusReport, TopReport, ErrorReport).
  const name = (root.title ?? file.replace('.json', '')).replace(/[^A-Za-z0-9]/g, '');
  roots[name] = root;
}
// O schema das linhas do --events descreve a linha final com o resultado;
// o Event e o ResultLine do Core já estão em $defs.
delete roots.EventLine;

const combined = {
  title: 'AuroraSchemas',
  type: 'object',
  additionalProperties: false,
  properties: {},
  $defs: { ...defs, ...roots },
};

let ts = await compile(combined, 'AuroraSchemas', {
  bannerComment: '',
  unreachableDefinitions: true,
  additionalProperties: false,
  strictIndexSignatures: true,
  format: true,
  style: { singleQuote: true, printWidth: 100 },
});
ts = ts.replace(/export interface AuroraSchemas \{\}\n?/, '');

const header = `// GERADO por scripts/gen-aurora-types.mjs a partir de aurora/docs/schema/.
// Não edite à mão: rode \`npm run gen:types\`.
//
// São os tipos do Core da AURORA serializados (o mesmo JSON do \`--json\` da
// CLI). Os nomes seguem os tipos Rust: StatusReport, TopReport, ErrorInfo,
// Event, ResultLine.

/* eslint-disable */
`;
const text = header + ts;

if (check) {
  let current = '';
  try {
    current = readFileSync(out, 'utf8');
  } catch {
    // sem o arquivo, a comparação abaixo falha e diz o que fazer
  }
  if (current !== text) {
    console.error(`${out} não é o que ${dir} gera. Rode npm run gen:types e confira o diff.`);
    process.exit(1);
  }
  console.log(`${out} confere com ${dir}`);
} else {
  writeFileSync(out, text);
  console.log(`Wrote ${out} (${Object.keys(defs).length} definitions, ${Object.keys(roots).length} command types)`);
}
