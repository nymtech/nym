import { describe, it, expect } from 'vitest';
// @ts-expect-error - plain ESM JS module, no type declarations
import { chunkCode, chunkCodeFile, langOf, symbolOf } from './code-chunker.mjs';

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

  it('groups preceding doc comments and attributes with the item they describe', () => {
    const withDocs = [
      'pub fn alpha() {',
      '  do_thing();',
      '}',
      '',
      '/// Beta does things.',
      '#[derive(Debug)]',
      'pub struct Beta {',
      '  x: u32,',
      '}',
    ].join('\n');
    const chunks = chunkCode(withDocs, 'rust') as Array<{ symbol: string; text: string }>;
    const alpha = chunks.find((c) => c.symbol === 'alpha');
    const beta = chunks.find((c) => c.symbol === 'Beta');
    // the doc comment + attribute belong to Beta, not to alpha's chunk
    expect(beta?.text).toContain('/// Beta does things.');
    expect(beta?.text).toContain('#[derive(Debug)]');
    expect(alpha?.text).not.toContain('Beta does things.');
    expect(alpha?.text).not.toContain('#[derive(Debug)]');
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

describe('symbolOf: rust', () => {
  const cases: [string, string, string][] = [
    ['impl<C> TlsWrap<C> {', 'TlsWrap', 'was C, the generic parameter'],
    ['impl<C, S> Service<Uri> for TlsWrap<C>', 'TlsWrap', 'was C; takes the type after `for`'],
    ["impl<'a, St> BandwidthImporter<'a, St>", 'BandwidthImporter', "was a, from the lifetime 'a"],
    ["impl<'de> Deserialize<'de> for Recipient {", 'Recipient', 'was de'],
    ['const fn v1_reply_surb_serialised_len() -> usize {', 'v1_reply_surb_serialised_len', 'was fn'],
    ['impl<T: Into<String>> Wrapper<T> {', 'Wrapper', 'nested generics must not end the skip early'],
  ];

  for (const [line, expected, why] of cases) {
    it(`${line.slice(0, 46)} -> ${expected} (${why})`, () => {
      expect(symbolOf(line, 'rust')).toBe(expected);
    });
  }

  // Lines the old regex already handled. The fix must not regress them.
  const unchanged: [string, string][] = [
    ['pub mod v2;', 'v2'],
    ['pub fn new(config: Config) -> Self {', 'new'],
    ['pub struct MixnetClient {', 'MixnetClient'],
    ['static mut BUFFER: [u8; 32] = [0; 32];', 'BUFFER'],
    ['const MAX_HOPS: usize = 5;', 'MAX_HOPS'],
    ['impl Default for Config {', 'Config'],
    ['pub trait Transport {', 'Transport'],
  ];

  for (const [line, expected] of unchanged) {
    it(`unchanged: ${line.slice(0, 40)} -> ${expected}`, () => {
      expect(symbolOf(line, 'rust')).toBe(expected);
    });
  }
});

describe('symbolOf: typescript', () => {
  it('keeps $ as a name, since it is one', () => {
    expect(symbolOf('const $ = <T extends HTMLElement>(id: string): T => {', 'typescript')).toBe('$');
  });
});
