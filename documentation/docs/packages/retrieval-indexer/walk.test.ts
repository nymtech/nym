import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { assertNoSilentDrops, assertRootsYieldChunks, collectSourceFiles } from './walk.ts';

describe('collectSourceFiles', () => {
  it('keeps chunkable files under the roots and skips excludes, oversized files, and missing roots', () => {
    const repo = fs.mkdtempSync(path.join(os.tmpdir(), 'walk-'));
    const write = (rel: string, text: string) => {
      fs.mkdirSync(path.dirname(path.join(repo, rel)), { recursive: true });
      fs.writeFileSync(path.join(repo, rel), text);
    };
    write('crate/src/lib.rs', 'pub fn a() {}');
    write('crate/README.md', '# not code');
    write('crate/target/debug/build.rs', 'fn built() {}');
    write('crate/src/big.rs', 'x'.repeat(64));
    write('outside/src/lib.rs', 'pub fn b() {}');
    write('crate/ffi/shim.c', 'int x;');

    const { files, oversized, dropped } = collectSourceFiles(repo, ['crate', 'gone'], { maxBytes: 32 });

    expect(files).toEqual([{ path: 'crate/src/lib.rs', content: 'pub fn a() {}' }]);
    expect(oversized).toEqual(['crate/src/big.rs']);
    expect(dropped).toEqual({ c: ['crate/ffi/shim.c'] });
  });
});

describe('assertNoSilentDrops', () => {
  it('passes an accepted drop', () => {
    expect(() => assertNoSilentDrops({ c: ['a.c'], proto: ['b.proto'] })).not.toThrow();
  });

  it('fails on a dropped language that is not accepted, naming the files', () => {
    expect(() => assertNoSilentDrops({ c: ['a.c'], rb: ['app/x.rb', 'app/y.rb'] })).toThrow(/\.rb \(2\): app\/x\.rb, app\/y\.rb/);
  });
});

describe('assertRootsYieldChunks', () => {
  const files = [{ path: 'a/src/lib.rs', content: '' }, { path: 'b/x.rs', content: '' }];

  it('passes when every root with source has a chunk, and ignores a root with none', () => {
    expect(() => assertRootsYieldChunks(['a', 'b', 'docs-only'], files, ['a/src/lib.rs', 'b/x.rs'])).not.toThrow();
  });

  it('fails on a root whose source produced no chunks', () => {
    expect(() => assertRootsYieldChunks(['a', 'b'], files, ['a/src/lib.rs'])).toThrow(/produced no chunks: b$/);
  });

  it('does not count a sibling with a shared prefix as under the root', () => {
    expect(() => assertRootsYieldChunks(['a'], files, ['ab/lib.rs'])).toThrow(/: a$/);
  });
});
