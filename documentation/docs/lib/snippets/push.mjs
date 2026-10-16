// Local push (D8, task 1.6): write this repository's snippets, records, and
// pages into a codex checkout.
//
//   node lib/snippets/push.mjs --target <codex checkout> [--root <repo root>]
//
// Writes `snippets/<repo>/` (the snippet files, {prose,code,diagram}.jsonl,
// manifest.json with the source commit, pages.json, the link map) and the
// pages under `content/`. Each target is replaced in full, so a deleted or
// renamed snippet leaves codex. `_meta.js` is written from each `_meta.json`.
// `lastUpdated` in each pushed page is the newest commit date of the page and
// the snippets it imports; no other frontmatter field changes.
//
// The manifest lists the page directories the repository owns in codex. Until
// every page there is split, only the directories that hold split pages are
// replaced, so a push cannot delete the pages that are not split yet.

import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { openRepo, pageLinks, pageFiles, snippetImports, snippetFile } from './repo.mjs';
import { splitFrontmatter } from './split.mjs';
import { buildAll } from './records.mjs';

const git = (root, ...args) => execFileSync('git', ['-C', root, ...args], { encoding: 'utf-8' }).trim();

/** The newest commit date (YYYY-MM-DD) of the files, or null when none is committed. */
function lastCommitDate(root, files) {
  const out = git(root, 'log', '-1', '--format=%cs', '--', ...files.map((f) => path.relative(root, f)));
  return out || null;
}

/** Set `lastUpdated` in a page's frontmatter, adding a frontmatter block when the page has none. */
export function withLastUpdated(text, date) {
  const { frontmatter, rest } = splitFrontmatter(text);
  if (!frontmatter) return `---\nlastUpdated: "${date}"\n---\n\n${rest}`;
  const line = `lastUpdated: "${date}"`;
  const body = frontmatter.slice(4, -4);
  const next = /^lastUpdated:.*$/m.test(body) ? body.replace(/^lastUpdated:.*$/m, line) : `${body}\n${line}`;
  return `---\n${next}\n---\n${rest}`;
}

const jsonl = (records) => records.map((r) => JSON.stringify(r)).join('\n') + (records.length ? '\n' : '');

async function main(argv) {
  const at = (name) => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1]; };
  if (!at('target')) throw new Error('--target <codex checkout> is required');
  const target = path.resolve(at('target'));
  const repo = openRepo(at('root') ? path.resolve(at('root')) : undefined);
  const site = repo.manifest.sites.codex;

  const built = await buildAll(repo);
  const { errors } = built;
  if (errors.length) throw new Error(`the records have errors; run records.mjs:\n${errors.join('\n')}`);

  // snippets/<repo>/: replaced in full.
  const snippetsOut = path.join(target, 'snippets', repo.repo);
  fs.rmSync(snippetsOut, { recursive: true, force: true });
  fs.cpSync(repo.snippets, snippetsOut, { recursive: true });
  fs.writeFileSync(path.join(snippetsOut, 'prose.jsonl'), jsonl([...built.prose, ...built.requirement].sort((a, b) => (a.id < b.id ? -1 : 1))));
  fs.writeFileSync(path.join(snippetsOut, 'diagram.jsonl'), jsonl(built.diagram));
  fs.writeFileSync(path.join(snippetsOut, 'code.jsonl'), jsonl(built.code));
  fs.writeFileSync(path.join(snippetsOut, 'model.jsonl'), jsonl(built.model));
  const links = Object.fromEntries([...pageLinks(repo, 'codex')].sort(([a], [b]) => (a < b ? -1 : 1)));
  fs.writeFileSync(path.join(snippetsOut, 'pages.json'), `${JSON.stringify(links, null, 2)}\n`);
  const commit = git(repo.root, 'rev-parse', 'HEAD');
  fs.writeFileSync(path.join(snippetsOut, 'manifest.json'), `${JSON.stringify({ repo: repo.repo, github: repo.manifest.github, commit }, null, 2)}\n`);

  // content/: each directory that holds split pages, replaced in full.
  const pagesDir = path.join(repo.root, site.pages);
  const pages = pageFiles(pagesDir);
  const dirs = [...new Set(pages.map((p) => path.dirname(p)))].sort();
  for (const dir of dirs) {
    if (!site.owns.some((own) => dir === own || dir.startsWith(`${own}/`))) throw new Error(`${dir} is not under a directory the manifest owns in codex`);
    fs.rmSync(path.join(target, 'content', dir), { recursive: true, force: true });
  }
  let undated = 0;
  for (const rel of pages) {
    const file = path.join(pagesDir, rel);
    const text = fs.readFileSync(file, 'utf-8');
    const sources = [file, ...snippetImports(text, repo.repo).map((i) => snippetFile(repo, i.id, '.mdx'))];
    let date = lastCommitDate(repo.root, sources);
    if (!date) {
      undated++;
      date = new Date().toISOString().slice(0, 10);
    }
    const out = path.join(target, 'content', rel);
    fs.mkdirSync(path.dirname(out), { recursive: true });
    fs.writeFileSync(out, withLastUpdated(text, date));
  }
  for (const dir of dirs) {
    const meta = path.join(pagesDir, dir, '_meta.json');
    if (fs.existsSync(meta)) {
      fs.writeFileSync(path.join(target, 'content', dir, '_meta.js'), `export default ${fs.readFileSync(meta, 'utf-8').trim()};\n`);
    }
  }

  console.log(`snippets/${repo.repo}/: ${built.prose.length} prose, ${built.requirement.length} requirement, ${built.diagram.length} diagram, ${built.code.length} code, ${built.model.length} model records; commit ${commit.slice(0, 10)}`);
  console.log(`content/: ${pages.length} pages in ${dirs.length} directories, replaced: ${dirs.join(', ')}`);
  if (undated) console.log(`${undated} pages have uncommitted files; their lastUpdated is today`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}
