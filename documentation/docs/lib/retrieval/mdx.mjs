// Shared MDX helpers for the build generators (generate-index, generate-llms-txt,
// generate-page-markdown): one implementation for all three.
//
// Frontmatter is parsed by gray-matter (real YAML, head-anchored), so a body
// `---`, such as a mermaid config block, is never mistaken for frontmatter and
// never eats content. The strip is fence-aware, so code examples inside fenced
// blocks survive intact.

import fs from 'node:fs';
import path from 'node:path';
import matter from 'gray-matter';

/**
 * Parse frontmatter and body. Returns { data, content }. gray-matter only treats
 * a leading `---` block as frontmatter, so a `---` in the body is never eaten.
 */
export function parseFrontmatter(raw) {
  const { data, content } = matter(raw);
  return { data, content };
}

/** Title: frontmatter `title`, else the first H1 in the body, else humanised slug. */
export function pageTitle(data, content, fallback) {
  if (data && data.title != null) return String(data.title);
  const h1 = content.match(/^#\s+(.+)$/m);
  if (h1) return h1[1].trim();
  return fallback.replace(/[-_]/g, ' ');
}

/** Description from frontmatter, or ''. */
export function pageDescription(data) {
  return data && data.description != null ? String(data.description) : '';
}

/**
 * A code fence tracker, by the CommonMark rule: a fence opens with three or
 * more backticks or tildes, and closes on a line of the same character, at
 * least as long, with no info string. Call it once for each line, in order:
 * it returns 'open', 'inside', 'close', or null for a line outside any fence.
 * `tracker.info` holds the info string of the open fence (`mermaid`, `rust`),
 * and `tracker.isOpen()` tells whether a fence is still open after the last line.
 */
export function fenceTracker() {
  let open = null;
  const tracker = (line) => {
    if (open) {
      const close = line.match(/^\s*(`{3,}|~{3,})\s*$/);
      if (close && close[1][0] === open.ch && close[1].length >= open.len) {
        open = null;
        return 'close';
      }
      return 'inside';
    }
    const m = line.match(/^\s*(`{3,}|~{3,})(.*)$/);
    if (!m || (m[1][0] === '`' && m[2].includes('`'))) return null;
    open = { ch: m[1][0], len: m[1].length };
    tracker.info = m[2].trim();
    return 'open';
  };
  tracker.info = '';
  tracker.isOpen = () => open !== null;
  return tracker;
}

/**
 * An explicit heading anchor at the end of a heading: Nextra 4 writes it as
 * `## Title [#anchor]` (remark-custom-heading-id), and the older `{#anchor}` form
 * is still read. Group 1 or group 2 holds the anchor.
 */
export const HEADING_ANCHOR = /\s*(?:\{#([\w-]+)\}|\[#([^\]]+)\])\s*$/;

/**
 * Remove explicit anchors from heading lines, outside code fences. Nextra renders
 * the anchor as the heading id and not as text, so a plain-text copy of the page
 * must not show it either.
 */
export function stripHeadingIds(body) {
  const fence = fenceTracker();
  return body.split('\n').map((line) => {
    if (fence(line) || !/^#{1,6}\s/.test(line)) return line;
    return line.replace(HEADING_ANCHOR, '');
  }).join('\n');
}

/**
 * Inline MDX partials before anything else looks at the page.
 *
 * A page's source file is not the page's content. `performance-and-testing.mdx`
 * imports `node-perf-mixnet.mdx` and renders it as `<NodePerfMixnet />`, so every
 * sentence in that partial is on the published page and none of it is in the file
 * the chunker reads. A partial that a page imports and renders is invisible to
 * retrieval unless it is inlined, so the mixnet performance figures a developer
 * needs to judge whether a workload fits never reach the index.
 *
 * Partials are plain Markdown, so this needs no rendering: resolve the import,
 * splice the text in where the tag appears, and let the normal chunking treat it
 * as part of the page, which is what a reader sees.
 *
 * Specifiers are resolved the way the app resolves them: bare paths against the
 * docs root (tsconfig `baseUrl`), and `./` or `../` against the importing file.
 * `aliases` maps a prefix such as `@snippets/nym/` to a directory, as the site's
 * bundler alias does. Nesting works and cycles are broken, so a partial may
 * include another.
 *
 * Snippet pages also hold citations and `snippet:` links. With `citation`, each
 * `<Snippet id="..." />` becomes the text that `citation(id, attrs)` returns
 * (a `mermaid` fence for a diagram, for example). With `link`, each
 * `](snippet:<id>)` becomes `](<link(id)>)`. Both skip code fences.
 *
 * @param {string} content   page body, frontmatter already removed
 * @param {{ filePath: string, root: string, lineOffset?: number, aliases?: Record<string, string>,
 *   citation?: (id: string, attrs: string) => string, link?: (id: string) => string,
 *   seen?: Set<string> }} opts  `lineOffset`: the lines of frontmatter above `content`,
 *   so an error names the line in the file
 */
export function inlineMdxPartials(content, opts) {
  const { filePath, root, lineOffset = 0, aliases = {}, citation, link, seen = new Set() } = opts;
  // Citations and links first, in this file's own text, so an error names its line.
  const at = (line) => (fn) => {
    try {
      return fn();
    } catch (error) {
      throw new Error(`${filePath}:${line + lineOffset}: ${error.message}`, { cause: error });
    }
  };
  let out = content;
  if (citation) {
    out = outsideFences(out, (text, line) => text.replace(CITATION, (whole, id, attrs, offset) =>
      at(line + lineOf(text, offset))(() => citation(id, attrs.trim()))));
  }
  if (link) {
    out = outsideFences(out, (text, line) => text.replace(/\]\(snippet:([^)\s]+)\)/g, (whole, id, offset) =>
      `](${at(line + lineOf(text, offset))(() => link(id))})`));
  }

  for (const { name, spec, line: bodyLine } of mdxImports(content)) {
    const line = bodyLine + lineOffset;
    const resolved = resolveImport(spec, { filePath, root, aliases });

    // Nextra fails the site build on this import, so a text output must not
    // pass it silently with the partial's text missing.
    if (!fs.existsSync(resolved)) {
      throw new Error(`${filePath}:${line}: cannot resolve the MDX import '${spec}'`);
    }
    // A cycle would otherwise recurse until the stack gives out.
    if (seen.has(resolved)) continue;

    const raw = fs.readFileSync(resolved, 'utf-8');
    const { content: partial } = parseFrontmatter(raw);
    let nested;
    try {
      nested = inlineMdxPartials(partial, { ...opts, filePath: resolved, lineOffset: frontmatterLines(raw, partial), seen: new Set([...seen, resolved]) });
    } catch (error) {
      throw new Error(`${filePath}:${line}: in '${spec}': ${error.message}`, { cause: error });
    }

    // The tag can appear anywhere, including nested inside another element as
    // `<MyTab><NodePerfMixnet /></MyTab>`, so this is not a whole-line match.
    // Blank lines around the splice keep the partial's own headings parseable.
    out = out.replace(
      new RegExp(`<${name}\\s*/>`, 'g'),
      () => `\n\n${nested}\n\n`,
    );
  }
  return out;
}

/** The lines above `content` in `raw`: the frontmatter that parseFrontmatter removed. */
export const frontmatterLines = (raw, content) => raw.split('\n').length - content.split('\n').length;

/** `<Snippet id="..." view="..." />`, the citation form of a snippet body. */
export const CITATION = /<Snippet\s+id="([^"]+)"([^>]*?)\s*\/>/g;

const lineOf = (text, offset) => text.slice(0, offset).split('\n').length - 1;

/** Apply `fn(text, firstLine)` to each run of lines outside a code fence. Lines are 1-indexed. */
function outsideFences(text, fn) {
  const out = [];
  let run = [];
  let runStart = 1;
  const fence = fenceTracker();
  const flush = () => { if (run.length) out.push(fn(run.join('\n'), runStart)); run = []; };
  text.split('\n').forEach((line, i) => {
    if (fence(line)) {
      flush();
      out.push(line);
      return;
    }
    if (run.length === 0) runStart = i + 1;
    run.push(line);
  });
  flush();
  return out.join('\n');
}

/**
 * Resolve an import specifier the way the app does: an alias prefix to its
 * directory, `./` and `../` against the importing file, a bare path against the
 * docs root (tsconfig `baseUrl`).
 */
export function resolveImport(spec, { filePath, root, aliases = {} }) {
  const prefix = Object.keys(aliases).find((p) => spec.startsWith(p));
  if (prefix) return path.join(aliases[prefix], spec.slice(prefix.length));
  return spec.startsWith('.') ? path.resolve(path.dirname(filePath), spec) : path.resolve(root, spec);
}

/**
 * The `.mdx` default imports of a body, outside code fences, with the 1-indexed
 * line of each. An import line inside a fence is example code, not an import.
 */
function mdxImports(content) {
  const found = [];
  const fence = fenceTracker();
  content.split('\n').forEach((text, i) => {
    if (fence(text)) return;
    const m = text.match(/^import\s+(\w+)\s+from\s+['"]([^'"]+\.mdx)['"]\s*;?\s*$/);
    if (m) found.push({ name: m[1], spec: m[2], line: i + 1 });
  });
  return found;
}

/**
 * Strip `import` statements, `export` statements and whole-line JSX tags from an
 * MDX body, leaving fenced code blocks untouched. Expects frontmatter already
 * removed (parseFrontmatter).
 *
 * `expand` is optional: `(tagName, attrs, ctx) => string | null`. When it returns
 * text, that text replaces the tag. This is how a component that renders content
 * from typed data gets into the index at all; see projections.mjs. Returning null
 * drops the tag, which is right for anything purely visual.
 *
 * `ctx` is one object per call, so an expander can carry page-level state such as
 * which scenario the page declared.
 */
export function stripMdx(content, { expand, values } = {}) {
  const out = [];
  let fence = null;
  let jsDepth = 0;
  let inComment = false;
  const ctx = { ...scenariosOf(content) };

  for (const line of content.split('\n')) {
    const fenceMatch = line.match(/^(\s*)(`{3,}|~{3,})/);
    if (fenceMatch) {
      const marker = fenceMatch[2][0];
      if (fence === null) fence = marker;
      else if (marker === fence) fence = null;
      out.push(line);
      continue;
    }
    if (fence !== null) {
      out.push(line); // inside a code fence: keep verbatim
      continue;
    }
    // An MDX comment, `{/* ... */}`. The single-line form is caught by the bare
    // expression rule below; a multi-line one must be skipped to its closing
    // `*/}`, or editor-facing notes reach readers as page content.
    if (inComment) {
      if (line.includes('*/}')) inComment = false;
      continue;
    }
    if (/^\s*\{\s*\/\*/.test(line) && !line.includes('*/}')) {
      inComment = true;
      continue;
    }

    // A multi-line statement opened on an earlier line: keep skipping until its
    // brackets balance. Without this only the first line of a statement is
    // dropped and the rest reaches the index as prose.
    if (jsDepth > 0) {
      jsDepth += bracketDelta(line);
      continue;
    }
    if (/^import\s+.*$/.test(line)) continue;
    // Page wiring, not prose. `export const scenario = requireGenericScenario('mixnet')`
    // and the `dynamic(() => import(...))` blocks beside it must not reach the
    // index; they would be served to readers as if they were documentation.
    if (JS_STATEMENT.test(line)) {
      jsDepth = bracketDelta(line);
      continue;
    }
    // `{RUST_MSRV}` and friends are values the page interpolates at render time.
    // Substitute the ones we know: emitting the source text states a fact the
    // reader cannot read, and a version requirement is exactly the kind of thing
    // someone comes to the docs for.
    let text = values ? substitute(line, values) : line;
    // A bare expression line we could not resolve renders as nothing on the page,
    // so it should contribute nothing here either.
    if (/^\s*\{[^{}]*\}\s*$/.test(text)) continue;

    const jsx = text.match(/^\s*<([A-Za-z][\w.-]*)((?:\s[^>]*)?)\s*\/?>\s*$/);
    if (jsx) {
      const projected = expand ? expand(jsx[1], jsx[2] ?? '', ctx) : null;
      if (projected) out.push(projected);
      continue;
    }
    if (/^\s*<\/[\w.-]+\s*>\s*$/.test(text)) continue; // closing tag line
    out.push(text);
  }
  return out.join('\n').replace(/\n{3,}/g, '\n\n').trim();
}

// Top-level JS in an MDX page. Anchored at column 0 and requiring the shape of a
// declaration, so prose beginning with a word like "export" is not mistaken for
// code.
const JS_STATEMENT = /^(?:export\s|(?:const|let|var)\s+[\w$]+\s*=|(?:async\s+)?function\s|class\s)/;

/** Net bracket depth a line opens, used to span multi-line statements. */
function bracketDelta(line) {
  let d = 0;
  for (const ch of line) {
    if (ch === '{' || ch === '(' || ch === '[') d++;
    else if (ch === '}' || ch === ')' || ch === ']') d--;
  }
  return d;
}

/** Replace `{IDENT}` with its value where the identifier is one we loaded. */
function substitute(line, values) {
  return line.replace(/\{\s*([A-Za-z_$][\w$]*)\s*\}/g, (whole, name) =>
    Object.prototype.hasOwnProperty.call(values, name) ? values[name] : whole,
  );
}

/**
 * The scenarios a page declares: `export const mixnet = requireWalletScenario('mixnet-ipr')`.
 * A component names one through its props (`<NetworkDiagram item={mixnet} />`).
 * Returns `scenarios` (const name -> { source, id }), `firstScenario`, and
 * `scenarioId` (the first generic one, for a page that names it once).
 */
function scenariosOf(content) {
  const scenarios = new Map();
  for (const [, name, source, id] of content.matchAll(/export\s+const\s+(\w+)\s*=\s*require(Generic|Browsing|Messaging|Wallet)Scenario\(\s*["']([^"']+)["']\s*\)/g)) {
    scenarios.set(name, { source: source.toLowerCase(), id });
  }
  const firstScenario = scenarios.values().next().value ?? null;
  const generic = [...scenarios.values()].find((s) => s.source === 'generic');
  return { scenarios, firstScenario, scenarioId: generic?.id ?? null };
}
