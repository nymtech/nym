// Helpers for the tests of the tools.

import path from 'node:path';
import type { Snippet } from '../retrieval/types.ts';
import type { Manifest, Repo } from './repo.mts';
import { CODEX_DIR } from './repo.mts';

/** The value, or an error that names what is missing, so a failing test says what it did not find. */
export function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`missing: ${what}`);
  return value;
}

/** A record with the given fields, and empty values for the others. */
export const fixtureRecord = (fields: Partial<Snippet> & { id: string }): Snippet => ({
  kind: 'prose', authority: 'descriptive', repo: 'nym', path: '', title: '', body: '', hash: '', refs: [], meta: {}, ...fields,
});

/** The layout of a repository under `root`, with the given manifest fields. */
export function fixtureRepo(root: string, manifest: Partial<Manifest> = {}): Repo {
  const dir = path.join(root, CODEX_DIR);
  const snippets = path.join(dir, 'snippets');
  return { root, dir, snippets, manifest: { repo: 'nym', github: '', ...manifest }, repo: 'nym', aliases: { '@snippets/nym/': snippets } };
}
