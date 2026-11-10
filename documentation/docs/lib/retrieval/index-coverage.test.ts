import { describe, it, expect } from 'vitest';
// @ts-expect-error - plain ESM JS module, no type declarations
import { ROOTS } from '../../../indexed-sources.mjs';
// @ts-expect-error - plain ESM JS module
import { PROBES } from '../../../scripts/index-coverage-probes.mjs';

const atOrUnder = (path: string, root: string) => path === root || path.startsWith(`${root}/`);

const probePaths = Object.keys(PROBES);

describe('code index coverage probes', () => {
  it('every root in ROOTS has a probe at or under it', () => {
    const unprobed = (ROOTS as string[]).filter((root) => !probePaths.some((p) => atOrUnder(p, root)));
    expect(unprobed, 'add a probe to scripts/index-coverage-probes.mjs for each of these roots').toEqual([]);
  });

  it('every probe sits at or under a root in ROOTS', () => {
    const orphaned = probePaths.filter((p) => !(ROOTS as string[]).some((root) => atOrUnder(p, root)));
    expect(orphaned, 'these probes point outside ROOTS; remove them or add the root').toEqual([]);
  });
});
