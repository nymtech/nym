// Migration tool: cut whole pages into snippet files and pages that import them.
//
//   node lib/snippets/split.mjs --from <content dir> --dir <page dir under it>[,<dir>...] [--out <repo root>]
//   node lib/snippets/split.mjs --site docs --dir network,developers [--out <repo root>]
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
import { REPO_ROOT, routeOf, openRepo } from './repo.mjs';

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

/** A page that `split` cannot cut yet; `reason` says what it waits for. */
export class PageWaits extends Error {
  constructor(file, reason) {
    super(`${file}: ${reason}`);
    this.reason = reason;
  }
}

const IMPORT_LINE = /^import\s+(.+?)\s+from\s+['"]([^'"]+)['"]\s*;?\s*$/;
// A tag alone on its line: an element opens, closes, or closes itself. `<>` is a fragment.
const TAG_OPEN = /^\s*<([A-Za-z][\w.-]*)?(\s[^>]*)?(?<!\/)>\s*$/;
const TAG_CLOSE = /^\s*<\/\s*([A-Za-z][\w.-]*)?\s*>\s*$/;
const TAG_SELF = /^\s*<[A-Za-z][\w.-]*(\s[^>]*)?\/>\s*$/;
// An element that opens and closes on one line: `<Callout>text</Callout>`.
const TAG_INLINE = /^\s*<([A-Za-z][\w.-]*)(\s[^>]*)?>.*<\/\1>\s*$/;
const VOID = new Set(['br', 'hr', 'img', 'input', 'wbr']);
// Wrapper elements whose headings are cut, by Diataxis type (D17).
const WRAPPERS = { 'how-to': new Set(['Steps']), tutorial: new Set(['Steps']) };

/** The local names an import clause binds: `A`, `{ B, C as D }` -> A, B, D. */
function importNames(clause) {
  return clause.replace(/[{}]/g, ',').split(',').map((part) => part.trim().split(/\s+/).pop()).filter(Boolean);
}

/** The text of a page outside code fences, to tell a rendered tag from one shown in an example. */
function outsideCode(lines) {
  const fence = fenceTracker();
  return lines.filter((l) => !fence(l)).join('\n');
}

/**
 * Cut one page into sections. Pure: no file is read or written.
 *
 * `policy.components` maps each component name a site gives in its
 * `mdx-components` to its import source. An import of such a component is
 * removed, because the site gives it; any other import, a value used as
 * `{X}`, or an `export` makes the page wait (PageWaits). With no policy (the
 * codex pages), a page has no imports at all. Every problem with the page
 * itself is a PageWaits, so one page cannot stop a run.
 *
 * @returns {{ frontmatter: string, title: string, sections: object[] }}
 *   Each section: { slug, heading (the line, or null for the intro), text, level, lines },
 *   or a wrapper marker { wrapper: 'open' | 'close', name }.
 */
