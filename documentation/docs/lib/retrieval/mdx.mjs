// Shared MDX helpers for the build generators (generate-index, generate-llms-txt,
// generate-page-markdown). Replaces three hand-rolled, subtly divergent copies:
// two parsed frontmatter with a non-head-anchored regex (a body `---`, e.g. a
// mermaid config block, could be mistaken for frontmatter and eat content), and
// one stripped `import`/JSX lines without fence-awareness (corrupting code
// examples inside fenced blocks in llms-full.txt).
//
// Frontmatter is parsed by gray-matter (real YAML, head-anchored); the strip is
// the fence-aware line filter the good copies already used.

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
 * Strip `import` statements, `export` statements and whole-line JSX tags from an
 * MDX body, leaving fenced code blocks untouched. Expects frontmatter already
 * removed (parseFrontmatter).
 *
 * `expand` is optional: `(tagName, attrs, ctx) => string | null`. When it returns
 * text, that text replaces the tag. This is how a component that renders content
 * from typed data gets into the index at all; see projections.mjs. Returning null
 * drops the tag as before, which is right for anything purely visual.
 *
 * `ctx` is one object per call, so an expander can carry page-level state such as
 * which scenario the page declared.
 */
export function stripMdx(content, { expand, values } = {}) {
  const out = [];
  let fence = null;
  let jsDepth = 0;
  const ctx = { scenarioId: scenarioIdOf(content) };

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
    // A multi-line statement opened on an earlier line: keep skipping until its
    // brackets balance. Without this only the first line of a statement is
    // dropped and the rest reaches the index as prose.
    if (jsDepth > 0) {
      jsDepth += bracketDelta(line);
      continue;
    }
    if (/^import\s+.*$/.test(line)) continue;
    // Page wiring, not prose. `export const scenario = requireGenericScenario('mixnet')`
    // and the `dynamic(() => import(...))` blocks beside it were reaching the
    // index verbatim and being served to readers as if they were documentation.
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
 * The scenario a page declares, from `requireGenericScenario('id')`. Threat-model
 * configuration pages name their scenario once at the top and then render it
 * through several components.
 */
function scenarioIdOf(content) {
  const m = content.match(/requireGenericScenario\(\s*["']([^"']+)["']\s*\)/);
  return m ? m[1] : null;
}
