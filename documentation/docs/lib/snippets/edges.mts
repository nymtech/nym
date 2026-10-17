// Authored edges on records that have no frontmatter (D4c, task 2b.4):
// `.nym-codex/edges.json` is a list of { from, to, rel }. A `realises` edge from
// a code item to a requirement goes here.

import fs from 'node:fs';
import path from 'node:path';
import type { RefRel, SnippetRef } from '../retrieval/types.ts';
import type { Repo } from './repo.mts';
import { CODEX_DIR } from './repo.mts';

const RELS: ReadonlySet<string> = new Set<RefRel>(['relates-to', 'depends-on', 'realises', 'specialises', 'observes', 'mitigates', 'assessed-against']);
const isAuthoredRel = (rel: unknown): rel is RefRel => typeof rel === 'string' && RELS.has(rel);

/** One entry of edges.json as written, unchecked, and where it is for messages. */
export interface AuthoredEdge {
  from: unknown;
  to: unknown;
  rel: unknown;
  at: string;
}

/** The authored edges, each with its index for messages. */
export function loadEdges(repo: Repo): AuthoredEdge[] {
  const file = path.join(repo.dir, 'edges.json');
  if (!fs.existsSync(file)) return [];
  const edges: unknown = JSON.parse(fs.readFileSync(file, 'utf-8'));
  if (!Array.isArray(edges)) throw new Error(`${CODEX_DIR}/edges.json is not a list`);
  return edges.map((e: { from?: unknown; to?: unknown; rel?: unknown } | null, i) => ({ from: e?.from, to: e?.to, rel: e?.rel, at: `${CODEX_DIR}/edges.json[${i}]` }));
}

/**
 * Check each edge and put it in the `refs` of its `from` record. `cites` is
 * not authored: the tools write it from citations.
 * Returns the errors.
 */
export function applyEdges(edges: AuthoredEdge[], records: { id: string; refs: SnippetRef[] }[]): string[] {
  const byId = new Map(records.map((r) => [r.id, r]));
  const errors: string[] = [];
  for (const e of edges) {
    const { from, to, rel } = e;
    const record = typeof from === 'string' ? byId.get(from) : undefined;
    if (typeof from !== 'string' || typeof to !== 'string') errors.push(`${e.at}: an edge needs a "from" and a "to" id`);
    else if (!isAuthoredRel(rel)) errors.push(`${e.at}: relation "${rel}" is not one of ${[...RELS].join(', ')}`);
    else if (!record) errors.push(`${e.at}: from ${from}, which has no record`);
    else if (!byId.has(to)) errors.push(`${e.at}: to ${to}, which has no record`);
    else {
      const { refs } = record;
      if (!refs.some((r) => r.id === to && r.rel === rel)) refs.push({ id: to, rel });
      refs.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : a.rel < b.rel ? -1 : a.rel > b.rel ? 1 : 0));
    }
  }
  return errors;
}
