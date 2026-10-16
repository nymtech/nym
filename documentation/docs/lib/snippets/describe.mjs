// Page summaries to page descriptions (task 2b.2, decided 2026-10-16).
//
//   node lib/snippets/describe.mjs [--site codex] [--write]
//
// An index page often holds a "Documents in this section" list: one link and one
// summary for each page beside it. The sidebar (_meta.json) already gives the
// list, and the summary belongs in the `description` of the page it describes,
// where the site uses it as the meta description. For each such section, this
// moves each summary into that page's frontmatter (when it has no description)
// and removes the section snippet and its import. Without --write it only reports.
//
// It acts only when every item is `- [Title](snippet:<id>): summary` and every
// target is a page in the same directory as the index page.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseFrontmatter } from '../retrieval/mdx.mjs';
import { splitFrontmatter } from './split.mjs';
import { openRepo, pageFiles, pageLinks, snippetFile, snippetImports, routeOf } from './repo.mjs';

const LIST_TITLES = new Set(['documents in this section', 'in this section']);
const ITEM = /^\s*[-*]\s+\[[^\]]+\]\(snippet:([^)\s]+)\):\s*(.+?)\s*$/;

/** A summary as plain text for a meta description: no code marks, links, or emphasis. */
export function plainSummary(text) {
  return text.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/[`*_]/g, '').replace(/\s+/g, ' ').trim();
}

/** Set `description` in a page's frontmatter, unless it has one. Returns the new text or null. */
export function withDescription(text, description) {
  const { frontmatter, rest } = splitFrontmatter(text);
  if (frontmatter && /^description:/m.test(frontmatter)) return null;
  const line = `description: ${JSON.stringify(description)}`;
  if (!frontmatter) return `---\n${line}\n---\n\n${rest}`;
  return `${frontmatter.slice(0, -4)}${line}\n---\n${rest}`;
}

/** @returns {{ moves: object[], skipped: string[] }} */
export function planDescriptions(repo, site) {
  const pagesDir = path.join(repo.root, repo.manifest.sites[site].pages);
  const links = pageLinks(repo, site);
  const moves = [];
  const skipped = [];
  for (const rel of pageFiles(pagesDir)) {
    const pageText = fs.readFileSync(path.join(pagesDir, rel), 'utf-8');
    for (const imp of snippetImports(pageText, repo.repo)) {
      const file = snippetFile(repo, imp.id, '.mdx');
      if (!file || !fs.existsSync(file)) continue;
      const { content } = parseFrontmatter(fs.readFileSync(file, 'utf-8'));
      const lines = content.split('\n').filter((l) => l.trim() !== '');
      const title = lines[0]?.match(/^#{2,4}\s+(.+?)\s*(?:\[#[^\]]+\])?\s*$/)?.[1];
      if (!title || !LIST_TITLES.has(title.toLowerCase())) continue;
      const items = lines.slice(1).map((l) => l.match(ITEM));
      // The pages a section lists are the ones in the directory its page heads.
      const dir = routeOf(rel);
      const targets = items.map((m) => m && links.get(m[1]));
      const ok = items.length > 0 && items.every(Boolean) && targets.every((t) => t && path.dirname(t.page) === dir && t.anchor === null);
      if (!ok) {
        skipped.push(`${rel}: ${imp.id} is not a list of links to the pages beside it, each with a summary`);
        continue;
      }
      moves.push({
        page: rel,
        import: imp,
        snippetFile: file,
        descriptions: items.map((m, i) => ({ page: `${targets[i].page}.mdx`, description: plainSummary(m[2]) })),
      });
    }
  }
  return { moves, skipped };
}

function main(argv) {
  const at = (name) => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1]; };
  const repo = openRepo();
  const site = at('site') ?? 'codex';
  const write = argv.includes('--write');
  const pagesDir = path.join(repo.root, repo.manifest.sites[site].pages);
  const { moves, skipped } = planDescriptions(repo, site);
  for (const s of skipped) console.log(`skip   ${s}`);
  let set = 0;
  let kept = 0;
  for (const m of moves) {
    console.log(`move   ${m.page}: ${m.descriptions.length} summaries from ${m.import.id}`);
    for (const d of m.descriptions) {
      const target = fs.existsSync(path.join(pagesDir, d.page)) ? path.join(pagesDir, d.page) : path.join(pagesDir, d.page.replace(/\.mdx$/, '/index.mdx'));
      const next = withDescription(fs.readFileSync(target, 'utf-8'), d.description);
      if (next === null) {
        kept++;
        console.log(`  keep   ${d.page}: it has a description`);
        continue;
      }
      set++;
      if (write) fs.writeFileSync(target, next);
    }
    if (write) {
      // Remove the section: its import line, its tag, and its file.
      const file = path.join(pagesDir, m.page);
      const text = fs.readFileSync(file, 'utf-8').split('\n')
        .filter((l) => !l.startsWith(`import ${m.import.name} from `) && l.trim() !== `<${m.import.name} />`);
      fs.writeFileSync(file, text.join('\n'));
      fs.rmSync(m.snippetFile);
    }
  }
  console.log(`${moves.length} sections, ${set} descriptions set, ${kept} pages already had one, ${skipped.length} sections skipped${write ? '' : ' (dry run; --write applies it)'}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2));
}