export function cutPage(raw, { file, policy = {} }) {
  const wait = (reason) => new PageWaits(file, reason);
  const { frontmatter, rest } = splitFrontmatter(raw);
  const lines = rest.split('\n');
  const diataxis = frontmatter.match(/^diataxis:\s*["']?([\w-]+)/m)?.[1];
  const wrappers = WRAPPERS[diataxis] ?? new Set();

  // Imports first: each must be a component the site gives.
  const imported = [];
  const scan = fenceTracker();
  lines.forEach((line, i) => {
    if (scan(line)) return;
    if (/^export\s/.test(line)) throw wait(`line ${i + 1} is an export; a snippet cannot hold one (stage 4 for scenario data)`);
    if (!/^import\s/.test(line)) return;
    // Two imports can share a line: `import A from 'a';import B from 'b'`.
    for (const statement of line.split(/;\s*(?=import\s)/)) {
      const m = statement.match(IMPORT_LINE);
      if (!m) throw wait(`line ${i + 1} is an import this tool cannot read`);
      imported.push({ names: importNames(m[1]), spec: m[2], at: i });
    }
  });
  const prose = outsideCode(lines);
  for (const { names, spec } of imported) {
    for (const name of names) {
      const given = policy.components?.get(name);
      const rendered = new RegExp(`<${name}[\\s/>.]`).test(prose);
      if (given === undefined && !rendered) throw wait(`imports ${name} from '${spec}' as a value; MDX gives a snippet no scope for it`);
      if (given === undefined) throw wait(`imports ${name} from '${spec}', which the site does not give in mdx-components`);
      if (policy.resolveSource && policy.resolveSource(spec) !== given) throw wait(`imports ${name} from '${spec}', but the site gives ${name} from '${given}'`);
      // An unused import of a component the site gives is dropped with the others.
    }
  }
  const importLines = new Set(imported.map((x) => x.at));
  const body = lines.filter((_, i) => !importLines.has(i));

  const titleAt = body.findIndex((l) => /^# /.test(l));
  if (titleAt === -1) throw wait('no "# " title line');
  if (body.slice(0, titleAt).some((l) => l.trim() !== '')) throw wait('content before the "# " title line');
  const title = body[titleAt].slice(2).trim();

  const slugger = new GithubSlugger();
  const taken = new Set(['intro']);
  slugger.slug('intro'); // reserve the intro name, so a heading "Intro" becomes intro-1
  const sections = [{ slug: 'intro', heading: null, level: 1, lines: [] }];
  const fence = fenceTracker();
  const open = []; // the elements open at this line; '' is a fragment
  let comment = false; // inside a multi-line {/* ... */}
  for (const line of body.slice(titleAt + 1)) {
    const inFence = fence(line) !== null;
    if (!inFence && comment) {
      if (line.includes('*/}')) comment = false;
      sections.at(-1).lines.push(line);
      continue;
    }
    if (!inFence && line.includes('{/*') && !line.includes('*/}')) {
      comment = true;
      sections.at(-1).lines.push(line);
      continue;
    }
    if (!inFence && /^\s*<[A-Za-z/>]/.test(line)) {
      const opening = line.match(TAG_OPEN);
      const closing = line.match(TAG_CLOSE);
      if (opening) {
        const [, name = '', attrs] = opening;
        const isWrapper = wrappers.has(name) && open.length === 0;
        if (isWrapper && (attrs || line !== `<${name}>`)) throw wait(`<${name}> has attributes or indentation; a page holds the bare wrapper tag only`);
        if (isWrapper) {
          sections.push({ wrapper: 'open', name });
          open.push(name);
          sections.push({ slug: null, heading: null, lines: [] }); // holds the text before the first step
          continue;
        }
        if (!VOID.has(name.toLowerCase())) open.push(name);
      } else if (closing) {
        const [, name = ''] = closing;
        if (open.at(-1) !== name) throw wait(`</${name}> closes <${open.at(-1) ?? 'nothing'}>`);
        if (open.length === 1 && wrappers.has(name)) {
          if (line !== `</${name}>`) throw wait(`</${name}> has indentation or a space; a page holds the bare wrapper tag only`);
          open.pop();
          sections.push({ wrapper: 'close', name });
          sections.push({ slug: null, heading: null, lines: [] }); // holds the text after the wrapper
          continue;
        }
        open.pop();
      } else if (!TAG_SELF.test(line) && !TAG_INLINE.test(line)) {
        throw wait(`"${line.trim().slice(0, 50)}" is a tag this tool cannot follow (over more than one line, or text after the tag)`);
      }
    }
    // A heading is a cut only outside every element, or directly inside a wrapper.
    const cuttable = !inFence && (open.length === 0 || (open.length === 1 && wrappers.has(open[0])));
    const cut = cuttable ? line.match(CUT) : null;
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
    if (taken.has(slug)) throw wait(`two sections have the anchor #${slug}`);
    taken.add(slug);
    sections.push({ slug, heading: `${cut[1]} ${text.trim()} [#${slug}]`, text: plainHeading(text), level: cut[1].length, lines: [] });
  }
  if (fence.isOpen()) throw wait('a code fence is not closed');
  if (comment) throw wait('a {/* comment is not closed');
  if (open.length) throw wait(`<${open.at(-1)}> is not closed`);
  // A heading that is not a cut (inside an element, at depth 5 or 6, indented,
  // or in a blockquote) would give a snippet a second heading (D4).
  for (const sec of sections) {
    const inner = fenceTracker();
    let inComment = false;
    const extra = (sec.lines ?? []).find((l) => {
      if (inner(l)) return false;
      if (inComment) { inComment = !l.includes('*/}'); return false; }
      if (l.includes('{/*') && !l.includes('*/}')) { inComment = true; return false; }
      return /^\s*(>\s*)*#{1,6}\s/.test(l);
    });
    if (extra) throw wait(`"${extra.trim().slice(0, 50)}" is a heading that split does not cut (inside an element, at depth 5 or 6, indented, or quoted); the page needs its runbook form first (stage 8)`);
  }
  // Text between a wrapper tag and the next heading has no snippet of its own.
  for (const s of sections) {
    if (s.slug === null && !s.wrapper && s.lines.some((l) => l.trim() !== '')) {
      throw wait('text next to a <Steps> tag and outside a step; a wrapper holds only steps');
    }
  }
  return { frontmatter, title, sections: sections.filter((s) => s.wrapper || s.slug !== null) };
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
 * @param {{ from: string, files: string[], repo: string, pagesOut?: string, policy?: object }} opts
 *   `files` relative to `from`. `pagesOut`: where pages go, relative to the repo
 *   root (`.nym-codex/pages` for codex; the docs content directory, in place,
 *   for the docs site). `policy`: see cutPage, plus `diagrams: false` to make a
 *   page with a `mermaid` block wait (a site with no diagram renderer).
 * @returns {{ writes: Map<string, string>, waits: { rel, reason }[], report: object }}
 *   paths relative to the repo root
 */
export function splitPages({ from, files, repo, pagesOut = '.nym-codex/pages', policy = {} }) {
  const waits = [];
  const pages = [];
  for (const rel of files) {
    const raw = fs.readFileSync(path.join(from, rel), 'utf-8');
    try {
      if (policy.components && rel.endsWith('.md')) throw new PageWaits(rel, 'a .md page; a page that imports snippets is .mdx, so rename it first');
      if (policy.diagrams === false && /^\s*(`{3,}|~{3,})\s*mermaid\s*$/m.test(raw)) throw new PageWaits(rel, 'a mermaid block, and the site has no diagram renderer yet (task 10.6)');
      const resolveSource = policy.importRoot && ((spec) => (spec.startsWith('.')
        ? path.relative(policy.importRoot, path.resolve(path.dirname(path.join(from, rel)), spec))
        : spec).replace(/\.(tsx?|jsx?|mjs)$/, ''));
      const cut = cutPage(raw, { file: rel, policy: { ...policy, resolveSource } });
      // An index page's snippets go under `<dir>/index/`, apart from its siblings.
      const dir = rel.replace(/\.mdx?$/, '');
      const kept = cut.sections.filter((s) => s.wrapper || s.heading !== null || trimBlank(s.lines).length > 0);
      pages.push({ rel, route: routeOf(rel), dir, idBase: `${repo}/${dir}`, ...cut, kept });
    } catch (error) {
      if (!(error instanceof PageWaits)) throw error;
      waits.push({ rel, reason: error.reason });
    }
  }
  const byRoute = new Map(pages.map((p) => [p.route, p]));

  const report = { pages: pages.length, waiting: waits.length, snippets: 0, diagrams: 0, linksConverted: 0, linksKept: 0, unresolved: [], aspectReview: 0 };
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
    const order = []; // the page body: snippet tags and wrapper lines, in order
    for (const section of page.kept) {
      if (section.wrapper) {
        order.push(section.wrapper === 'open' ? `<${section.name}>` : `</${section.name}>`);
        continue;
      }
      const { lines: withCitations, diagrams } = extractDiagrams(section.lines, { idBase: page.idBase, slug: section.slug });
      const body = rewriteLinks(trimBlank(withCitations), resolve);
      const aspect = ruleAspect(section);
      const aspectLines = aspect ? `aspect: ${aspect}` : 'aspect: design\naspect_review: true';
      const fm = section.heading === null
        ? `---\ntitle: ${JSON.stringify(page.title)}\n${aspectLines}\n---\n`
        : `---\n${aspectLines}\n---\n`;
      // Keep the original spacing: a blank line after the heading only where the page had one.
      const gap = body.length && section.lines.length && section.lines[0].trim() === '' ? [''] : [];
      const text = [fm, ...(section.heading ? [section.heading, ...gap] : []), ...body].join('\n');
      write(`.nym-codex/snippets/${page.dir}/${section.slug}.mdx`, `${text.replace(/\n+$/, '')}\n`);
      for (const d of diagrams) {
        // A diagram and a section with one name would share an id.
        if (page.kept.some((s) => s.slug === d.name)) throw new Error(`${page.rel}: diagram ${d.name} has the name of a section`);
        write(`.nym-codex/snippets/${page.dir}/${d.name}.mmd`, d.text);
      }
      imports.push({ name: componentName(section.slug), id: `${page.idBase}/${section.slug}` });
      order.push(`<${componentName(section.slug)} />`);
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
      ...order,
    ].filter((l, i) => i > 0 || l !== '');
    write(`${pagesOut}/${page.rel.replace(/\.md$/, '.mdx')}`, `${pageText.join('\n')}\n`);
  }
  return { writes, waits, report };
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
  const out = path.resolve(opt('out', REPO_ROOT));
  const site = opt('site', 'codex');
  const repo = openRepo();
  const conf = repo.manifest.sites[site];
  if (!conf) throw new Error(`manifest.json has no site "${site}"`);
  // codex pages come from a codex checkout and go to .nym-codex/pages; docs
  // pages are split in place, with the site's components and skips.
  const inPlace = site !== 'codex';
  const from = path.resolve(opt('from', inPlace ? path.join(repo.root, conf.pages) : undefined));
  const dir = opt('dir');
  const skip = new Set(conf.skip ?? []);
  const policy = inPlace
    ? { components: new Map(Object.entries(conf.components ?? {})), diagrams: conf.diagrams !== false, importRoot: path.dirname(from) }
    : {};

  const files = [];
  const metas = [];
  const walk = (rel) => {
    for (const e of fs.readdirSync(path.join(from, rel), { withFileTypes: true })) {
      const r = path.join(rel, e.name);
      if (e.isDirectory()) {
        if (!skip.has(e.name)) walk(r);
      } else if (/\.mdx?$/.test(e.name)) files.push(r);
      else if (e.name === '_meta.js') metas.push(r);
    }
  };
  for (const d of dir.split(',')) {
    if (d.split('/').some((seg) => skip.has(seg))) throw new Error(`--dir ${d} is in a directory the site skips (${[...skip].join(', ')})`);
    walk(d); // several directories split as one run, so their links can name each other
  }
  files.sort();

  const { writes, waits, report } = splitPages({ from, files, repo: repo.repo, pagesOut: inPlace ? conf.pages : '.nym-codex/pages', policy });
  // A docs page keeps its _meta.js in place; codex pages get _meta.json from it.
  if (!inPlace) for (const m of metas.sort()) writes.set(`.nym-codex/pages/${m.replace(/_meta\.js$/, '_meta.json')}`, metaJson(fs.readFileSync(path.join(from, m), 'utf-8')));

  for (const [rel, text] of writes) {
    const target = path.join(out, rel);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, text);
  }
  console.log(`${report.pages} pages -> ${report.snippets} snippets, ${report.diagrams} diagrams${inPlace ? '' : `, ${metas.length} _meta.json`}; ${report.waiting} pages wait`);
  console.log(`links: ${report.linksConverted} converted to snippet:, ${report.linksKept} kept as routes, ${report.unresolved.length} with an anchor that no section has`);
  console.log(`aspect: ${report.snippets - report.aspectReview} from a rule, ${report.aspectReview} marked aspect_review`);
  for (const u of report.unresolved) console.log(`  ${u}`);
  if (waits.length) console.log('pages that wait:');
  for (const w of waits) console.log(`  ${w.rel}: ${w.reason}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2));
}
