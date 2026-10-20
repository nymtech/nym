// Migration tool: cut whole pages into snippet files and pages that import them.
//
//   node lib/snippets/split.mts --from <content dir> --dir <page dir under it>[,<dir>...] [--out <repo root>]
//   node lib/snippets/split.mts --site docs --dir network,developers [--out <repo root>]
//
// Example, the pilot of stage 1:
//   node lib/snippets/split.mts --from ../../../codex/content \
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
import type { Aspect } from '../retrieval/types.ts';
import { REPO_ROOT, CODEX_DIR, routeOf, openRepo, siteConfig } from './repo.mts';

// The heading templates of the Nym network pages that decide an aspect. Any
// other heading gives `design`, marked for review (TBD(stage 2 owner): the full table).
const ASPECT_RULES = new Map<string, Aspect>([
  ['why it exists', 'motivation'],
  ['code', 'implementation'],
]);

const CUT = /^(#{2,4})\s+(.+?)\s*$/;

/** Split raw text into frontmatter (verbatim, with its fences) and the rest. */
export function splitFrontmatter(raw: string): { frontmatter: string; rest: string } {
  const m = raw.match(/^---\r?\n[\s\S]*?\r?\n---\r?\n/);
  return m ? { frontmatter: m[0], rest: raw.slice(m[0].length) } : { frontmatter: '', rest: raw };
}

/**
 * Heading text as a reader sees it, which is what Nextra slugs: the content of
 * each code span kept as it is, link syntax and paired emphasis removed outside
 * code spans.
 */
export function plainHeading(text: string): string {
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
export function componentName(slug: string): string {
  const name = slug.split(/[^A-Za-z0-9]+/).filter(Boolean).map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join('');
  return /^[A-Za-z]/.test(name) ? name : `S${name}`;
}

/** A page that `split` cannot cut yet; `reason` says what it waits for. */
export class PageWaits extends Error {
  reason: string;

  constructor(file: string, reason: string) {
    super(`${file}: ${reason}`);
    this.reason = reason;
  }
}

const IMPORT_LINE = /^import\s+(.+?)\s+from\s+['"]([^'"]+)['"]\s*;?\s*$/;
// A tag alone on its line: an element opens, closes, or closes itself. `<>` is a fragment.
// An attribute value in braces can hold `>`, as in `items={[<code>a</code>]}`
// (up to two levels of nested braces, as in `sx={{ "&:hover": { color: "red" } }}`).
const ATTRS = String.raw`(\s(?:[^>{]|\{(?:[^{}]|\{(?:[^{}]|\{[^{}]*\})*\})*\})*)?`;
const TAG_OPEN = new RegExp(String.raw`^\s*<([A-Za-z][\w.-]*)?${ATTRS}(?<!\/)>\s*$`);
const TAG_CLOSE = /^\s*<\/\s*([A-Za-z][\w.-]*)?\s*>\s*$/;
const TAG_SELF = new RegExp(String.raw`^\s*<[A-Za-z][\w.-]*${ATTRS}\/>\s*$`);
// An element that opens and closes on one line: `<Callout>text</Callout>`.
const TAG_INLINE = /^\s*<([A-Za-z][\w.-]*)(\s[^>]*)?>.*<\/\1>\s*$/;
const VOID = new Set(['br', 'hr', 'img', 'input', 'wbr']);
// Wrapper elements whose headings are cut, by Diataxis type (D17).
const WRAPPERS: ReadonlyMap<unknown, ReadonlySet<string>> = new Map([['how-to', new Set(['Steps'])], ['tutorial', new Set(['Steps'])]]);

/** A run of lines of a page: the intro, a section under a heading, or the text next to a wrapper tag (slug null). */
interface CutSection {
  slug: string | null;
  /** The heading line as split writes it, or null for the intro and the text next to a wrapper tag. */
  heading: string | null;
  /** The heading as a reader sees it. */
  text?: string;
  level?: number;
  lines: string[];
}

export type Section = CutSection & { slug: string };

/** A wrapper element, such as `<Steps>`, that stays in the page. */
export interface WrapperMarker {
  wrapper: 'open' | 'close';
  name: string;
}

export type Part = Section | WrapperMarker;

export const isSection = (p: Part): p is Section => !('wrapper' in p);

export interface SplitPolicy {
  /** Each component that the site gives in `mdx-components`, by name, with its import source. */
  components?: Map<string, string>;
  /** False: a page with a `mermaid` block waits (a site with no diagram renderer). */
  diagrams?: boolean;
  /** The directory that a relative import of a page resolves against. */
  importRoot?: string;
  resolveSource?: (spec: string) => string;
}

export interface CutPage {
  frontmatter: string;
  title: string;
  sections: Part[];
}

/** The local names an import clause binds: `A`, `{ B, C as D }` -> A, B, D. */
function importNames(clause: string): string[] {
  return clause.replace(/[{}]/g, ',').split(',').flatMap((part) => part.trim().split(/\s+/).pop() || []);
}

/** The text of a page outside code fences, to tell a rendered tag from one shown in an example. */
function outsideCode(lines: string[]): string {
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
 * The sections are the parts of the page in order: each section with its
 * heading, and a marker for each wrapper element.
 */
export function cutPage(raw: string, { file, policy = {} }: { file: string; policy?: SplitPolicy }): CutPage {
  const wait = (reason: string): PageWaits => new PageWaits(file, reason);
  const { frontmatter, rest } = splitFrontmatter(raw);
  const lines = rest.split('\n');
  const diataxis = frontmatter.match(/^diataxis:\s*["']?([\w-]+)/m)?.[1];
  const wrappers = WRAPPERS.get(diataxis) ?? new Set<string>();

  // Imports first: each must be a component the site gives.
  const imported: { names: string[]; spec: string; at: number }[] = [];
  const scan = fenceTracker();
  lines.forEach((line, i) => {
    if (scan(line)) return;
    if (/^export\s/.test(line)) throw wait(`line ${i + 1} is an export; a snippet cannot hold one (stage 4 for scenario data)`);
    if (!/^import\s/.test(line)) return;
    // Two imports can share a line: `import A from 'a';import B from 'b'`.
    for (const statement of line.split(/;\s*(?=import\s)/)) {
      const [, clause, spec] = statement.match(IMPORT_LINE) ?? [];
      if (clause === undefined || spec === undefined) throw wait(`line ${i + 1} is an import this tool cannot read`);
      imported.push({ names: importNames(clause), spec, at: i });
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
  const title = (body[titleAt] ?? '').slice(2).trim();

  const slugger = new GithubSlugger();
  const taken = new Set(['intro']);
  slugger.slug('intro'); // reserve the intro name, so a heading "Intro" becomes intro-1
  // The parts in order; `current` is the last section, which takes each line.
  let current: CutSection = { slug: 'intro', heading: null, level: 1, lines: [] };
  const sections: (CutSection | WrapperMarker)[] = [current];
  const add = (section: CutSection): void => { sections.push(section); current = section; };
  const fence = fenceTracker();
  const open: string[] = []; // the elements open at this line; '' is a fragment
  let comment = false; // inside a multi-line {/* ... */}
  for (const line of body.slice(titleAt + 1)) {
    const inFence = fence(line) !== null;
    if (!inFence && comment) {
      if (line.includes('*/}')) comment = false;
      current.lines.push(line);
      continue;
    }
    if (!inFence && line.includes('{/*') && !line.includes('*/}')) {
      comment = true;
      current.lines.push(line);
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
          add({ slug: null, heading: null, lines: [] }); // holds the text before the first step
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
          add({ slug: null, heading: null, lines: [] }); // holds the text after the wrapper
          continue;
        }
        open.pop();
      } else if (!TAG_SELF.test(line) && !TAG_INLINE.test(line)) {
        throw wait(`"${line.trim().slice(0, 50)}" is a tag this tool cannot follow (over more than one line, or text after the tag)`);
      }
    }
    // A heading is a cut only outside every element, or directly inside a wrapper.
    const cuttable = !inFence && (open.length === 0 || (open.length === 1 && wrappers.has(open[0] ?? '')));
    const [, hashes, headingText] = (cuttable ? line.match(CUT) : null) ?? [];
    if (hashes === undefined || headingText === undefined) {
      current.lines.push(line);
      continue;
    }
    const explicit = headingText.match(HEADING_ANCHOR);
    const text = explicit ? headingText.slice(0, explicit.index) : headingText;
    // An explicit anchor is used as it is; the slugger still records it, so a
    // later heading with the same text gets a suffix, as Nextra gives it.
    const slug = explicit ? (explicit[1] ?? explicit[2] ?? '') : slugger.slug(plainHeading(text));
    if (explicit) slugger.slug(slug);
    if (taken.has(slug)) throw wait(`two sections have the anchor #${slug}`);
    taken.add(slug);
    add({ slug, heading: `${hashes} ${text.trim()} [#${slug}]`, text: plainHeading(text), level: hashes.length, lines: [] });
  }
  if (fence.isOpen()) throw wait('a code fence is not closed');
  if (comment) throw wait('a {/* comment is not closed');
  if (open.length) throw wait(`<${open.at(-1)}> is not closed`);
  // A heading that is not a cut (inside an element, at depth 5 or 6, indented,
  // or in a blockquote) would give a snippet a second heading (D4).
  for (const sec of sections) {
    const inner = fenceTracker();
    let inComment = false;
    const extra = ('wrapper' in sec ? [] : sec.lines).find((l) => {
      if (inner(l)) return false;
      if (inComment) { inComment = !l.includes('*/}'); return false; }
      if (l.includes('{/*') && !l.includes('*/}')) { inComment = true; return false; }
      return /^\s*(>\s*)*#{1,6}\s/.test(l);
    });
    if (extra) throw wait(`"${extra.trim().slice(0, 50)}" is a heading that split does not cut (inside an element, at depth 5 or 6, indented, or quoted); the page needs its runbook form first (stage 8)`);
  }
  // Text between a wrapper tag and the next heading has no snippet of its own.
  for (const s of sections) {
    if (!('wrapper' in s) && s.slug === null && s.lines.some((l) => l.trim() !== '')) {
      throw wait('text next to a <Steps> tag and outside a step; a wrapper holds only steps');
    }
  }
  return { frontmatter, title, sections: sections.filter((s): s is Part => 'wrapper' in s || s.slug !== null) };
}

/** Lines with each top-level `mermaid` fence moved out to a diagram, and a citation in its place. */
function extractDiagrams(lines: string[], { idBase, slug }: { idBase: string; slug: string }): { lines: string[]; diagrams: { name: string; text: string }[] } {
  const out: string[] = [];
  const diagrams: { name: string; text: string }[] = [];
  const fence = fenceTracker();
  let diagram: string[] | null = null;
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
function rewriteLinks(lines: string[], resolve: (target: string) => string | undefined): string[] {
  const fence = fenceTracker();
  return lines.map((line) => {
    if (fence(line)) return line;
    return line.split(/(`[^`]*`)/).map((part, i) => (i % 2 === 1 ? part : part.replace(
      /\]\(([^)\s]+)\)/g,
      (whole: string, target: string) => {
        if (!target.startsWith('/') && !target.startsWith('#')) return whole;
        const id = resolve(target);
        return id === undefined ? whole : `](snippet:${id})`;
      },
    ))).join('');
  });
}

function trimBlank(lines: string[]): string[] {
  let a = 0;
  let b = lines.length;
  while (a < b && (lines[a] ?? '').trim() === '') a++;
  while (b > a && (lines[b - 1] ?? '').trim() === '') b--;
  return lines.slice(a, b);
}

/** The aspect of a section from its heading template, or null when no rule matches. */
function ruleAspect(section: CutSection): Aspect | null {
  return section.heading ? ASPECT_RULES.get((section.text ?? '').toLowerCase()) ?? null : null;
}

export interface SplitOptions {
  /** The directory the page files are relative to. */
  from: string;
  files: string[];
  repo: string;
  /** Where the pages go, relative to the repository root. */
  pagesOut?: string;
  policy?: SplitPolicy;
}

export interface SplitReport {
  pages: number;
  waiting: number;
  alreadySplit: number;
  snippets: number;
  diagrams: number;
  linksConverted: number;
  linksKept: number;
  unresolved: string[];
  aspectReview: number;
}

interface SplitPage extends CutPage {
  rel: string;
  route: string;
  dir: string;
  idBase: string;
  /** The parts that the page keeps: each section with a heading or text, and the wrapper markers. */
  kept: Part[];
}

/**
 * Split a set of pages. Pure apart from reading the page files. `pagesOut` is
 * `<CODEX_DIR>/pages` for codex, and the docs content directory (in place) for
 * the docs site. The paths of `writes` are relative to the repository root.
 */
export function splitPages({ from, files, repo, pagesOut = `${CODEX_DIR}/pages`, policy = {} }: SplitOptions): { writes: Map<string, string>; waits: { rel: string; reason: string }[]; report: SplitReport } {
  const waits: { rel: string; reason: string }[] = [];
  const pages: SplitPage[] = [];
  let skipped = 0;
  for (const rel of files) {
    const raw = fs.readFileSync(path.join(from, rel), 'utf-8');
    // A page that already imports snippets was split by an earlier run.
    if (/from\s+['"]@snippets\//.test(raw)) {
      skipped++;
      continue;
    }
    try {
      if (policy.components && rel.endsWith('.md')) throw new PageWaits(rel, 'a .md page; a page that imports snippets is .mdx, so rename it first');
      if (policy.diagrams === false && /^\s*(`{3,}|~{3,})\s*mermaid\s*$/m.test(raw)) throw new PageWaits(rel, 'a mermaid block, and the site has no diagram renderer yet (task 10.6)');
      const { importRoot } = policy;
      const resolveSource = importRoot ? (spec: string): string => (spec.startsWith('.')
        ? path.relative(importRoot, path.resolve(path.dirname(path.join(from, rel)), spec))
        : spec).replace(/\.(tsx?|jsx?|mjs)$/, '') : undefined;
      const cut = cutPage(raw, { file: rel, policy: { ...policy, resolveSource } });
      // An index page's snippets go under `<dir>/index/`, apart from its siblings.
      const dir = rel.replace(/\.mdx?$/, '');
      const kept = cut.sections.filter((s) => !isSection(s) || s.heading !== null || trimBlank(s.lines).length > 0);
      pages.push({ rel, route: routeOf(rel), dir, idBase: `${repo}/${dir}`, ...cut, kept });
    } catch (error) {
      if (!(error instanceof PageWaits)) throw error;
      waits.push({ rel, reason: error.reason });
    }
  }
  const byRoute = new Map(pages.map((p) => [p.route, p]));

  const report: SplitReport = { pages: pages.length, waiting: waits.length, alreadySplit: skipped, snippets: 0, diagrams: 0, linksConverted: 0, linksKept: 0, unresolved: [], aspectReview: 0 };
  const writes = new Map<string, string>();
  const write = (rel: string, text: string): void => {
    if (writes.has(rel)) throw new Error(`split writes ${rel} twice`);
    writes.set(rel, text);
  };

  for (const page of pages) {
    // A link target: undefined keeps the route (a page outside the run, or a
    // page with no intro for a link with no anchor); a missing anchor is reported.
    const resolve = (target: string): string | undefined => {
      const [route = '', anchor] = target.startsWith('#') ? [page.route, target.slice(1)] : target.slice(1).split('#');
      const tp = byRoute.get(route.replace(/\/$/, ''));
      const sections = tp?.kept.filter(isSection) ?? [];
      const hit = anchor ? sections.find((s) => s.heading && s.slug === anchor) : sections.find((s) => s.heading === null);
      if (tp && hit) {
        report.linksConverted++;
        return `${tp.idBase}/${hit.slug}`;
      }
      if (tp && anchor) report.unresolved.push(`${page.rel}: ${target}`);
      report.linksKept++;
      return undefined;
    };

    const imports: { name: string; id: string }[] = [];
    const order: string[] = []; // the page body: snippet tags and wrapper lines, in order
    for (const section of page.kept) {
      if (!isSection(section)) {
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
      const gap = body.length && section.lines[0]?.trim() === '' ? [''] : [];
      const text = [fm, ...(section.heading ? [section.heading, ...gap] : []), ...body].join('\n');
      write(`${CODEX_DIR}/snippets/${page.dir}/${section.slug}.mdx`, `${text.replace(/\n+$/, '')}\n`);
      for (const d of diagrams) {
        // A diagram and a section with one name would share an id.
        if (page.kept.some((s) => isSection(s) && s.slug === d.name)) throw new Error(`${page.rel}: diagram ${d.name} has the name of a section`);
        write(`${CODEX_DIR}/snippets/${page.dir}/${d.name}.mmd`, d.text);
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
export function metaJson(metaJs: string): string {
  const src = metaJs.replace(/^\s*export\s+default\s*/, '').replace(/;\s*$/, '');
  return `${JSON.stringify(new Function(`return (${src})`)(), null, 2)}\n`;
}

function main(argv: string[]): void {
  // A flag with no fallback is required.
  function opt(name: string): string;
  function opt<T>(name: string, fallback: T): string | T;
  function opt(name: string, fallback?: unknown): unknown {
    const at = argv.indexOf(`--${name}`);
    if (at !== -1) {
      const value = argv[at + 1];
      if (value === undefined) throw new Error(`--${name} needs a value`);
      return value;
    }
    if (fallback === undefined) throw new Error(`--${name} is required`);
    return fallback;
  }
  const out = path.resolve(opt('out', REPO_ROOT));
  const site = opt('site', 'codex');
  const repo = openRepo();
  const conf = siteConfig(repo, site);
  // codex pages come from a codex checkout and go to <CODEX_DIR>/pages; docs
  // pages are split in place, with the site's components and skips.
  const inPlace = site !== 'codex';
  const from = path.resolve(inPlace ? opt('from', path.join(repo.root, conf.pages)) : opt('from'));
  const dir = opt('dir');
  const skip = new Set(conf.skip ?? []);
  const policy = inPlace
    ? { components: new Map(Object.entries(conf.components ?? {})), diagrams: conf.diagrams !== false, importRoot: path.dirname(from) }
    : {};

  const files: string[] = [];
  const metas: string[] = [];
  const walk = (rel: string): void => {
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
    // Several directories (or single page files) split as one run, so their links can name each other.
    if (fs.statSync(path.join(from, d)).isFile()) files.push(d);
    else walk(d);
  }
  // A page that stays whole (the manifest `whole` list) is not split, even when its text would cut.
  const whole = new Set(inPlace ? conf.whole ?? [] : []);
  files.splice(0, files.length, ...files.filter((f) => !whole.has(f)));
  files.sort();

  const { writes, waits, report } = splitPages({ from, files, repo: repo.repo, pagesOut: inPlace ? conf.pages : `${CODEX_DIR}/pages`, policy });
  // A docs page keeps its _meta.js in place; codex pages get _meta.json from it.
  if (!inPlace) for (const m of metas.sort()) writes.set(`${CODEX_DIR}/pages/${m.replace(/_meta\.js$/, '_meta.json')}`, metaJson(fs.readFileSync(path.join(from, m), 'utf-8')));

  for (const [rel, text] of writes) {
    const target = path.join(out, rel);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, text);
  }
  console.log(`${report.pages} pages -> ${report.snippets} snippets, ${report.diagrams} diagrams${inPlace ? '' : `, ${metas.length} _meta.json`}; ${report.waiting} pages wait; ${report.alreadySplit} already split`);
  console.log(`links: ${report.linksConverted} converted to snippet:, ${report.linksKept} kept as routes, ${report.unresolved.length} with an anchor that no section has`);
  console.log(`aspect: ${report.snippets - report.aspectReview} from a rule, ${report.aspectReview} marked aspect_review`);
  for (const u of report.unresolved) console.log(`  ${u}`);
  if (waits.length) console.log('pages that wait:');
  for (const w of waits) console.log(`  ${w.rel}: ${w.reason}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2));
}
