// OpenSpec adapter (D15, tasks 7.1 and 7.1b): one statement record for each
// requirement block in openspec/specs/<spec>/spec.md.
//
//   node lib/snippets/openspec.mjs     print the counts by level and status
//
// Id: <repo>/openspec/<spec>/<slug of the requirement title>. The record is
// prose with the aspect `requirement`; `meta.level` is the strongest RFC 2119
// keyword in the block, and `meta.status` is `enforced` when the manifest lists
// the id, else `approved`. Two requirements of one spec with one slug fail.

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { slug } from 'github-slugger';
import { fenceTracker } from '../retrieval/mdx.mjs';
import { openRepo } from './repo.mjs';

// MUST and SHALL are equal in RFC 2119; the order only picks a name for a block that has both.
const LEVELS = ['MUST', 'SHALL', 'SHOULD', 'MAY'];

/** The strongest RFC 2119 keyword in a text, or null. */
export function levelOf(text) {
  return LEVELS.find((k) => new RegExp(`\\b${k}\\b`).test(text)) ?? null;
}

/** The requirement blocks of one spec.md: [{ title, body, line }]. */
export function requirementBlocks(text) {
  const blocks = [];
  const fence = fenceTracker();
  let current = null;
  text.split('\n').forEach((line, i) => {
    const inFence = fence(line) !== null;
    const req = !inFence && line.match(/^### Requirement:\s*(.+?)\s*$/);
    const ends = !inFence && /^#{1,3} /.test(line);
    if (req || (ends && current)) {
      if (current) blocks.push(current);
      current = req ? { title: req[1], lines: [], line: i + 1 } : null;
      return;
    }
    if (current) current.lines.push(line);
  });
  if (current) blocks.push(current);
  return blocks.map(({ title, lines, line }) => ({ title, body: lines.join('\n').trim(), line }));
}

/** @returns {{ records: object[], errors: string[] }} */
export function buildRequirements(repo) {
  const specsDir = path.join(repo.root, 'openspec', 'specs');
  const enforced = new Set(repo.manifest.enforced ?? []);
  const records = [];
  const errors = [];
  if (!fs.existsSync(specsDir)) return { records, errors }; // a repository with no OpenSpec specs
  for (const spec of fs.readdirSync(specsDir).sort()) {
    const file = path.join(specsDir, spec, 'spec.md');
    if (!fs.existsSync(file)) continue;
    const rel = path.relative(repo.root, file);
    const seen = new Map();
    for (const block of requirementBlocks(fs.readFileSync(file, 'utf-8'))) {
      const s = slug(block.title);
      if (seen.has(s)) {
        errors.push(`${rel}:${block.line}: requirement "${block.title}" has the slug ${s}, as the one on line ${seen.get(s)}`);
        continue;
      }
      seen.set(s, block.line);
      const id = `${repo.repo}/openspec/${spec}/${s}`;
      records.push({
        id,
        kind: 'prose',
        aspect: 'requirement',
        authority: 'normative',
        repo: repo.repo,
        path: rel,
        title: block.title,
        body: block.body,
        hash: crypto.createHash('sha256').update(block.body).digest('hex'),
        refs: [],
        meta: { spec, level: levelOf(`${block.title}\n${block.body}`), status: enforced.has(id) ? 'enforced' : 'approved' },
      });
    }
  }
  records.sort((a, b) => (a.id < b.id ? -1 : 1));
  return { records, errors };
}

function main() {
  const { records, errors } = buildRequirements(openRepo());
  for (const e of errors) console.log(`error  ${e}`);
  const count = (key) => Object.entries(records.reduce((m, r) => ({ ...m, [r.meta[key] ?? 'none']: (m[r.meta[key] ?? 'none'] ?? 0) + 1 }), {})).map(([k, n]) => `${k} ${n}`).join(', ');
  console.log(`${records.length} requirement records from ${new Set(records.map((r) => r.meta.spec)).size} specs`);
  console.log(`level: ${count('level')}`);
  console.log(`status: ${count('status')}`);
  process.exitCode = errors.length ? 1 : 0;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
