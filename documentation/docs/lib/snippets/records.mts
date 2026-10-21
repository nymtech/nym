// Record builder and page check for one repository's `.nym-codex/`.
//
//   node lib/snippets/records.mts [--root <repo root>] [--out <dir>]
//
// Reads the snippets and the pages of each site in the manifest, checks them,
// and builds one record for each snippet. With --out, writes prose.jsonl and
// diagram.jsonl there, one record on each line, sorted by id. Prints every
// error and warning; exits 1 on an error.

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseFrontmatter, HEADING_ANCHOR, CITATION, fenceTracker, frontmatterLines } from '../retrieval/mdx.mjs';
import type { Authority, RefRel, Snippet, SnippetRef } from '../retrieval/types.ts';
import type { Repo, SnippetImport } from './repo.mts';
import { openRepo, snippetImports, pageFiles, citationText, pageLinks, CODEX_DIR } from './repo.mts';
import type { CodeRecord } from './extract.mts';
import { extractCode } from './extract.mts';
import type { PathCheck } from './paths.mts';
import { checkPaths } from './paths.mts';
import { buildThreatModel } from './threat-model.mts';
import { buildRequirements } from './openspec.mts';
import { loadEdges, applyEdges } from './edges.mts';
import { lintSnippets } from './standalone.mts';

const RELS: ReadonlySet<string> = new Set<RefRel>(['cites', 'relates-to', 'depends-on', 'realises', 'specialises', 'observes', 'mitigates', 'assessed-against']);
const isRel = (x: unknown): x is RefRel => typeof x === 'string' && RELS.has(x);
// The indexer splits a chunk at this size, so a body over it is more than one chunk.
const MAX_BODY = 2400;
// Wrapper elements a page may hold, by Diataxis type (D17).
const WRAPPERS: ReadonlyMap<unknown, ReadonlySet<string>> = new Map([['how-to', new Set(['Steps'])], ['tutorial', new Set(['Steps'])]]);

