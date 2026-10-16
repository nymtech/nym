// The PR comment of the snippet check (D7, task 4.1): each code excerpt that
// changes between two record sets, as a diff, and the pages that show it.
//
//   node lib/snippets/excerpt-diff.mjs --base <records dir> --head <records dir> [--base-root <base tree>] [--root <repo root>] > comment.md
//
// <records dir> is the --out directory of records.mjs (code.jsonl, prose.jsonl).
// A missing base directory means the base branch had no snippets yet. With
// --base-root, a citation that the PR removes still names the pages it was on.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { openRepo, pageFiles, snippetImports, CODEX_DIR } from './repo.mjs';

export const MARKER = '<!-- nym-codex-snippets -->';

function readJsonl(file) {
  if (!fs.existsSync(file)) return [];
  return fs.readFileSync(file, 'utf-8').split('\n').filter(Boolean).map((l) => JSON.parse(l));
}

/** A unified diff of two texts, from the system `diff`. */
function unified(a, b, label) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'excerpt-'));
  fs.writeFileSync(path.join(dir, 'a'), `${a}\n`);
  fs.writeFileSync(path.join(dir, 'b'), `${b}\n`);
  const out = spawnSync('diff', ['-u', '--label', `base/${label}`, '--label', `head/${label}`, path.join(dir, 'a'), path.join(dir, 'b')], { encoding: 'utf-8' }).stdout;
  fs.rmSync(dir, { recursive: true, force: true });
  return out;
}

/** The pages of each site that show a code id: pages that import a prose snippet citing it. */
function pagesShowing(repo, prose) {
  const citers = new Map(); // code id -> prose ids
  for (const p of prose) for (const r of p.refs) if (r.rel === 'cites' && r.id.includes('#')) {
    if (!citers.has(r.id)) citers.set(r.id, new Set());
    citers.get(r.id).add(p.id);
  }
  const pagesOf = new Map(); // prose id -> page paths
  for (const [site, conf] of Object.entries(repo.manifest.sites ?? {})) {
    const dir = path.join(repo.root, conf.pages);
    for (const rel of pageFiles(dir)) {
      for (const { id } of snippetImports(fs.readFileSync(path.join(dir, rel), 'utf-8'), repo.repo)) {
        if (!pagesOf.has(id)) pagesOf.set(id, new Set());
        pagesOf.get(id).add(`${site}: ${rel}`);
      }
    }
  }
  return (codeId) => [...new Set([...(citers.get(codeId) ?? [])].flatMap((p) => [...(pagesOf.get(p) ?? [])]))].sort();
}

/** A fence that no line of `text` can close: one backtick more than its longest run. */
const fenceFor = (text) => '`'.repeat(Math.max(3, ...(text.match(/`+/g) ?? []).map((run) => run.length + 1)));

/** The comment text. Empty string when no excerpt changes. */
export function excerptComment({ base, head, showing }) {
  const before = new Map(base.map((r) => [r.id, r]));
  const after = new Map(head.map((r) => [r.id, r]));
  const sections = [];
  for (const id of [...new Set([...before.keys(), ...after.keys()])].sort()) {
    const a = before.get(id);
    const b = after.get(id);
    if (a && b && a.hash === b.hash) continue;
    const status = !a ? 'new citation' : !b ? 'citation removed' : 'excerpt changed';
    const pages = showing(id);
    const where = b ? `\`${b.path}\` lines ${b.lines[0]}-${b.lines[1]}` : `\`${a.path}\``;
    sections.push([
      `### \`${id.slice(id.indexOf('/') + 1)}\` (${status})`,
      '',
      `${where}. Pages: ${pages.length ? pages.map((p) => `\`${p}\``).join(', ') : 'none'}.`,
      '',
      ...(a && b
        ? fenced('diff', unified(a.body, b.body, id.split('#')[1]).trimEnd())
        : fenced((b ?? a).lang, (b ?? a).body)),
    ].join('\n'));
  }
  if (!sections.length) return '';
  return [MARKER, '## Code excerpts that this PR changes', '', `${sections.length} cited code items change. Each one shows on the pages listed with it.`, '', sections.join('\n\n')].join('\n');
}

function fenced(lang, text) {
  const fence = fenceFor(text);
  return [`${fence}${lang}`, text, fence];
}

function main(argv) {
  const at = (name) => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1]; };
  const repo = openRepo(at('root') ? path.resolve(at('root')) : undefined);
  const base = readJsonl(path.join(at('base'), 'code.jsonl'));
  const head = readJsonl(path.join(at('head'), 'code.jsonl'));
  const headShowing = pagesShowing(repo, readJsonl(path.join(at('head'), 'prose.jsonl')));
  // The pages of a citation the PR removes are in the base tree.
  const baseShowing = at('base-root') && fs.existsSync(path.join(at('base-root'), CODEX_DIR, 'manifest.json'))
    ? pagesShowing(openRepo(path.resolve(at('base-root'))), readJsonl(path.join(at('base'), 'prose.jsonl')))
    : () => [];
  const showing = (id) => [...new Set([...headShowing(id), ...baseShowing(id)])].sort();
  process.stdout.write(excerptComment({ base, head, showing }));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2));
}
