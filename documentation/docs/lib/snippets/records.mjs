// Record builder and page check for one repository's `.nym-codex/`.
//
//   node lib/snippets/records.mjs [--root <repo root>] [--out <dir>]
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
import { openRepo, snippetImports, pageFiles, citationText, pageLinks, CODEX_DIR } from './repo.mjs';
import { extractCode } from './extract.mjs';
import { checkPaths } from './paths.mjs';
import { buildThreatModel } from './threat-model.mjs';
import { buildRequirements } from './openspec.mjs';
import { loadEdges, applyEdges } from './edges.mjs';
import { lintSnippets } from './standalone.mjs';

const ASPECTS = new Set(['motivation', 'requirement', 'design', 'implementation', 'operation']);
const RELS = new Set(['cites', 'relates-to', 'depends-on', 'realises', 'specialises', 'observes', 'mitigates', 'assessed-against']);
// The indexer splits a chunk at this size, so a body over it is more than one chunk.
const MAX_BODY = 2400;
// Wrapper elements a page may hold, by Diataxis type (D17).
const WRAPPERS = { 'how-to': new Set(['Steps']), tutorial: new Set(['Steps']) };

const LINK = /\]\(snippet:([^)\s]+)\)/g;
const HEADING = /^(#{1,6})\s+(.*)$/;
const IMPORT = /^import\s+[\w{},*\s]+\s+from\s+['"]/;

const sha256 = (text) => crypto.createHash('sha256').update(text).digest('hex');
const byId = (a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0);
const anchorOf = (text) => { const m = text.match(HEADING_ANCHOR); return m ? (m[1] ?? m[2]) : null; };

/**
 * Lines outside code fences, with their 1-indexed line numbers. A setext
 * heading (a text line, then `===` or `---`) is marked, so it counts as a heading.
 */
function proseLines(text) {
  const out = [];
  const fence = fenceTracker();
  text.split('\n').forEach((line, i) => {
    if (fence(line)) return;
    const prev = out.at(-1);
    if (/^(=+|-+)\s*$/.test(line) && prev && prev.n === i && prev.line.trim() !== '' && !HEADING.test(prev.line)) prev.setext = true;
    out.push({ line, n: i + 1 });
  });
  return out;
}

function walkSnippets(dir) {
  const out = [];
  const walk = (rel) => {
    for (const e of fs.readdirSync(path.join(dir, rel), { withFileTypes: true })) {
      const r = path.join(rel, e.name);
      if (e.isDirectory()) walk(r);
      else if (/\.(mdx|mmd)$/.test(e.name)) out.push(r);
    }
  };
  if (fs.existsSync(dir)) walk('');
  return out.sort();
}

/**
 * @returns {{ prose: object[], diagram: object[], errors: string[], warnings: string[], counts: object }}
 */
export function buildRecords(repo, { code = new Map(), failed = new Set() } = {}) {
  const errors = [];
  const warnings = [];
  const rel = (f) => path.relative(repo.root, f);
  const files = walkSnippets(repo.snippets);
  const own = (id) => id.startsWith(`${repo.repo}/`);

  // Ids first, so every later check can tell prose from diagram.
  const kinds = new Map();
  for (const f of files) {
    const id = `${repo.repo}/${f.replace(/\.(mdx|mmd)$/, '')}`;
    const kind = f.endsWith('.mdx') ? 'prose' : 'diagram';
    if (kinds.has(id)) errors.push(`${rel(path.join(repo.snippets, f))}: duplicate id ${id}`);
    else kinds.set(id, kind);
  }

  const prose = [];
  const diagram = [];
  // id -> { anchor, fmTitle, links }: what the page check needs from each prose file.
  const info = new Map();
  for (const f of files) {
    const file = path.join(repo.snippets, f);
    const id = `${repo.repo}/${f.replace(/\.(mdx|mmd)$/, '')}`;
    const raw = fs.readFileSync(file, 'utf-8');
    if (f.endsWith('.mmd')) {
      diagram.push({ id, kind: 'diagram', repo: repo.repo, path: rel(file), title: path.basename(f, '.mmd'), lang: 'mermaid', body: raw, hash: sha256(raw), refs: [], meta: {} });
      continue;
    }
    const where = rel(file);
    const { data, content } = parseFrontmatter(raw);
    const all = content.split('\n');
    // Line numbers in messages count from the top of the file, frontmatter included.
    const offset = frontmatterLines(raw, content);
    const lines = proseLines(content).map((l) => ({ ...l, n: l.n + offset }));
    const headings = lines.filter((l) => HEADING.test(l.line) || l.setext);
    const firstAt = all.findIndex((l) => l.trim() !== '') + 1 + offset;

    let title = data.title;
    let anchor = null;
    let body = content.replace(/^\n+/, '');
    if (headings.length > 1) errors.push(`${where}:${headings[1].n}: a second heading; a snippet has one heading at most`);
    const [h] = headings;
    if (h?.setext) {
      errors.push(`${where}:${h.n}: a setext heading; write it as "## Title [#anchor]"`);
    } else if (h) {
      if (h.n !== firstAt) errors.push(`${where}:${h.n}: the heading is not on the first line of the body`);
      const text = h.line.match(HEADING)[2];
      anchor = anchorOf(text);
      const stem = path.basename(f, '.mdx');
      if (anchor !== stem) errors.push(`${where}:${h.n}: heading anchor ${anchor ? `[#${anchor}]` : '(none)'} is not the file name [#${stem}]`);
      if (data.title !== undefined) errors.push(`${where}: a frontmatter title next to a heading; the title has one source`);
      title = text.replace(HEADING_ANCHOR, '').trim();
      body = all.slice(h.n - offset).join('\n').replace(/^\n+/, '');
    } else if (data.title === undefined) {
      errors.push(`${where}: no heading and no frontmatter title`);
    }
    body = body.replace(/\n+$/, '');

    for (const l of lines) {
      if (IMPORT.test(l.line)) errors.push(`${where}:${l.n}: an import; a snippet imports no file`);
    }

    if (data.aspect === undefined) errors.push(`${where}: no aspect`);
    else if (!ASPECTS.has(data.aspect)) errors.push(`${where}: aspect "${data.aspect}" is not one of ${[...ASPECTS].join(', ')}`);
    if (body.length > MAX_BODY) warnings.push(`${where}: body is ${body.length} characters, over ${MAX_BODY}; the indexer splits it`);

    const refs = [];
    const links = [];
    for (const l of lines) {
      for (const [, target] of l.line.matchAll(CITATION)) {
        if (!own(target)) {
          warnings.push(`${where}:${l.n}: cites ${target} from another repository; the codex build checks it`);
          refs.push({ id: target, rel: 'cites' });
          continue;
        }
        if (failed.has(target)) continue; // the extractor reported it, at this citation
        try {
          citationText(repo, target, { code }); // the same resolver that composes the page
          refs.push({ id: target, rel: 'cites' });
        } catch (error) {
          errors.push(`${where}:${l.n}: ${error.message}`);
        }
      }
      for (const [, target] of l.line.matchAll(LINK)) {
        if (!own(target)) warnings.push(`${where}:${l.n}: snippet:${target} is in another repository; the codex build checks it`);
        else if (kinds.get(target) === 'diagram') errors.push(`${where}:${l.n}: snippet:${target} names a diagram; a link names a prose snippet`);
        else if (!kinds.has(target)) errors.push(`${where}:${l.n}: snippet:${target} has no snippet`);
        refs.push({ id: target, rel: 'relates-to' });
        links.push({ target, n: l.n });
      }
    }
    if (data.refs !== undefined && !Array.isArray(data.refs)) errors.push(`${where}: refs is not a list`);
    for (const entry of Array.isArray(data.refs) ? data.refs : []) {
      const ref = typeof entry === 'string' ? { id: entry, rel: 'relates-to' } : entry;
      if (ref.rel === 'cites') errors.push(`${where}: a cites edge in frontmatter; the tools write cites from <Snippet> citations`);
      else if (!RELS.has(ref.rel)) errors.push(`${where}: relation "${ref.rel}" is not one of ${[...RELS].join(', ')}`);
      else if (own(ref.id) && !kinds.has(ref.id)) errors.push(`${where}: edge to ${ref.id}, which has no snippet in this repository`);
      else refs.push({ id: ref.id, rel: ref.rel });
    }

    // The JSX elements the body renders, which each site that shows it must give (D18).
    // Inline code is not JSX, so `<PATH>` in backticks is not a component.
    const components = [...new Set(lines.flatMap((l) => [...l.line.replace(/`[^`]*`/g, '').matchAll(/<([A-Z][\w]*)[\s/>.]/g)].map((m) => m[1])))].filter((c) => c !== 'Snippet').sort();
    info.set(id, { anchor, fmTitle: headings.length === 0 ? data.title ?? null : null, links, where, components });
    const { aspect, title: _t, refs: _r, ...rest } = data;
    const meta = components.length ? { ...rest, components } : rest;
    prose.push({ id, kind: 'prose', aspect, repo: repo.repo, path: where, title, body, hash: sha256(body), refs: normaliseRefs(refs), meta });
  }

  // A diagram takes the aspect of the first prose snippet that cites it, in
  // snippet file path order.
  for (const d of diagram) {
    const citer = prose.find((p) => p.refs.some((r) => r.id === d.id && r.rel === 'cites'));
    d.aspect = citer?.aspect ?? 'design';
  }

  const pages = checkPages(repo, { kinds, info, errors, warnings });

  // In use: a page imports it, or an imported prose snippet cites it.
  const imported = new Set(pages.flatMap((p) => p.imports.map((i) => i.id)));
  const cited = new Set(prose.filter((p) => imported.has(p.id)).flatMap((p) => p.refs.filter((r) => r.rel === 'cites').map((r) => r.id)));
  for (const [id, kind] of kinds) {
    if (!imported.has(id) && !cited.has(id)) errors.push(`${id}: not in use; no page imports it and no imported snippet cites it`);
    if (kind === 'diagram' && imported.has(id)) errors.push(`${id}: a page imports a diagram; a prose snippet cites it`);
  }

  prose.sort(byId);
  diagram.sort(byId);
  const counts = {
    snippets: prose.length,
    diagrams: diagram.length,
    pages: pages.length,
    aspectReview: prose.filter((p) => p.meta.aspect_review).length,
  };
  counts.code = code.size;
  return { prose, diagram, code: [...code.values()].sort(byId), errors, warnings, counts };
}

function normaliseRefs(refs) {
  const seen = new Set();
  return refs
    .filter((r) => { const k = `${r.id}\0${r.rel}`; if (seen.has(k)) return false; seen.add(k); return true; })
    .sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : a.rel < b.rel ? -1 : a.rel > b.rel ? 1 : 0));
}

/**
 * The page check (D9): a page holds frontmatter, one `# ` line, snippet
 * imports, one tag for each import, and the wrapper its archetype names.
 */
function checkPages(repo, { kinds, info, errors, warnings }) {
  const pages = [];
  for (const [site, conf] of Object.entries(repo.manifest.sites ?? {})) {
    const pagesDir = path.join(repo.root, conf.pages);
    const siteLinks = pageLinks(repo, site);
    const reported = new Set();
    // A site that lists its components checks each snippet against the list.
    const given = conf.components ? new Set(Object.keys(conf.components)) : null;
    for (const relPage of pageFiles(pagesDir)) {
      const file = path.join(pagesDir, relPage);
      const where = path.relative(repo.root, file);
      const raw = fs.readFileSync(file, 'utf-8');
      const { data, content } = parseFrontmatter(raw);
      const offset = frontmatterLines(raw, content);
      const imports = snippetImports(content).map((i) => ({ ...i, line: i.line + offset }));
      // On a site split in place, a page that imports no snippet is a whole page
      // that is not split yet (D18), not a fault. A page under CODEX_DIR is always checked.
      if (imports.length === 0 && !conf.pages.startsWith(`${CODEX_DIR}/`)) continue;
      const own = imports.filter((i) => i.id.startsWith(`${repo.repo}/`));
      for (const i of imports) if (!own.includes(i)) warnings.push(`${where}:${i.line}: ${i.id} is from another repository; the codex build checks it`);
      const names = new Map();
      for (const i of imports) {
        if (names.has(i.name)) errors.push(`${where}:${i.line}: the import name ${i.name} is used twice`);
        names.set(i.name, i);
      }
      const wrappers = WRAPPERS[data.diataxis] ?? new Set();

      let titles = 0;
      let pageTitle = null;
      const used = new Map();
      content.split('\n').forEach((line, i) => {
        const n = i + 1 + offset;
        if (line.trim() === '' || /^import\s+\w+\s+from\s+['"]@snippets\//.test(line)) return;
        if (/^# /.test(line)) {
          titles++;
          pageTitle = line.slice(2).trim();
          return;
        }
        const tag = line.match(/^\s*<([A-Za-z][\w]*)\s*\/>\s*$/);
        if (tag && names.has(tag[1])) {
          used.set(tag[1], (used.get(tag[1]) ?? 0) + 1);
          return;
        }
        const wrap = line.match(/^\s*<\/?([A-Za-z]\w*)>\s*$/);
        if (wrap && wrappers.has(wrap[1])) return;
        errors.push(`${where}:${n}: content with no snippet; a page holds only imports, tags, and its title`);
      });
      if (titles !== 1) errors.push(`${where}: ${titles} "# " title lines; a page has one`);
      for (const i of imports) {
        if ((used.get(i.name) ?? 0) !== 1) errors.push(`${where}:${i.line}: ${i.name} is imported once and rendered ${used.get(i.name) ?? 0} times`);
      }

      const anchors = new Map();
      for (const i of own) {
        const kind = kinds.get(i.id);
        if (kind === undefined) {
          errors.push(`${where}:${i.line}: imports ${i.id}, which has no snippet file`);
          continue;
        }
        const { anchor, fmTitle, links = [], where: snippetWhere, components = [] } = info.get(i.id) ?? {};
        for (const c of components) {
          if (given && !given.has(c)) errors.push(`${where}:${i.line}: ${i.id} renders <${c}>, which the site "${site}" does not give`);
        }
        // A link resolves on each site that shows the snippet (spec: "A link names a snippet id").
        for (const { target, n } of links) {
          const key = `${site}\0${snippetWhere}\0${n}\0${target}`;
          if (target.startsWith(`${repo.repo}/`) && kinds.get(target) === 'prose' && !siteLinks.has(target) && !reported.has(key)) {
            reported.add(key);
            errors.push(`${snippetWhere}:${n}: snippet:${target} is on no page of the site "${site}"`);
          }
        }
        if (anchor) {
          if (anchors.has(anchor)) errors.push(`${where}:${i.line}: anchor #${anchor} is on this page twice (${anchors.get(anchor)} and ${i.id})`);
          anchors.set(anchor, i.id);
        }
        // P26: the title of a first snippet with no heading is the page title.
        if (i === own[0] && fmTitle && pageTitle !== null && fmTitle !== pageTitle) {
          errors.push(`${where}:${i.line}: the first snippet title "${fmTitle}" is not the page title "${pageTitle}" (P26)`);
        }
      }
      pages.push({ site, file, imports: own });
    }
  }
  return pages;
}

/**
 * Every record of the repository: the snippets (prose, diagram, code), the
 * threat model, the OpenSpec requirements, and the authored edges on them.
 * @returns {Promise<{ prose, requirement, diagram, code, model, errors, warnings, counts, paths }>}
 *   `prose` holds the authored prose records; `requirement` the OpenSpec ones,
 *   which are prose too and go to prose.jsonl with them.
 */
export async function buildAll(repo) {
  const extracted = await extractCode(repo);
  const built = buildRecords(repo, { code: extracted.code, failed: extracted.failed });
  const model = await buildThreatModel({ repo: repo.repo });
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

async function main(argv) {
  const at = (name) => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1]; };
  const repo = openRepo(at('root') ? path.resolve(at('root')) : undefined);
  const out = at('out');
  const { prose, requirement, diagram, code, model, errors, warnings, counts, paths } = await buildAll(repo);
  for (const e of errors) console.log(`error  ${e}`);
  for (const w of warnings) console.log(`warn   ${w}`);
  const links = prose.flatMap((p) => p.refs.filter((r) => r.rel === 'relates-to')).length;
  console.log(`${counts.pages} pages, ${counts.snippets} prose snippets, ${counts.diagrams} diagrams, ${counts.code} code items, ${model.length} model records, ${requirement.length} requirements`);
  console.log(`snippet: links ${links}; inline paths ${paths.distinct - paths.missing.size} of ${paths.distinct} exist; aspect from a rule or an author ${counts.snippets - counts.aspectReview}, marked aspect_review ${counts.aspectReview}`);
  // The standalone lint is a count until stage 3 ends (task 4b.4); `standalone.mjs --list` names each fault.
  const standalone = lintSnippets(repo.snippets);
  const clean = standalone.filter((r) => r.faults.length === 0).length;
  console.log(`standalone: ${clean} of ${standalone.length} prose snippets have no fault (standalone.mjs --list names the rest)`);
  console.log(`${errors.length} errors, ${warnings.length} warnings`);
  if (out && errors.length === 0) {
    fs.mkdirSync(out, { recursive: true });
    const jsonl = (records) => records.map((r) => JSON.stringify(r)).join('\n') + (records.length ? '\n' : '');
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
