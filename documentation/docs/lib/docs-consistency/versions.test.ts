import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import { NYM_SDK_VERSION, RUST_MSRV } from '../../components/versions';

// Crate versions reach the docs by two different routes, and only one of them is
// a projection.
//
// `{RUST_MSRV}` is interpolated from components/versions.ts, so it cannot drift:
// there is one copy and the page renders it.
//
// A Cargo.toml snippet cannot do that. MDX does not evaluate expressions inside a
// fenced code block, so `nym-sdk = "{NYM_SDK_VERSION}"` would render literally.
// The versions in those snippets are therefore hardcoded, and kept current by a
// second sed in ci-crates-version-bump.yml.
//
// Two seds against the same value is exactly the shape that drifts: a page whose
// snippet is formatted slightly differently is missed by the regex, keeps an old
// version, and nothing reports it. The docs then tell a developer to depend on a
// version that may no longer exist.
//
// This test is the guard for the copy that cannot be projected: it fails the
// build rather than letting a stale version ship.

const PAGES = path.resolve(__dirname, '../../pages');

function walk(dir: string, acc: string[] = []): string[] {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) walk(full, acc);
    else if (/\.mdx?$/.test(e.name)) acc.push(full);
  }
  return acc;
}

/** Every `nym-<crate> = "x.y.z"` literal in the docs, in either Cargo.toml form. */
function crateVersionLiterals() {
  const found: { file: string; crate: string; version: string }[] = [];
  for (const file of walk(PAGES)) {
    const text = fs.readFileSync(file, 'utf-8');
    const re = /^(nym-[a-z0-9-]+)\s*=\s*(?:"([0-9][^"]*)"|\{\s*version\s*=\s*"([0-9][^"]*)")/gm;
    for (const m of text.matchAll(re)) {
      found.push({
        file: path.relative(PAGES, file),
        crate: m[1],
        version: (m[2] ?? m[3])!,
      });
    }
  }
  return found;
}

describe('crate versions in Cargo.toml snippets', () => {
  const literals = crateVersionLiterals();

  it('finds the snippets at all, so a silent regex break is caught', () => {
    // If the snippets are reformatted past this regex the test would pass
    // vacuously, which is the failure mode it exists to prevent.
    expect(literals.length).toBeGreaterThan(5);
  });

  it('every hardcoded nym-* version matches the central constant', () => {
    const stale = literals.filter((l) => l.version !== NYM_SDK_VERSION);
    expect(
      stale.map((l) => `${l.file}: ${l.crate} = "${l.version}" (expected "${NYM_SDK_VERSION}")`),
    ).toEqual([]);
  });
});

describe('version constants', () => {
  it('the MSRV is a plain version the docs can interpolate', () => {
    // The retrieval build substitutes this into prose, so a value that is not a
    // bare version would reach readers looking like a placeholder.
    expect(RUST_MSRV).toMatch(/^\d+\.\d+(\.\d+)?$/);
  });

  it('the SDK version is a plain semver, optionally pre-release', () => {
    expect(NYM_SDK_VERSION).toMatch(/^\d+\.\d+\.\d+(-[\w.]+)?$/);
  });
});
