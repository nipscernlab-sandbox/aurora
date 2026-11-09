// As gramáticas do C± e do assembly do SAPHO foram portadas das do Studio
// (src/editor/languages/ do repositório nipscernlab-sandbox/aurora-ide), que
// foram conferidas contra o YANC. Este teste confere que as listas continuam
// as mesmas e que as expressões reconhecem o que devem. As do Studio são as
// cópias de test/fixtures/aurora-ide/, que o `npm run sync:ide-grammars`
// atualiza.

import * as assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { EXTENSION_ROOT, FIXTURES } from '../support/fixtures';

interface Rule {
  match?: string;
  begin?: string;
  patterns?: Rule[];
  name?: string;
}

const studio = (file: string) => readFileSync(join(FIXTURES, 'aurora-ide', `${file}.txt`), 'utf8');
const grammar = (file: string) => JSON.parse(readFileSync(join(EXTENSION_ROOT, 'syntaxes', file), 'utf8'));

/** Todas as expressões da gramática, em texto. */
function expressions(node: unknown): string[] {
  if (Array.isArray(node)) return node.flatMap(expressions);
  if (typeof node !== 'object' || node === null) return [];
  const rule = node as Record<string, unknown>;
  const own = ['match', 'begin', 'end'].flatMap((k) => (typeof rule[k] === 'string' ? [rule[k] as string] : []));
  return [...own, ...Object.values(rule).flatMap(expressions)];
}

function words(source: string, list: string): string[] {
  const match = new RegExp(`const ${list} = \\[([^\\]]*)\\]`).exec(source);
  assert.ok(match, `${list} no Studio`);
  return [...match[1].matchAll(/'([^']+)'/g)].map((m) => m[1]);
}

/** A palavra é casada inteira por alguma expressão da gramática. */
function covered(all: string[], word: string): boolean {
  return all.some((source) => {
    const re = new RegExp(`^(?:${source})$`);
    return re.test(word) || re.test(`#${word}`);
  });
}

describe('gramática do C±', () => {
  const cmm = grammar('cmm.tmLanguage.json');
  const all = expressions(cmm);
  const source = studio('cmm.ts');

  it('as expressões compilam', () => {
    for (const re of all) {
      assert.doesNotThrow(() => new RegExp(re), re);
    }
  });

  it('as palavras, os tipos, as funções e as diretivas do Studio (e do YANC)', () => {
    const functions = [...source.matchAll(/\{ nome: '([a-z0-9]+)', params:/g)].map((m) => m[1]);
    const directives = [...source.split('const DIRETIVAS')[1].split('];')[0].matchAll(/nome: '([A-Za-z]+)'/g)].map((m) => m[1]);
    assert.ok(functions.length > 20 && directives.length > 10);
    for (const word of [...words(source, 'PALAVRAS'), ...words(source, 'TIPOS'), ...functions, ...directives]) {
      assert.ok(covered(all, word), word);
    }
  });

  it('o literal complexo, o ket, o bra, o |I| e o índice invertido', () => {
    const find = (text: string) => all.filter((re) => new RegExp(re).test(text));
    assert.ok(find('2+3i').some((re) => re.includes('(i)')), 'a+bi');
    assert.ok(find('|v⟩').length > 0, 'ket');
    assert.ok(find('⟨v|').length > 0, 'bra');
    assert.ok(find('|I|').length > 0, 'identidade');
    assert.ok(find('x[k)').some((re) => re.includes('\\)')), 'índice invertido');
  });

  it('i sozinho não é palavra reservada (só no literal complexo)', () => {
    const named = (cmm.repository.common.patterns as Rule[]).filter((r) => r.name?.startsWith('keyword.control') || r.name?.startsWith('storage.type'));
    assert.ok(named.every((r) => !new RegExp(r.match ?? '$^').test('i')));
  });
});

describe('gramática do assembly do SAPHO', () => {
  const all = expressions(grammar('sapho-asm.tmLanguage.json'));

  it('os mnemônicos do Studio (a isa.tsv do YANC)', () => {
    const opcodes = [...studio('asm.ts').matchAll(/^\s*\['([A-Z_0-9]+)',/gm)].map((m) => m[1]);
    assert.ok(opcodes.length > 100);
    for (const opcode of opcodes) {
      assert.ok(covered(all, opcode), opcode);
    }
  });

  it('os mnemônicos diferenciam maiúsculas, como no montador', () => {
    assert.ok(!covered(all, 'add'));
  });
});
