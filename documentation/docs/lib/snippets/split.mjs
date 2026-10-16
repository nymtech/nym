// Migration tool: cut whole pages into snippet files and pages that import them.
//
//   node lib/snippets/split.mjs --from <content dir> --dir <page dir under it> [--repo nym] [--out <repo root>]
//
// Example, the pilot of stage 1:
//   node lib/snippets/split.mjs --from ../../../codex/content \
//     --dir specifications/nym-network/technical-details/discovery-urls
//
// Each page is cut at its `##` to `####` headings (outside code fences). The text
// before the first heading becomes a snippet with no heading, named `intro`.
// Each `mermaid` block becomes a `.mmd` file and a citation. Each link to a page
// in the same run becomes a `snippet:` link. The text does not change otherwise.
// The pages of one run are split together, so their links can name each other.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import GithubSlugger from 'github-slugger';
import { HEADING_ANCHOR, fenceTracker } from '../retrieval/mdx.mjs';
import { REPO_ROOT, routeOf } from './repo.mjs';

// The heading templates of the Nym network pages that decide an aspect. Any
// other heading gives `design`, marked for review (TBD(stage 2 owner): the full table).
const ASPECT_RULES = new Map([
  ['why it exists', 'motivation'],
  ['code', 'implementation'],
]);

