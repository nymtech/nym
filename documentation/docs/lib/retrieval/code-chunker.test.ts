import { describe, it, expect } from 'vitest';
// @ts-expect-error - plain ESM JS module, no type declarations
import { chunkCode, chunkCodeFile, langOf } from './code-chunker.mjs';

describe('langOf', () => {
  it('detects rust and typescript, skips the rest', () => {
    expect(langOf('a/b.rs')).toBe('rust');
    expect(langOf('a/b.ts')).toBe('typescript');
    expect(langOf('a/b.tsx')).toBe('typescript');
    expect(langOf('a/b.md')).toBe(null);
  });
});

describe('chunkCode (rust)', () => {
  const src = [
    'use foo::bar::baz;',
    '',
    'pub fn alpha() {',
    '  do_thing();',
    '}',
    '',
    'struct Beta {',
    '  x: u32,',
    '}',
    '',
    'impl Beta {',
    '  fn method(&self) {}',
    '}',
  ].join('\n');

  it('splits at top-level item boundaries and names the symbols', () => {
    const chunks = chunkCode(src, 'rust');
    const symbols = chunks.map((c) => c.symbol);
    expect(symbols).toContain('alpha');
    expect(symbols).toContain('Beta');
    // the `use` preamble is its own leading block, not attached to alpha
    expect(chunks[0].text.startsWith('use foo::bar::baz;')).toBe(true);
  });

  it('records a 1-based start line for each block', () => {
    const chunks = chunkCode(src, 'rust');
    const alpha = chunks.find((c) => c.symbol === 'alpha');
    expect(alpha?.startLine).toBe(3); // `pub fn alpha` is line 3
  });
});

describe('chunkCode (typescript)', () => {
  it('splits at exported declarations and names them', () => {
    const src = [
      "import { x } from 'y';",
      '',
      'export function setupMixTunnel() {',
      '  return 1;',
      '}',
      '',
      'export class MixClient {}',
    ].join('\n');
    const symbols = chunkCode(src, 'typescript').map((c) => c.symbol);
    expect(symbols).toContain('setupMixTunnel');
    expect(symbols).toContain('MixClient');
  });
});

describe('chunkCodeFile', () => {
  it('tags source nym-code and builds a GitHub deep link with the line', () => {
    const [chunk] = chunkCodeFile('pub fn only() {}', 'common/nymsphinx/src/lib.rs');
    expect(chunk.source).toBe('nym-code');
    expect(chunk.title).toBe('common/nymsphinx/src/lib.rs');
    expect(chunk.heading).toBe('only');
    expect(chunk.url).toBe('https://github.com/nymtech/nym/blob/develop/common/nymsphinx/src/lib.rs#L1');
    expect(chunk.lang).toBe('rust');
  });

  it('returns nothing for non-code files', () => {
    expect(chunkCodeFile('# readme', 'a/b.md')).toEqual([]);
  });
});
