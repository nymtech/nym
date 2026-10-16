// Standalone lint (stage 3, task 4b.1): a snippet must read alone, with no page
// around it, because a second site or the index can show it anywhere.
//
//   node lib/snippets/standalone.mjs [--root <repo root>] [--list]
//
// Two warnings:
// - a body that refers to text around it ("as described above", "the table
//   below", "the previous section", "this page");
// - a title that is a template word with no subject ("Code", "Used by").
// Each is a warning with a count until stage 3 ends (task 4b.4).

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseFrontmatter, fenceTracker, HEADING_ANCHOR } from '../retrieval/mdx.mjs';
import { openRepo } from './repo.mjs';

// Phrases that only make sense with the surrounding page. Each is narrow on
// purpose: "below 10 ms" or "above the threshold" are not references.
const REFERENCES = [
  /\bas (?:described|noted|shown|mentioned|discussed|explained|listed|defined) (?:above|below|earlier|previously|before)\b/i,
  /\b(?:the|this) (?:previous|next|following|preceding|last) (?:section|paragraph|table|list|example|diagram|step|page|chapter)\b/i,
  /\b(?:section|table|list|example|diagram|figure|code|snippet|command|overrides?|steps?) (?:above|below)\b(?! (?:the|a|an|\d))/i,
  /\b(?:above|below)[:,.)]/i,
  /\bon this page\b|\bthis (?:page|document)\b/i,
];

// Titles of the page templates: a word that names a kind of section, not a subject.
const TEMPLATE_TITLES = new Set([
  'code', 'technical details', 'used by', 'why it exists', 'status', 'overview',
  'introduction', 'details', 'notes', 'summary', 'example', 'examples', 'usage',
  'configuration', 'see also', 'further reading', 'references', 'documents in this section',
  'in this section', 'next steps', 'background', 'how it works',
]);

/** The standalone faults of one snippet file: [{ line, kind, text }]. */
export function standaloneFaults(raw) {
  const { content } = parseFrontmatter(raw);
  const offset = raw.split('\n').length - content.split('\n').length;
  const faults = [];
  const fence = fenceTracker();
  content.split('\n').forEach((line, i) => {
    if (fence(line)) return;
    const heading = line.match(/^#{1,6}\s+(.*)$/);
    if (heading) {
      const title = heading[1].replace(HEADING_ANCHOR, '').replace(/[`*_]/g, '').trim();
      if (TEMPLATE_TITLES.has(title.toLowerCase())) faults.push({ line: i + 1 + offset, kind: 'title', text: title });
      return;
    }
    // Inline code is example text, not prose that refers to the page.
    const prose = line.replace(/`[^`]*`/g, '');
    for (const pattern of REFERENCES) {
      const m = prose.match(pattern);
      if (m) {
        faults.push({ line: i + 1 + offset, kind: 'reference', text: m[0] });
        break;
      }
    }
  });
  return faults;
}

/** Each .mdx snippet under a directory, with its faults. */
export function lintSnippets(dir) {
  const out = [];
  const walk = (d) => {
    for (const e of fs.readdirSync(d, { withFileTypes: true }).sort((a, b) => (a.name < b.name ? -1 : 1))) {
      const full = path.join(d, e.name);
      if (e.isDirectory()) walk(full);
      else if (e.name.endsWith('.mdx')) out.push({ file: full, faults: standaloneFaults(fs.readFileSync(full, 'utf-8')) });
    }
  };
  if (fs.existsSync(dir)) walk(dir);
  return out;
}

function main(argv) {
  const at = (name) => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1]; };
  const repo = openRepo(at('root') ? path.resolve(at('root')) : undefined);
  const results = lintSnippets(repo.snippets);
  const with_ = (kind) => results.filter((r) => r.faults.some((f) => f.kind === kind)).length;
  if (argv.includes('--list')) {
    for (const r of results) for (const f of r.faults) console.log(`warn  ${path.relative(repo.root, r.file)}:${f.line}  ${f.kind}  ${f.text}`);
  }
  const clean = results.filter((r) => r.faults.length === 0).length;
  console.log(`${results.length} prose snippets: ${clean} with no standalone fault (${results.length ? Math.round((100 * clean) / results.length) : 0}%)`);
  console.log(`  refer to text around them: ${with_('reference')}`);
  console.log(`  template title with no subject: ${with_('title')}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2));
}
