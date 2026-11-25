import { readFileSync } from 'node:fs';
import { describe, it, expect } from 'vitest';
// @ts-expect-error - plain ESM JS module, no type declarations
import { ROOTS } from '../../../indexed-sources.mjs';

// The coverage probes are the `path` probes in the retrieval probe file, which
// the mcp-checks workflow of nymtech/retrieval-indexer runs against nym.com.
const PROBE_FILE = new URL('../../../mcp-probes/retrieval.json', import.meta.url);
const { probes } = JSON.parse(readFileSync(PROBE_FILE, 'utf-8')) as { probes: { path?: { repo: string; path: string } }[] };
const probePaths = probes.filter((p) => p.path?.repo === 'nymtech/nym').map((p) => p.path!.path);

const atOrUnder = (path: string, root: string) => path === root || path.startsWith(`${root}/`);

describe('code index coverage probes', () => {
  it('every root in ROOTS has a probe at or under it', () => {
    const unprobed = (ROOTS as string[]).filter((root) => !probePaths.some((p) => atOrUnder(p, root)));
    expect(unprobed, 'add a path probe to documentation/mcp-probes/retrieval.json for each of these roots').toEqual([]);
  });

  it('every probe sits at or under a root in ROOTS', () => {
    const orphaned = probePaths.filter((p) => !(ROOTS as string[]).some((root) => atOrUnder(p, root)));
    expect(orphaned, 'these probes point outside ROOTS; remove them or add the root').toEqual([]);
  });
});