const LINK = /\]\(snippet:([^)\s]+)\)/g;
const HEADING = /^(#{1,6})\s+(.*)$/;
const IMPORT = /^import\s+[\w{},*\s]+\s+from\s+['"]/;

const sha256 = (text: string): string => crypto.createHash('sha256').update(text).digest('hex');
const byId = (a: { id: string }, b: { id: string }): number => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0);
const anchorOf = (text: string): string | null => { const m = text.match(HEADING_ANCHOR); return m ? (m[1] ?? m[2] ?? null) : null; };
const messageOf = (error: unknown): string => (error instanceof Error ? error.message : String(error));

// A record gets its authority after the other fields, so that key comes last in
// the JSON, as it always has.
type ProseDraft = Omit<Snippet, 'authority'>;
type DiagramDraft = Omit<Snippet, 'authority'>;

interface ProseLine {
  line: string;
  n: number;
  setext?: boolean;
}

/** What the page check needs from each prose file. */
interface SnippetInfo {
  anchor: string | null;
  fmTitle: string | null;
  links: { target: string; n: number }[];
  where: string;
  components: string[];
}

interface CheckedPage {
  site: string;
  file: string;
  imports: SnippetImport[];
  diataxis: unknown;
}

export interface Counts {
  snippets: number;
  diagrams: number;
  pages: number;
  code: number;
}

export interface BuiltRecords {
  prose: Snippet[];
  diagram: Snippet[];
  code: Snippet[];
  errors: string[];
  warnings: string[];
  counts: Counts;
}

/**
 * Lines outside code fences, with their 1-indexed line numbers. A setext
 * heading (a text line, then `===` or `---`) is marked, so it counts as a heading.
 */
function proseLines(text: string): { lines: ProseLine[]; unclosedAt: number | null } {
  const out: ProseLine[] = [];
  let openedAt = 0;
  const fence = fenceTracker();
  // A `{/* ... */}` block over several lines is not rendered, so its lines are not prose (split skips them too).
  let comment = false;
  text.split('\n').forEach((line, i) => {
    if (fence(line)) return;
    if (comment) {
      if (line.includes('*/}')) comment = false;
      return;
    }
    if (line.includes('{/*') && !line.includes('*/}')) {
      comment = true;
      openedAt = i + 1;
      return;
    }
    const prev = out.at(-1);
    if (/^(=+|-+)\s*$/.test(line) && prev && prev.n === i && prev.line.trim() !== '' && !HEADING.test(prev.line)) prev.setext = true;
    out.push({ line, n: i + 1 });
  });
  return { lines: out, unclosedAt: comment ? openedAt : null };
}

function walkSnippets(dir: string): string[] {
  const out: string[] = [];
  const walk = (rel: string): void => {
    for (const e of fs.readdirSync(path.join(dir, rel), { withFileTypes: true })) {
      const r = path.join(rel, e.name);
      if (e.isDirectory()) walk(r);
      else if (/\.(mdx|mmd)$/.test(e.name)) out.push(r);
    }
  };
  if (fs.existsSync(dir)) walk('');
  return out.sort();
}

export interface BuildOptions {
  code?: Map<string, CodeRecord>;
  /** The code citations that the extractor reported. */
  failed?: Set<string>;
  /** The threat-model record ids, so a model citation is checked against them. */
  models?: Set<string>;
}

export function buildRecords(repo: Repo, { code = new Map(), failed = new Set(), models }: BuildOptions = {}): BuiltRecords {
  const errors: string[] = [];
  const warnings: string[] = [];
  const rel = (f: string): string => path.relative(repo.root, f);
  const files = walkSnippets(repo.snippets);
  const own = (id: string): boolean => id.startsWith(`${repo.repo}/`);

  // Ids first, so every later check can tell prose from diagram.
  const kinds = new Map<string, 'prose' | 'diagram'>();
  for (const f of files) {
    const id = `${repo.repo}/${f.replace(/\.(mdx|mmd)$/, '')}`;
    const kind = f.endsWith('.mdx') ? 'prose' : 'diagram';
    if (kinds.has(id)) errors.push(`${rel(path.join(repo.snippets, f))}: duplicate id ${id}`);
    else kinds.set(id, kind);
  }

  const proseDrafts: ProseDraft[] = [];
  const diagramDrafts: DiagramDraft[] = [];
  const info = new Map<string, SnippetInfo>();
  for (const f of files) {
    const file = path.join(repo.snippets, f);
    const id = `${repo.repo}/${f.replace(/\.(mdx|mmd)$/, '')}`;
    const raw = fs.readFileSync(file, 'utf-8');
    if (f.endsWith('.mmd')) {
      diagramDrafts.push({ id, kind: 'diagram', repo: repo.repo, path: rel(file), title: path.basename(f, '.mmd'), lang: 'mermaid', body: raw, hash: sha256(raw), refs: [], meta: {} });
      continue;
    }
    const where = rel(file);
    // The frontmatter is untyped YAML: each field is checked where it is read.
    const { data, content }: { data: Record<string, unknown>; content: string } = parseFrontmatter(raw);
    const fmTitle = typeof data.title === 'string' ? data.title : undefined;
    if (data.title !== undefined && fmTitle === undefined) errors.push(`${where}: the frontmatter title is not a string`);
    const all = content.split('\n');
    // Line numbers in messages count from the top of the file, frontmatter included.
    const offset: number = frontmatterLines(raw, content);
    const prose = proseLines(content);
    if (prose.unclosedAt !== null) errors.push(`${where}:${prose.unclosedAt + offset}: a {/* comment is not closed`);
    const lines = prose.lines.map((l) => ({ ...l, n: l.n + offset }));
    const headings = lines.filter((l) => HEADING.test(l.line) || l.setext);
    const firstAt = all.findIndex((l) => l.trim() !== '') + 1 + offset;

    let title = fmTitle ?? '';
    let anchor: string | null = null;
    let body = content.replace(/^\n+/, '');
    const [h, second] = headings;
    if (second) errors.push(`${where}:${second.n}: a second heading; a snippet has one heading at most`);
    if (h?.setext) {
      errors.push(`${where}:${h.n}: a setext heading; write it as "## Title [#anchor]"`);
    } else if (h) {
      if (h.n !== firstAt) errors.push(`${where}:${h.n}: the heading is not on the first line of the body`);
      const text = h.line.match(HEADING)?.[2] ?? '';
      anchor = anchorOf(text);
      const stem = path.basename(f, '.mdx');
      if (anchor !== stem) errors.push(`${where}:${h.n}: heading anchor ${anchor ? `[#${anchor}]` : '(none)'} is not the file name [#${stem}]`);
      if (fmTitle !== undefined) errors.push(`${where}: a frontmatter title next to a heading; the title has one source`);
      title = text.replace(HEADING_ANCHOR, '').trim();
      body = all.slice(h.n - offset).join('\n').replace(/^\n+/, '');
    } else if (fmTitle === undefined) {
      errors.push(`${where}: no heading and no frontmatter title`);
    }
    body = body.replace(/\n+$/, '');

    for (const l of lines) {
      if (IMPORT.test(l.line)) errors.push(`${where}:${l.n}: an import; a snippet imports no file`);
    }

    if (body.length > MAX_BODY) warnings.push(`${where}: body is ${body.length} characters, over ${MAX_BODY}; the indexer splits it`);

    const refs: SnippetRef[] = [];
    const links: SnippetInfo['links'] = [];
    for (const l of lines) {
      for (const [, target = '', attrs = ''] of l.line.matchAll(CITATION)) {
        if (!own(target)) {
          warnings.push(`${where}:${l.n}: cites ${target} from another repository; the codex build checks it`);
          refs.push({ id: target, rel: 'cites' });
          continue;
        }
        if (failed.has(target)) continue; // the extractor reported it, at this citation
        try {
          citationText(repo, target, { code, attrs, models }); // the same resolver that composes the page
          refs.push({ id: target, rel: 'cites' });
        } catch (error) {
          errors.push(`${where}:${l.n}: ${messageOf(error)}`);
        }
      }
      for (const [, target = ''] of l.line.matchAll(LINK)) {
        if (!own(target)) warnings.push(`${where}:${l.n}: snippet:${target} is in another repository; the codex build checks it`);
        else if (kinds.get(target) === 'diagram') errors.push(`${where}:${l.n}: snippet:${target} names a diagram; a link names a prose snippet`);
        else if (!kinds.has(target)) errors.push(`${where}:${l.n}: snippet:${target} has no snippet`);
        refs.push({ id: target, rel: 'relates-to' });
        links.push({ target, n: l.n });
      }
    }
    if (data.refs !== undefined && !Array.isArray(data.refs)) errors.push(`${where}: refs is not a list`);
    for (const entry of Array.isArray(data.refs) ? data.refs : []) {
      const ref: { id?: unknown; rel?: unknown } = typeof entry === 'string' ? { id: entry, rel: 'relates-to' } : (entry ?? {});
      if (ref.rel === 'cites') errors.push(`${where}: a cites edge in frontmatter; the tools write cites from <Snippet> citations`);
      else if (!isRel(ref.rel)) errors.push(`${where}: relation "${ref.rel}" is not one of ${[...RELS].join(', ')}`);
      else if (typeof ref.id !== 'string') errors.push(`${where}: an edge in refs has no id`);
      else if (own(ref.id) && !kinds.has(ref.id)) errors.push(`${where}: edge to ${ref.id}, which has no snippet in this repository`);
      else refs.push({ id: ref.id, rel: ref.rel });
    }

    // The JSX elements the body renders, which each site that shows it must give (D18).
    // Inline code is not JSX, so `<PATH>` in backticks is not a component.
    const components = [...new Set(lines.flatMap((l) => [...l.line.replace(/`[^`]*`/g, '').matchAll(/<([A-Z][\w]*)[\s/>.]/g)].flatMap((m) => m[1] ?? [])))].filter((c) => c !== 'Snippet').sort();
    info.set(id, { anchor, fmTitle: headings.length === 0 ? fmTitle ?? null : null, links, where, components });
    const { title: _t, refs: _r, ...rest } = data;
    const meta = components.length ? { ...rest, components } : rest;
    proseDrafts.push({ id, kind: 'prose', repo: repo.repo, path: where, title, body, hash: sha256(body), refs: normaliseRefs(refs), meta });
  }

  const cites = (p: { refs: SnippetRef[] }, id: string): boolean => p.refs.some((r) => r.id === id && r.rel === 'cites');

  const pages = checkPages(repo, { kinds, info, errors, warnings });

  // Authority (P9): prose on a how-to or tutorial page illustrates; other prose
  // describes. A diagram takes the authority of the prose that cites it.
  const guide = new Set(pages.filter((p) => p.diataxis === 'how-to' || p.diataxis === 'tutorial').flatMap((p) => p.imports.map((i) => i.id)));
  const prose: Snippet[] = proseDrafts.map((p) => {
    const authority: Authority = guide.has(p.id) ? 'illustrative' : 'descriptive';
    return Object.assign(p, { authority });
  });
  const diagram: Snippet[] = diagramDrafts.map((d) => {
    const authority: Authority = prose.find((p) => cites(p, d.id))?.authority ?? 'descriptive';
    return Object.assign(d, { authority });
  });
  const authority: Authority = 'descriptive';
  const codeRecords: Snippet[] = [...code.values()].map((c) => Object.assign(c, { authority }));

  // In use: a page imports it, or an imported prose snippet cites it.
  const imported = new Set(pages.flatMap((p) => p.imports.map((i) => i.id)));
  const cited = new Set(prose.filter((p) => imported.has(p.id)).flatMap((p) => p.refs.filter((r) => r.rel === 'cites').map((r) => r.id)));
  for (const [id, kind] of kinds) {
    if (!imported.has(id) && !cited.has(id)) errors.push(`${id}: not in use; no page imports it and no imported snippet cites it`);
    if (kind === 'diagram' && imported.has(id)) errors.push(`${id}: a page imports a diagram; a prose snippet cites it`);
  }

  prose.sort(byId);
  diagram.sort(byId);
  const counts: Counts = {
    snippets: prose.length,
    diagrams: diagram.length,
    pages: pages.length,
    code: code.size,
  };
  return { prose, diagram, code: codeRecords.sort(byId), errors, warnings, counts };
}

function normaliseRefs(refs: SnippetRef[]): SnippetRef[] {
  const seen = new Set<string>();
  return refs
    .filter((r) => { const k = `${r.id}\0${r.rel}`; if (seen.has(k)) return false; seen.add(k); return true; })
    .sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : a.rel < b.rel ? -1 : a.rel > b.rel ? 1 : 0));
}

interface PageCheckInput {
  kinds: Map<string, 'prose' | 'diagram'>;
  info: Map<string, SnippetInfo>;
  errors: string[];
  warnings: string[];
}

/**
 * The page check (D9): a page holds frontmatter, one `# ` line, snippet
 * imports, one tag for each import, and the wrapper its archetype names.
 */
function checkPages(repo: Repo, { kinds, info, errors, warnings }: PageCheckInput): CheckedPage[] {
  const pages: CheckedPage[] = [];
  for (const [site, conf] of Object.entries(repo.manifest.sites ?? {})) {
    const pagesDir = path.join(repo.root, conf.pages);
    const siteLinks = pageLinks(repo, site);
    const reported = new Set<string>();
    // A site that lists its components checks each snippet against the list.
    const given = conf.components ? new Set(Object.keys(conf.components)) : null;
    for (const relPage of pageFiles(pagesDir)) {
      const file = path.join(pagesDir, relPage);
      const where = path.relative(repo.root, file);
      const raw = fs.readFileSync(file, 'utf-8');
      const { data, content }: { data: Record<string, unknown>; content: string } = parseFrontmatter(raw);
      const offset: number = frontmatterLines(raw, content);
      const imports = snippetImports(content).map((i) => ({ ...i, line: i.line + offset }));
      // On a site split in place, a page that imports no snippet is a whole page
      // that is not split yet (D18), not a fault. A page under CODEX_DIR is always checked.
      if (imports.length === 0 && !conf.pages.startsWith(`${CODEX_DIR}/`)) continue;
      const own = imports.filter((i) => i.id.startsWith(`${repo.repo}/`));
      for (const i of imports) if (!own.includes(i)) warnings.push(`${where}:${i.line}: ${i.id} is from another repository; the codex build checks it`);
      const names = new Map<string, SnippetImport>();
      for (const i of imports) {
        if (names.has(i.name)) errors.push(`${where}:${i.line}: the import name ${i.name} is used twice`);
        names.set(i.name, i);
      }
      const wrappers = WRAPPERS.get(data.diataxis) ?? new Set<string>();
      // A page that stays whole holds its own text; only its imports are checked.
      const whole = (conf.whole ?? []).includes(relPage);

      let titles = 0;
      let pageTitle: string | null = null;
      const used = new Map<string, number>();
      if (!whole) content.split('\n').forEach((line, i) => {
        const n = i + 1 + offset;
        if (line.trim() === '' || /^import\s+\w+\s+from\s+['"]@snippets\//.test(line)) return;
        if (/^# /.test(line)) {
          titles++;
          pageTitle = line.slice(2).trim();
          return;
        }
        const tag = line.match(/^\s*<([A-Za-z][\w]*)\s*\/>\s*$/)?.[1];
        if (tag !== undefined && names.has(tag)) {
          used.set(tag, (used.get(tag) ?? 0) + 1);
          return;
        }
        const wrap = line.match(/^\s*<\/?([A-Za-z]\w*)>\s*$/)?.[1];
        if (wrap !== undefined && wrappers.has(wrap)) return;
        errors.push(`${where}:${n}: content with no snippet; a page holds only imports, tags, and its title`);
      });
      if (!whole && titles !== 1) errors.push(`${where}: ${titles} "# " title lines; a page has one`);
      // A whole page is the route of a snippet it imports, so it must render it.
      if (whole) for (const i of imports) if (!new RegExp(`<${i.name}[\\s/>]`).test(content)) errors.push(`${where}:${i.line}: ${i.name} is imported and not rendered`);
      for (const i of whole ? [] : imports) {
        if ((used.get(i.name) ?? 0) !== 1) errors.push(`${where}:${i.line}: ${i.name} is imported once and rendered ${used.get(i.name) ?? 0} times`);
      }

      const anchors = new Map<string, string>();
      for (const i of own) {
        const kind = kinds.get(i.id);
        if (kind === undefined) {
          errors.push(`${where}:${i.line}: imports ${i.id}, which has no snippet file`);
          continue;
        }
        // Only a prose snippet has an entry; a diagram import is an error above.
        const snippet = info.get(i.id);
        if (!snippet) continue;
        for (const c of snippet.components) {
          if (given && !given.has(c)) errors.push(`${where}:${i.line}: ${i.id} renders <${c}>, which the site "${site}" does not give`);
        }
        // A link resolves on each site that shows the snippet (spec: "A link names a snippet id").
        for (const { target, n } of snippet.links) {
          const key = `${site}\0${snippet.where}\0${n}\0${target}`;
          if (target.startsWith(`${repo.repo}/`) && kinds.get(target) === 'prose' && !siteLinks.has(target) && !reported.has(key)) {
            reported.add(key);
            errors.push(`${snippet.where}:${n}: snippet:${target} is on no page of the site "${site}"`);
          }
        }
        const { anchor, fmTitle } = snippet;
        if (anchor) {
          if (anchors.has(anchor)) errors.push(`${where}:${i.line}: anchor #${anchor} is on this page twice (${anchors.get(anchor)} and ${i.id})`);
          anchors.set(anchor, i.id);
        }
        // P26: the title of a first snippet with no heading is the page title.
        if (i === own[0] && fmTitle && pageTitle !== null && fmTitle !== pageTitle) {
          errors.push(`${where}:${i.line}: the first snippet title "${fmTitle}" is not the page title "${pageTitle}" (P26)`);
        }
      }
      pages.push({ site, file, imports: own, diataxis: data.diataxis });
    }
  }
  return pages;
}

export interface AllRecords extends BuiltRecords {
  /** The OpenSpec requirement records, which are prose too and go to prose.jsonl with them. */
  requirement: Snippet[];
  model: Snippet[];
  paths: PathCheck;
}

/**
 * Every record of the repository: the snippets (prose, diagram, code), the
 * threat model, the OpenSpec requirements, and the authored edges on them.
 * `prose` holds the authored prose records.
 */
export async function buildAll(repo: Repo): Promise<AllRecords> {
  const extracted = await extractCode(repo);
  const model = await buildThreatModel({ repo: repo.repo });
  const built = buildRecords(repo, { code: extracted.code, failed: extracted.failed, models: new Set(model.records.map((r) => r.id)) });
  const requirements = buildRequirements(repo);
  const errors = [
    ...extracted.errors,
    ...built.errors,
    ...model.errors.map((e) => `threat model: ${e}`),
    ...requirements.errors,
  ];
  const warnings = [...built.warnings, ...model.warnings.map((w) => `threat model: ${w}`)];
  const all = [...built.prose, ...requirements.records, ...built.diagram, ...built.code, ...model.records];
  errors.push(...applyEdges(loadEdges(repo), all));
  // Inline source paths: a warning until stage 2 ends (D14).
  const paths = checkPaths(built.prose.map((p) => path.join(repo.root, p.path)), repo.root);
  for (const [p, where] of paths.missing) warnings.push(`${where[0]}: inline path ${p} does not exist${where.length > 1 ? ` (and ${where.length - 1} more)` : ''}`);
  return { ...built, requirement: requirements.records, model: model.records, errors, warnings, paths };
}

async function main(argv: string[]): Promise<void> {
  const at = (name: string): string | null => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1] ?? null; };
  const root = at('root');
  const repo = openRepo(root ? path.resolve(root) : undefined);
  const out = at('out');
  const { prose, requirement, diagram, code, model, errors, warnings, counts, paths } = await buildAll(repo);
  for (const e of errors) console.log(`error  ${e}`);
  for (const w of warnings) console.log(`warn   ${w}`);
  const links = prose.flatMap((p) => p.refs.filter((r) => r.rel === 'relates-to')).length;
  console.log(`${counts.pages} pages, ${counts.snippets} prose snippets, ${counts.diagrams} diagrams, ${counts.code} code items, ${model.length} model records, ${requirement.length} requirements`);
  console.log(`snippet: links ${links}; inline paths ${paths.distinct - paths.missing.size} of ${paths.distinct} exist`);
  // The standalone lint is a count until stage 3 ends (task 4b.4); `standalone.mts --list` names each fault.
  const standalone = lintSnippets(repo.snippets);
  const clean = standalone.filter((r) => r.faults.length === 0).length;
  console.log(`standalone: ${clean} of ${standalone.length} prose snippets have no fault (standalone.mts --list names the rest)`);
  console.log(`${errors.length} errors, ${warnings.length} warnings`);
  if (out && errors.length === 0) {
    fs.mkdirSync(out, { recursive: true });
    const jsonl = (records: Snippet[]): string => records.map((r) => JSON.stringify(r)).join('\n') + (records.length ? '\n' : '');
    fs.writeFileSync(path.join(out, 'prose.jsonl'), jsonl([...prose, ...requirement].sort(byId)));
    fs.writeFileSync(path.join(out, 'diagram.jsonl'), jsonl(diagram));
    fs.writeFileSync(path.join(out, 'code.jsonl'), jsonl(code));
    fs.writeFileSync(path.join(out, 'model.jsonl'), jsonl(model));
    console.log(`wrote prose.jsonl, diagram.jsonl, code.jsonl, and model.jsonl to ${out}`);
  }
  process.exitCode = errors.length > 0 ? 1 : 0;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}
