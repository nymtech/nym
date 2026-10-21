// OpenSpec adapter (D15, tasks 7.1 and 7.1b): one statement record for each
// requirement block in openspec/specs/<spec>/spec.md.
//
//   node lib/snippets/openspec.mts     print the counts by level and status
//
// Id: <repo>/openspec/<spec>/<slug of the requirement title>. The record is
// prose; `meta.level` is the strongest RFC 2119 keyword in the block, and
// `meta.status` is `enforced` when the manifest lists the id, else `approved`.
// Two requirements of one spec with one slug fail.

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { slug } from 'github-slugger';
import { fenceTracker } from '../retrieval/mdx.mjs';
import type { Snippet } from '../retrieval/types.ts';
import type { Repo } from './repo.mts';
import { openRepo } from './repo.mts';

// MUST and SHALL are equal in RFC 2119; the order only picks a name for a block that has both.
const LEVELS = ['MUST', 'SHALL', 'SHOULD', 'MAY'] as const;
export type Level = (typeof LEVELS)[number];

export interface RequirementBlock {
  title: string;
  body: string;
  line: number;
}

/** The strongest RFC 2119 keyword in a text, or null. */
export function levelOf(text: string): Level | null {
  return LEVELS.find((k) => new RegExp(`\\b${k}\\b`).test(text)) ?? null;
}

/** The requirement blocks of one spec.md. */
export function requirementBlocks(text: string): RequirementBlock[] {
  const blocks: { title: string; lines: string[]; line: number }[] = [];
  const fence = fenceTracker();
  let current: { title: string; lines: string[]; line: number } | null = null;
  text.split('\n').forEach((line, i) => {
    const inFence = fence(line) !== null;
    const req = inFence ? undefined : line.match(/^### Requirement:\s*(.+?)\s*$/)?.[1];
    const ends = !inFence && /^#{1,3} /.test(line);
    if (req !== undefined || (ends && current)) {
      if (current) blocks.push(current);
      current = req !== undefined ? { title: req, lines: [], line: i + 1 } : null;
      return;
    }
    if (current) current.lines.push(line);
  });
  if (current) blocks.push(current);
  return blocks.map(({ title, lines, line }) => ({ title, body: lines.join('\n').trim(), line }));
}

export function buildRequirements(repo: Repo): { records: Snippet[]; errors: string[] } {
  const specsDir = path.join(repo.root, 'openspec', 'specs');
  const enforced = new Set(repo.manifest.enforced ?? []);
  const records: Snippet[] = [];
  const errors: string[] = [];
  if (!fs.existsSync(specsDir)) return { records, errors }; // a repository with no OpenSpec specs
  for (const spec of fs.readdirSync(specsDir).sort()) {
    const file = path.join(specsDir, spec, 'spec.md');
    if (!fs.existsSync(file)) continue;
    const rel = path.relative(repo.root, file);
    const seen = new Map<string, number>();
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

function main(): void {
  const { records, errors } = buildRequirements(openRepo());
  for (const e of errors) console.log(`error  ${e}`);
  const count = (key: string): string => Object.entries(records.reduce<Record<string, number>>((m, r) => {
    const value = String(r.meta[key] ?? 'none');
    return { ...m, [value]: (m[value] ?? 0) + 1 };
  }, {})).map(([k, n]) => `${k} ${n}`).join(', ');
  console.log(`${records.length} requirement records from ${new Set(records.map((r) => r.meta.spec)).size} specs`);
  console.log(`level: ${count('level')}`);
  console.log(`status: ${count('status')}`);
  process.exitCode = errors.length ? 1 : 0;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
