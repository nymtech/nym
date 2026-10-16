// Authored edges on records that have no frontmatter (D4c, task 2b.4):
// `.nym-codex/edges.json` is a list of { from, to, rel }. A `realises` edge from
// a code item to a requirement goes here.

import fs from 'node:fs';
import path from 'node:path';
import { CODEX_DIR } from './repo.mjs';

const RELS = new Set(['relates-to', 'depends-on', 'realises', 'specialises', 'observes', 'mitigates', 'assessed-against']);

/** The authored edges, each with its index for messages. */
export function loadEdges(repo) {
  const file = path.join(repo.dir, 'edges.json');
  if (!fs.existsSync(file)) return [];
  const edges = JSON.parse(fs.readFileSync(file, 'utf-8'));
  if (!Array.isArray(edges)) throw new Error(`${CODEX_DIR}/edges.json is not a list`);
  return edges.map((e, i) => ({ ...e, at: `${CODEX_DIR}/edges.json[${i}]` }));
}

/**
 * Check each edge and put it in the `refs` of its `from` record. `cites` is
 * not authored: the tools write it from citations.
 * @returns {string[]} errors
 */
export function applyEdges(edges, records) {
  const byId = new Map(records.map((r) => [r.id, r]));
  const errors = [];
  for (const e of edges) {
    if (typeof e.from !== 'string' || typeof e.to !== 'string') errors.push(`${e.at}: an edge needs a "from" and a "to" id`);
    else if (!RELS.has(e.rel)) errors.push(`${e.at}: relation "${e.rel}" is not one of ${[...RELS].join(', ')}`);
    else if (!byId.has(e.from)) errors.push(`${e.at}: from ${e.from}, which has no record`);
    else if (!byId.has(e.to)) errors.push(`${e.at}: to ${e.to}, which has no record`);
    else {
      const refs = byId.get(e.from).refs;
      if (!refs.some((r) => r.id === e.to && r.rel === e.rel)) refs.push({ id: e.to, rel: e.rel });
      refs.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : a.rel < b.rel ? -1 : a.rel > b.rel ? 1 : 0));
    }
  }
  return errors;
}