const CUT = /^(#{2,4})\s+(.+?)\s*$/;

/** Split raw text into frontmatter (verbatim, with its fences) and the rest. */
export function splitFrontmatter(raw) {
  const m = raw.match(/^---\r?\n[\s\S]*?\r?\n---\r?\n/);
  return m ? { frontmatter: m[0], rest: raw.slice(m[0].length) } : { frontmatter: '', rest: raw };
}

/**
 * Heading text as a reader sees it, which is what Nextra slugs: the content of
 * each code span kept as it is, link syntax and paired emphasis removed outside
 * code spans.
 */
export function plainHeading(text) {
  return text
    .split(/(`[^`]*`)/)
    .map((part, i) => (i % 2 === 1
      ? part.slice(1, -1)
      : part
        .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
        .replace(/(\*\*|__)(?=\S)(.+?)(?<=\S)\1/g, '$2')
        .replace(/(^|\W)([*_])(?=\S)(.+?)(?<=\S)\2(?=\W|$)/g, '$1$3')))
    .join('')
    .trim();
}

/** A JS identifier for an import, from a slug: `nymnetworkdetails-v1` -> `NymnetworkdetailsV1`. */
export function componentName(slug) {
  const name = slug.split(/[^A-Za-z0-9]+/).filter(Boolean).map((w) => w[0].toUpperCase() + w.slice(1)).join('');
  return /^[A-Za-z]/.test(name) ? name : `S${name}`;
}

/**
 * Cut one page into sections. Pure: no file is read or written.
 * @returns {{ frontmatter: string, title: string, sections: object[] }}
 *   Each section: { slug, heading (the line, or null for the intro), text, level, lines }.
 */
export function cutPage(raw, { file }) {
  const { frontmatter, rest } = splitFrontmatter(raw);
  const lines = rest.split('\n');
  const titleAt = lines.findIndex((l) => /^# /.test(l));
  if (titleAt === -1) throw new Error(`${file}: no "# " title line`);
  if (lines.slice(0, titleAt).some((l) => l.trim() !== '')) {
    throw new Error(`${file}: content before the "# " title line is not supported`);
  }
  const title = lines[titleAt].slice(2).trim();

  const slugger = new GithubSlugger();
  const taken = new Set(['intro']);
  slugger.slug('intro'); // reserve the intro name, so a heading "Intro" becomes intro-1
  const sections = [{ slug: 'intro', heading: null, level: 1, lines: [] }];
  const fence = fenceTracker();
  for (const line of lines.slice(titleAt + 1)) {
    const cut = fence(line) === null ? line.match(CUT) : null;
    if (!cut) {
      sections.at(-1).lines.push(line);
      continue;
    }
    const explicit = cut[2].match(HEADING_ANCHOR);
    const text = explicit ? cut[2].slice(0, explicit.index) : cut[2];
    // An explicit anchor is used as it is; the slugger still records it, so a
    // later heading with the same text gets a suffix, as Nextra gives it.
    const slug = explicit ? (explicit[1] ?? explicit[2]) : slugger.slug(plainHeading(text));
    if (explicit) slugger.slug(slug);
    if (taken.has(slug)) throw new Error(`${file}: two sections have the anchor #${slug}`);
    taken.add(slug);
    sections.push({ slug, heading: `${cut[1]} ${text.trim()} [#${slug}]`, text: plainHeading(text), level: cut[1].length, lines: [] });
  }
  if (fence.isOpen()) throw new Error(`${file}: a code fence is not closed`);
  return { frontmatter, title, sections };
}

/** Lines with each top-level `mermaid` fence moved out to a diagram, and a citation in its place. */
function extractDiagrams(lines, { idBase, slug }) {
  const out = [];
  const diagrams = [];
  const fence = fenceTracker();
  let diagram = null;
  for (const line of lines) {
    const state = fence(line);
    if (state === 'open' && fence.info === 'mermaid') {
      diagram = [];
      continue;
    }
    if (diagram !== null) {
      if (state === 'close') {
        const name = `${slug}-d${diagrams.length + 1}`;
        diagrams.push({ name, text: `${diagram.join('\n')}\n` });
        out.push(`<Snippet id="${idBase}/${name}" />`);
        diagram = null;
      } else {
        diagram.push(line);
      }
      continue;
    }
    out.push(line);
  }
  return { lines: out, diagrams };
}

/**
 * Rewrite each Markdown link to a page of the run (or an anchor on the same
 * page) as a `snippet:` link. Fence-aware, and inline code spans are left alone.
 * `resolve(target)` returns an id, or undefined to keep the link.
 */
function rewriteLinks(lines, resolve) {
  const fence = fenceTracker();
  return lines.map((line) => {
    if (fence(line)) return line;
    return line.split(/(`[^`]*`)/).map((part, i) => (i % 2 === 1 ? part : part.replace(
      /\]\(([^)\s]+)\)/g,
      (whole, target) => {
        if (!target.startsWith('/') && !target.startsWith('#')) return whole;
        const id = resolve(target);
        return id === undefined ? whole : `](snippet:${id})`;
      },
    ))).join('');
  });
}

function trimBlank(lines) {
  let a = 0;
  let b = lines.length;
  while (a < b && lines[a].trim() === '') a++;
  while (b > a && lines[b - 1].trim() === '') b--;
  return lines.slice(a, b);
}

/** The aspect of a section from its heading template, or null when no rule matches. */
function ruleAspect(section) {
  return section.heading ? ASPECT_RULES.get(section.text.toLowerCase()) ?? null : null;
}

/**
 * Split a set of pages. Pure apart from reading the page files.
 * @param {{ from: string, files: string[], repo: string }} opts  `files` relative to `from`
 * @returns {{ writes: Map<string, string>, report: object }} paths relative to the repo root
 */
export function splitPages({ from, files, repo }) {
  const pages = files.map((rel) => {
    const cut = cutPage(fs.readFileSync(path.join(from, rel), 'utf-8'), { file: rel });
    // An index page's snippets go under `<dir>/index/`, apart from its siblings.
    const dir = rel.replace(/\.mdx?$/, '');
    const kept = cut.sections.filter((s) => s.heading !== null || trimBlank(s.lines).length > 0);
    return { rel, route: routeOf(rel), dir, idBase: `${repo}/${dir}`, ...cut, kept };
  });
  const byRoute = new Map(pages.map((p) => [p.route, p]));

  const report = { pages: pages.length, snippets: 0, diagrams: 0, linksConverted: 0, linksKept: 0, unresolved: [], aspectReview: 0 };
  const writes = new Map();
  const write = (rel, text) => {
    if (writes.has(rel)) throw new Error(`split writes ${rel} twice`);
    writes.set(rel, text);
  };

  for (const page of pages) {
    // A link target: undefined keeps the route (a page outside the run, or a
    // page with no intro for a link with no anchor); a missing anchor is reported.
    const resolve = (target) => {
      const [route, anchor] = target.startsWith('#') ? [page.route, target.slice(1)] : target.slice(1).split('#');
      const tp = byRoute.get(route.replace(/\/$/, ''));
      const hit = tp && (anchor ? tp.kept.find((s) => s.heading && s.slug === anchor) : tp.kept.find((s) => s.heading === null));
      if (hit) {
        report.linksConverted++;
        return `${tp.idBase}/${hit.slug}`;
      }
      if (tp && anchor) report.unresolved.push(`${page.rel}: ${target}`);
      report.linksKept++;
      return undefined;
    };

    const imports = [];
    for (const section of page.kept) {
      const { lines: withCitations, diagrams } = extractDiagrams(section.lines, { idBase: page.idBase, slug: section.slug });
      const body = rewriteLinks(trimBlank(withCitations), resolve);
      const aspect = ruleAspect(section);
      const aspectLines = aspect ? `aspect: ${aspect}` : 'aspect: design\naspect_review: true';
      const fm = section.heading === null
        ? `---\ntitle: ${JSON.stringify(page.title)}\n${aspectLines}\n---\n`
        : `---\n${aspectLines}\n---\n`;
      const text = [fm, ...(section.heading ? [section.heading, ...(body.length ? [''] : [])] : []), ...body].join('\n');
      write(`.nym-codex/snippets/${page.dir}/${section.slug}.mdx`, `${text.replace(/\n+$/, '')}\n`);
      for (const d of diagrams) {
        // A diagram and a section with one name would share an id.
        if (page.kept.some((s) => s.slug === d.name)) throw new Error(`${page.rel}: diagram ${d.name} has the name of a section`);
        write(`.nym-codex/snippets/${page.dir}/${d.name}.mmd`, d.text);
      }
      imports.push({ name: componentName(section.slug), id: `${page.idBase}/${section.slug}` });
      report.snippets++;
      report.diagrams += diagrams.length;
      if (!aspect) report.aspectReview++;
    }
    const names = imports.map((i) => i.name);
    const repeated = names.find((n, i) => names.indexOf(n) !== i);
    if (repeated) throw new Error(`${page.rel}: two snippets give the import name ${repeated}`);

    const pageText = [
      page.frontmatter.replace(/\n$/, ''),
      ...imports.map((i) => `import ${i.name} from '@snippets/${i.id}.mdx'`),
      '',
      `# ${page.title}`,
      '',
      ...imports.map((i) => `<${i.name} />`),
    ].filter((l, i) => i > 0 || l !== '');
    write(`.nym-codex/pages/${page.rel.replace(/\.md$/, '.mdx')}`, `${pageText.join('\n')}\n`);
  }
  return { writes, report };
}

/** `_meta.js` -> `_meta.json`, in key order. Nextra 4 `_meta` files are an object literal. */
export function metaJson(metaJs) {
  const src = metaJs.replace(/^\s*export\s+default\s*/, '').replace(/;\s*$/, '');
  return `${JSON.stringify(new Function(`return (${src})`)(), null, 2)}\n`;
}

function main(argv) {
  const opt = (name, fallback) => {
    const at = argv.indexOf(`--${name}`);
    if (at !== -1) return argv[at + 1];
    if (fallback === undefined) throw new Error(`--${name} is required`);
    return fallback;
  };
  const from = path.resolve(opt('from'));
  const dir = opt('dir');
  const repo = opt('repo', 'nym');
  const out = path.resolve(opt('out', REPO_ROOT));

  const files = [];
  const metas = [];
  const walk = (rel) => {
    for (const e of fs.readdirSync(path.join(from, rel), { withFileTypes: true })) {
      const r = path.join(rel, e.name);
      if (e.isDirectory()) walk(r);
      else if (/\.mdx?$/.test(e.name)) files.push(r);
      else if (e.name === '_meta.js') metas.push(r);
    }
  };
  walk(dir);
  files.sort();

  const { writes, report } = splitPages({ from, files, repo });
  for (const m of metas.sort()) writes.set(`.nym-codex/pages/${m.replace(/_meta\.js$/, '_meta.json')}`, metaJson(fs.readFileSync(path.join(from, m), 'utf-8')));

  for (const [rel, text] of writes) {
    const target = path.join(out, rel);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, text);
  }
  console.log(`${report.pages} pages -> ${report.snippets} snippets, ${report.diagrams} diagrams, ${metas.length} _meta.json`);
  console.log(`links: ${report.linksConverted} converted to snippet:, ${report.linksKept} kept as routes, ${report.unresolved.length} with an anchor that no section has`);
  console.log(`aspect: ${report.snippets - report.aspectReview} from a rule, ${report.aspectReview} marked aspect_review`);
  for (const u of report.unresolved) console.log(`  ${u}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2));
}
