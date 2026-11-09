// The PR comment of the snippet check (D7, task 4.1): each code excerpt that
// changes between two record sets, as a diff, and the pages that show it.
//
//   node lib/snippets/excerpt-diff.mts --base <records dir> --head <records dir> [--base-root <base tree>] [--root <repo root>] > comment.md
//
// <records dir> is the --out directory of records.mts (code.jsonl, prose.jsonl).
// A missing base directory means the base branch had no snippets yet. With
// --base-root, a citation that the PR removes still names the pages it was on.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import type { Snippet } from '../retrieval/types.ts';
import type { Repo } from './repo.mts';
import { openRepo, pageFiles, snippetImports, CODEX_DIR } from './repo.mts';

/** The pages of each site that show a code id, as `<site>: <page file>`. */
type Showing = (codeId: string) => string[];

export const MARKER = '<!-- nym-codex-snippets -->';

/** The records of one file that records.mts wrote. */
function readJsonl(file: string): Snippet[] {
  if (!fs.existsSync(file)) return [];
  return fs.readFileSync(file, 'utf-8').split('\n').filter(Boolean).map((l) => JSON.parse(l));
}

/** A unified diff of two texts, from the system `diff`. */
function unified(a: string, b: string, label: string): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'excerpt-'));
  fs.writeFileSync(path.join(dir, 'a'), `${a}\n`);
  fs.writeFileSync(path.join(dir, 'b'), `${b}\n`);
  const out = spawnSync('diff', ['-u', '--label', `base/${label}`, '--label', `head/${label}`, path.join(dir, 'a'), path.join(dir, 'b')], { encoding: 'utf-8' }).stdout;
  fs.rmSync(dir, { recursive: true, force: true });
  return out;
}

/** The pages of each site that show a code id: pages that import a prose snippet citing it. */
function pagesShowing(repo: Repo, prose: Snippet[]): Showing {
  const citers = new Map<string, Set<string>>(); // code id -> prose ids
  for (const p of prose) for (const r of p.refs) if (r.rel === 'cites' && r.id.includes('#')) {
    const set = citers.get(r.id) ?? new Set<string>();
    set.add(p.id);
    citers.set(r.id, set);
  }
  const pagesOf = new Map<string, Set<string>>(); // prose id -> page paths
  for (const [site, conf] of Object.entries(repo.manifest.sites ?? {})) {
    const dir = path.join(repo.root, conf.pages);
    for (const rel of pageFiles(dir)) {
      for (const { id } of snippetImports(fs.readFileSync(path.join(dir, rel), 'utf-8'), repo.repo)) {
        const set = pagesOf.get(id) ?? new Set<string>();
        set.add(`${site}: ${rel}`);
        pagesOf.set(id, set);
      }
    }
  }
  return (codeId) => [...new Set([...(citers.get(codeId) ?? [])].flatMap((p) => [...(pagesOf.get(p) ?? [])]))].sort();
}

/** A fence that no line of `text` can close: one backtick more than its longest run. */
const fenceFor = (text: string): string => '`'.repeat(Math.max(3, ...(text.match(/`+/g) ?? []).map((run) => run.length + 1)));

/** The comment text. Empty string when no excerpt changes. */
export function excerptComment({ base, head, showing }: { base: Snippet[]; head: Snippet[]; showing: Showing }): string {
  const before = new Map(base.map((r) => [r.id, r]));
  const after = new Map(head.map((r) => [r.id, r]));
  const sections: string[] = [];
  for (const id of [...new Set([...before.keys(), ...after.keys()])].sort()) {
    const a = before.get(id);
    const b = after.get(id);
    const shown = b ?? a;
    if (!shown || (a && b && a.hash === b.hash)) continue;
    const status = !a ? 'new citation' : !b ? 'citation removed' : 'excerpt changed';
    const pages = showing(id);
    const where = b ? `\`${b.path}\` lines ${b.lines?.[0]}-${b.lines?.[1]}` : `\`${shown.path}\``;
    sections.push([
      `### \`${id.slice(id.indexOf('/') + 1)}\` (${status})`,
      '',
      `${where}. Pages: ${pages.length ? pages.map((p) => `\`${p}\``).join(', ') : 'none'}.`,
      '',
      ...(a && b
        ? fenced('diff', unified(a.body, b.body, id.split('#')[1] ?? id).trimEnd())
        : fenced(shown.lang ?? '', shown.body)),
    ].join('\n'));
  }
  if (!sections.length) return '';
  return [MARKER, '## Code excerpts that this PR changes', '', `${sections.length} cited code items change. Each one shows on the pages listed with it.`, '', sections.join('\n\n')].join('\n');
}

function fenced(lang: string, text: string): string[] {
  const fence = fenceFor(text);
  return [`${fence}${lang}`, text, fence];
}

function main(argv: string[]): void {
  const at = (name: string): string | null => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1] ?? null; };
  const required = (name: string): string => {
    const value = at(name);
    if (value === null) throw new Error(`--${name} <records dir> is required`);
    return value;
  };
  const root = at('root');
  const baseDir = required('base');
  const headDir = required('head');
  const baseRoot = at('base-root');
  const repo = openRepo(root ? path.resolve(root) : undefined);
  const base = readJsonl(path.join(baseDir, 'code.jsonl'));
  const head = readJsonl(path.join(headDir, 'code.jsonl'));
  const headShowing = pagesShowing(repo, readJsonl(path.join(headDir, 'prose.jsonl')));
  // The pages of a citation the PR removes are in the base tree.
  const baseShowing: Showing = baseRoot && fs.existsSync(path.join(baseRoot, CODEX_DIR, 'manifest.json'))
    ? pagesShowing(openRepo(path.resolve(baseRoot)), readJsonl(path.join(baseDir, 'prose.jsonl')))
    : () => [];
  const showing: Showing = (id) => [...new Set([...headShowing(id), ...baseShowing(id)])].sort();
  process.stdout.write(excerptComment({ base, head, showing }));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2));
}
