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
 * Strip `import` statements and whole-line JSX tags from an MDX body, leaving
 * fenced code blocks untouched. Expects frontmatter already removed (parseFrontmatter).
 */
export function stripMdx(content) {
  const out = [];
  let fence = null;

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
    if (/^import\s+.*$/.test(line)) continue;
    if (/^\s*<\w[\w.-]*(?:\s[^>]*)?\s*\/>\s*$/.test(line)) continue; // self-closing JSX
    if (/^\s*<\/?\w[\w.-]*(?:\s[^>]*)?\s*>\s*$/.test(line)) continue; // JSX tag line
    out.push(line);
  }
  return out.join('\n').replace(/\n{3,}/g, '\n\n').trim();
}
