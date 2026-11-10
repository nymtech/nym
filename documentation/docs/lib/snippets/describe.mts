// Page summaries to page descriptions (task 2b.2, decided 2026-10-16).
//
//   node lib/snippets/describe.mts [--site codex] [--write]
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
import { splitFrontmatter } from './split.mts';
import type { Repo, SnippetImport } from './repo.mts';
import { openRepo, pageFiles, pageLinks, sitePages, snippetFile, snippetImports, routeOf } from './repo.mts';

const LIST_TITLES = new Set(['documents in this section', 'in this section']);
const ITEM = /^\s*[-*]\s+\[[^\]]+\]\(snippet:([^)\s]+)\):\s*(.+?)\s*$/;

/** A summary as plain text for a meta description: no code marks, links, or emphasis. */
export function plainSummary(text: string): string {
  return text.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/[`*_]/g, '').replace(/\s+/g, ' ').trim();
}

/** Set `description` in a page's frontmatter, unless it has one. Returns the new text or null. */
export function withDescription(text: string, description: string): string | null {
  const { frontmatter, rest } = splitFrontmatter(text);
  if (frontmatter && /^description:/m.test(frontmatter)) return null;
  const line = `description: ${JSON.stringify(description)}`;
  if (!frontmatter) return `---\n${line}\n---\n\n${rest}`;
  return `${frontmatter.slice(0, -4)}${line}\n---\n${rest}`;
}

/** One list section to remove, and the summaries it moves into the pages it lists. */
export interface DescriptionMove {
  page: string;
  import: SnippetImport;
  snippetFile: string;
  descriptions: { page: string; description: string }[];
}

export function planDescriptions(repo: Repo, site: string): { moves: DescriptionMove[]; skipped: string[] } {
  const pagesDir = sitePages(repo, site);
  const links = pageLinks(repo, site);
  const moves: DescriptionMove[] = [];
  const skipped: string[] = [];
  for (const rel of pageFiles(pagesDir)) {
    const pageText = fs.readFileSync(path.join(pagesDir, rel), 'utf-8');
    for (const imp of snippetImports(pageText, repo.repo)) {
      const file = snippetFile(repo, imp.id, '.mdx');
      if (!file || !fs.existsSync(file)) continue;
      const { content }: { content: string } = parseFrontmatter(fs.readFileSync(file, 'utf-8'));
      const lines = content.split('\n').filter((l) => l.trim() !== '');
      const title = lines[0]?.match(/^#{2,4}\s+(.+?)\s*(?:\[#[^\]]+\])?\s*$/)?.[1];
      if (!title || !LIST_TITLES.has(title.toLowerCase())) continue;
      // The pages a section lists are the ones in the directory its page heads.
      const dir = routeOf(rel);
      const descriptions = lines.slice(1).map((l) => {
        const [, id, summary] = l.match(ITEM) ?? [];
        const target = id === undefined ? undefined : links.get(id);
        const beside = target && path.dirname(target.page) === dir && target.anchor === null;
        return beside && summary !== undefined ? { page: `${target.page}.mdx`, description: plainSummary(summary) } : null;
      });
      if (descriptions.length === 0 || descriptions.includes(null)) {
        skipped.push(`${rel}: ${imp.id} is not a list of links to the pages beside it, each with a summary`);
        continue;
      }
      moves.push({ page: rel, import: imp, snippetFile: file, descriptions: descriptions.filter((d) => d !== null) });
    }
  }
  return { moves, skipped };
}

function main(argv: string[]): void {
  const at = (name: string): string | null => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1] ?? null; };
  const repo = openRepo();
  const site = at('site') ?? 'codex';
  const write = argv.includes('--write');
  const pagesDir = sitePages(repo, site);
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
