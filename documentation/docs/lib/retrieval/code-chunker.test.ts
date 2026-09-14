import { describe, it, expect } from 'vitest';
// @ts-expect-error - plain ESM JS module, no type declarations
import { chunkCode, chunkCodeFile, langOf, symbolOf } from './code-chunker.mjs';

describe('langOf', () => {
  it('detects rust, typescript, kotlin, swift, go and python, skips the rest', () => {
    expect(langOf('a/b.rs')).toBe('rust');
    expect(langOf('a/b.ts')).toBe('typescript');
    expect(langOf('a/b.tsx')).toBe('typescript');
    expect(langOf('a/b.cjs')).toBe('typescript');
    expect(langOf('a/b.kt')).toBe('kotlin');
    expect(langOf('a/b.kts')).toBe('kotlin');
    expect(langOf('a/b.swift')).toBe('swift');
    expect(langOf('a/b.go')).toBe('go');
    expect(langOf('a/b.py')).toBe('python');
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

describe('chunkCode (kotlin)', () => {
  const src = [
    'package com.nymtech.vpn',
    '',
    'import kotlinx.coroutines.flow.Flow',
    '',
    '/** A tunnel. */',
    '@Serializable',
    'data class TunnelConfig(val entry: String)',
    '',
    'sealed class TunnelState {',
    '  object Down : TunnelState()',
    '}',
    '',
    'fun Project.getGitHash(): String {',
    '  return "abc"',
    '}',
    '',
    'suspend fun connect(config: TunnelConfig) {',
    '  doThing()',
    '}',
  ].join('\n');

  it('splits at top-level items and names them', () => {
    const symbols = chunkCode(src, 'kotlin').map((c: { symbol: string }) => c.symbol);
    expect(symbols).toContain('TunnelConfig');
    expect(symbols).toContain('TunnelState');
    expect(symbols).toContain('connect');
  });

  it('names an extension function after the function, not the receiver', () => {
    const chunks = chunkCode(src, 'kotlin') as Array<{ symbol: string; text: string }>;
    expect(chunks.map((c) => c.symbol)).toContain('getGitHash');
    expect(chunks.map((c) => c.symbol)).not.toContain('Project');
  });

  it('groups a preceding KDoc and annotation with the item', () => {
    const config = (chunkCode(src, 'kotlin') as Array<{ symbol: string; text: string }>).find(
      (c) => c.symbol === 'TunnelConfig',
    );
    expect(config?.text).toContain('/** A tunnel. */');
    expect(config?.text).toContain('@Serializable');
  });
});

describe('chunkCode (swift)', () => {
  const src = [
    'import Foundation',
    '',
    '/// The tunnel manager.',
    '@MainActor',
    'public final class TunnelManager {',
    '  func start() {}',
    '}',
    '',
    'struct TunnelConfig {',
    '  let entry: String',
    '}',
    '',
    'extension TunnelConfig {',
    '  func validate() -> Bool { true }',
    '}',
    '',
    'func connect(_ config: TunnelConfig) {}',
  ].join('\n');

  it('splits at top-level items and names them (methods inside a type stay with it)', () => {
    const symbols = chunkCode(src, 'swift').map((c: { symbol: string }) => c.symbol);
    expect(symbols).toContain('TunnelManager');
    expect(symbols).toContain('TunnelConfig');
    expect(symbols).toContain('connect');
    // `start` is indented inside the class, so it is not a top-level boundary.
    expect(symbols).not.toContain('start');
  });

  it('groups a preceding doc comment and attribute with the item', () => {
    const mgr = (chunkCode(src, 'swift') as Array<{ symbol: string; text: string }>).find(
      (c) => c.symbol === 'TunnelManager',
    );
    expect(mgr?.text).toContain('/// The tunnel manager.');
    expect(mgr?.text).toContain('@MainActor');
  });
});

describe('chunkCode (go)', () => {
  const src = [
    'package tunnel',
    '',
    'import "context"',
    '',
    '// Server runs the tunnel.',
    'type Server struct {',
    '  addr string',
    '}',
    '',
    'func New(addr string) *Server {',
    '  return &Server{addr: addr}',
    '}',
    '',
    'func (s *Server) Start(ctx context.Context) error {',
    '  return nil',
    '}',
  ].join('\n');

  it('splits at top-level items; a method is named after the method, not the receiver', () => {
    const chunks = chunkCode(src, 'go') as Array<{ symbol: string; text: string }>;
    const symbols = chunks.map((c) => c.symbol);
    expect(symbols).toContain('Server');
    expect(symbols).toContain('New');
    expect(symbols).toContain('Start');
    const start = chunks.find((c) => c.symbol === 'Start');
    expect(start?.text).toContain('func (s *Server) Start');
  });
});

describe('chunkCode (python)', () => {
  const src = [
    'import os',
    '',
    '@dataclass',
    'class TunnelConfig:',
    '    entry: str',
    '',
    'async def connect(config: TunnelConfig) -> None:',
    '    await do_thing()',
    '',
    'def helper():',
    '    return 1',
  ].join('\n');

  it('splits at top-level def/class and groups a preceding decorator', () => {
    const chunks = chunkCode(src, 'python') as Array<{ symbol: string; text: string }>;
    const symbols = chunks.map((c) => c.symbol);
    expect(symbols).toContain('TunnelConfig');
    expect(symbols).toContain('connect');
    expect(symbols).toContain('helper');
    const cfg = chunks.find((c) => c.symbol === 'TunnelConfig');
    expect(cfg?.text).toContain('@dataclass');
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
    ['impl CheckResponse for crate::nyxd::TxResponse {', 'TxResponse', 'was crate, the path root'],
    ['impl TendermintRpcErrorMap for reqwest::Error {', 'Error', 'was reqwest, the path root'],
    ['impl<const N: usize> Foo<N> {', 'Foo', 'was N; a const generic is not the target'],
    ['impl Foo { // for Bar', 'Foo', 'a trailing comment must not supply the `for` target'],
    ['unsafe impl Send for X {}', 'X', 'the unsafe prefix must not block the impl branch'],
    ['impl<C> DkgQueryClient for C where C: CosmWasmClient {', 'DkgQueryClient', 'blanket impl: name it after the trait, not the parameter'],
  ];

  for (const [line, expected, why] of cases) {
    it(`${line.slice(0, 46)} -> ${expected} (${why})`, () => {
      expect(symbolOf(line, 'rust')).toBe(expected);
    });
  }

  // Baseline cases that must keep working.
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

describe('symbolOf: kotlin', () => {
  const cases: [string, string, string][] = [
    ['fun connect() {', 'connect', 'plain function'],
    ['suspend fun connect() {', 'connect', 'modifier before fun'],
    ['fun Project.getGitHash(): String {', 'getGitHash', 'extension: name, not the receiver'],
    ['val Project.gitHash: String', 'gitHash', 'extension property: name, not the receiver'],
    ['fun <T> identity(x: T): T {', 'identity', 'generics before the name are skipped'],
    ['data class TunnelConfig(val entry: String)', 'TunnelConfig', 'modifier before class'],
    ['sealed class TunnelState {', 'TunnelState', 'sealed class'],
    ['object Singleton {', 'Singleton', 'object'],
    ['const val MAX = 5', 'MAX', 'const val'],
  ];
  for (const [line, expected, why] of cases) {
    it(`${line.slice(0, 42)} -> ${expected} (${why})`, () => {
      expect(symbolOf(line, 'kotlin')).toBe(expected);
    });
  }
});

describe('symbolOf: swift', () => {
  const cases: [string, string, string][] = [
    ['func connect() {', 'connect', 'plain function'],
    ['public final class TunnelManager {', 'TunnelManager', 'modifiers before class'],
    ['struct TunnelConfig {', 'TunnelConfig', 'struct'],
    ['extension TunnelConfig {', 'TunnelConfig', 'extension names its type'],
    ['func connect<T>(_ x: T) {', 'connect', 'generics after the name'],
    ['actor Store {', 'Store', 'actor'],
    ['static func make() -> Self {', 'make', 'static modifier'],
    ['let shared = Store()', 'shared', 'top-level let'],
  ];
  for (const [line, expected, why] of cases) {
    it(`${line.slice(0, 42)} -> ${expected} (${why})`, () => {
      expect(symbolOf(line, 'swift')).toBe(expected);
    });
  }
});

describe('symbolOf: go', () => {
  const cases: [string, string, string][] = [
    ['func New(addr string) *Server {', 'New', 'plain function'],
    ['func (s *Server) Start(ctx context.Context) error {', 'Start', 'method: name, not the receiver'],
    ['type Server struct {', 'Server', 'type'],
    ['const MaxHops = 5', 'MaxHops', 'const'],
    ['var DefaultAddr = ":0"', 'DefaultAddr', 'var'],
  ];
  for (const [line, expected, why] of cases) {
    it(`${line.slice(0, 42)} -> ${expected} (${why})`, () => {
      expect(symbolOf(line, 'go')).toBe(expected);
    });
  }
});

describe('symbolOf: python', () => {
  const cases: [string, string, string][] = [
    ['def connect(config):', 'connect', 'function'],
    ['async def connect(config):', 'connect', 'async function'],
    ['class TunnelConfig:', 'TunnelConfig', 'class'],
  ];
  for (const [line, expected, why] of cases) {
    it(`${line.slice(0, 42)} -> ${expected} (${why})`, () => {
      expect(symbolOf(line, 'python')).toBe(expected);
    });
  }
});
