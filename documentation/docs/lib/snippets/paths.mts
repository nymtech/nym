// Inline source path check (D14, task 2b.1): each repository path that a prose
// text names in inline code exists in the repository.
//
//   node lib/snippets/paths.mts [--from <content dir>] [--root <repo root>]
//
// With --from, checks every page under that directory (a baseline over pages
// that are not split yet); without it, checks the prose snippets. A path counts
// as a repository path when its first segment is a top-level entry of the
// repository, so `src/network/v2.rs` (relative to a crate) is not checked.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { fenceTracker } from '../retrieval/mdx.mjs';
import { REPO_ROOT, CODEX_DIR } from './repo.mts';

// `a/b`, `a/b.rs`, `a/b/`: no space, no scheme, at least one slash.
const PATH = /^[\w.@-]+(?:\/[\w.@-]+)+\/?$/;

export interface InlinePath {
  path: string;
  line: number;
}

export interface PathCheck {
  mentions: number;
  distinct: number;
  /** Each path that does not exist, with each `file:line` that names it. */
  missing: Map<string, string[]>;
}

/** The repository paths in the inline code of a text. */
export function inlinePaths(text: string, topLevel: Set<string>): InlinePath[] {
  const found: InlinePath[] = [];
  const fence = fenceTracker();
  text.split('\n').forEach((line, i) => {
    if (fence(line)) return;
    for (const [, code = ''] of line.matchAll(/`([^`\n]+)`/g)) {
      const candidate = code.trim().replace(/:\d+(?:-\d+)?$/, ''); // `file.rs:42` names a file
      if (PATH.test(candidate) && topLevel.has(candidate.split('/')[0] ?? '')) found.push({ path: candidate.replace(/\/$/, ''), line: i + 1 });
    }
  });
  return found;
}

/** Check the paths of each file. */
export function checkPaths(files: string[], root: string = REPO_ROOT): PathCheck {
  const topLevel = new Set(fs.readdirSync(root).filter((e) => !e.startsWith('.')));
  const distinct = new Set<string>();
  const missing = new Map<string, string[]>();
  let mentions = 0;
  for (const file of files) {
    for (const { path: p, line } of inlinePaths(fs.readFileSync(file, 'utf-8'), topLevel)) {
      mentions++;
      distinct.add(p);
      if (!fs.existsSync(path.join(root, p))) {
        const where = missing.get(p) ?? [];
        where.push(`${path.relative(process.cwd(), file)}:${line}`);
        missing.set(p, where);
      }
    }
  }
  return { mentions, distinct: distinct.size, missing };
}

function mdxFiles(dir: string): string[] {
  const out: string[] = [];
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) out.push(...mdxFiles(full));
    else if (/\.mdx?$/.test(e.name)) out.push(full);
  }
  return out.sort();
}

function main(argv: string[]): void {
  const at = (name: string): string | null => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1] ?? null; };
  const rootArg = at('root');
  const fromArg = at('from');
  const root = rootArg ? path.resolve(rootArg) : REPO_ROOT;
  const dir = fromArg ? path.resolve(fromArg) : path.join(root, CODEX_DIR, 'snippets');
  const { mentions, distinct, missing } = checkPaths(mdxFiles(dir), root);
  for (const [p, where] of [...missing].sort()) console.log(`missing  ${p}  (${where.length}: ${where.slice(0, 3).join(', ')}${where.length > 3 ? ', ...' : ''})`);
  console.log(`${mentions} mentions of ${distinct} distinct repository paths; ${distinct - missing.size} exist, ${missing.size} do not`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2));
}
