// Heading-scoped chunker for the docs retrieval index.
//
// Plain ESM JS (not TS) so the build generator, which runs under bare `node`,
// can import it directly; the co-located chunker.test.ts exercises it via
// vitest. Types live in ./types.ts and are referenced through JSDoc.
//
// Source-agnostic: chunkPages() takes normalised PageRecord objects, so the
// docs adapter and a future Confluence adapter feed the same pipeline.
//
/** @typedef {import('./types').PageRecord} PageRecord */
/** @typedef {import('./types').Chunk} Chunk */

import GithubSlugger, { slug } from 'github-slugger';

// Chunking targets. A chunk is one heading section; sections larger than
// MAX_CHARS are split, first on paragraph boundaries and then, if a single
// paragraph or code block is still too big, on a hard character cap so no
// chunk can blow the retrieval budget. Tuned for ~400-600 token chunks.
export const MAX_CHARS = 2400;
export const MIN_CHARS = 120;

// Heading slugs whose sections are auto-generated, prose-free noise (CLI shell
// completion specs). Skipped entirely: near-zero retrieval value, and they are
// the pages that produced the 6k-token chunks in the Phase 0 spike.
export const DEFAULT_SKIP_SLUGS = new Set(['generate-fig-spec']);

// Heading slug for deep-link anchors. Delegates to github-slugger, the same
// slugger Nextra renders its on-page anchors with (via rehype-slug), so our
// retrieval deep links match the real anchors exactly rather than approximating
// them. This is the stateless form (no de-duplication); chunkPages() uses a
// per-page GithubSlugger instance for the -1/-2 suffixes.
export function slugify(heading) {
  return slug(heading);
}

/** chars/4 is the standard rough token estimate; a real tokeniser comes later. */
export function estimateTokens(text) {
  return Math.ceil(text.length / 4);
}

/**
 * Split a Markdown body into heading-scoped sections. Fence-aware: `##` lines
 * inside fenced code blocks (``` or ~~~) are treated as code, not headings, so
 * a `## foo` comment in a bash example never creates a false section boundary.
 * Content before the first heading attaches to the page title.
 *
 * @param {string} body
 * @param {string} pageTitle
 * @returns {{ heading: string, level: number, text: string }[]}
 */
export function splitByHeadings(body, pageTitle) {
  const lines = body.split('\n');
  const sections = [];
  let current = { heading: pageTitle, level: 1, lines: [] };
  let fence = null; // the fence marker that opened the current code block

  for (const line of lines) {
    const fenceMatch = line.match(/^(\s*)(`{3,}|~{3,})/);
    if (fenceMatch) {
      const marker = fenceMatch[2][0]; // ` or ~
      if (fence === null) fence = marker;
      else if (marker === fence) fence = null; // closing fence
      current.lines.push(line);
      continue;
    }

    const headingMatch = fence === null && line.match(/^(#{2,4})\s+(.+?)\s*#*\s*$/);
    if (headingMatch) {
      if (current.lines.join('\n').trim().length > 0) {
        sections.push({ heading: current.heading, level: current.level, text: current.lines.join('\n').trim() });
      }
      current = { heading: headingMatch[2], level: headingMatch[1].length, lines: [] };
    } else {
      current.lines.push(line);
    }
  }
  if (current.lines.join('\n').trim().length > 0) {
    sections.push({ heading: current.heading, level: current.level, text: current.lines.join('\n').trim() });
  }
  return sections;
}

/**
 * Split oversized text so no part exceeds maxChars. Greedily packs paragraphs;
 * any single paragraph still over the cap (e.g. one long code block) is
 * hard-split on line then raw character boundaries. Guarantees every part is
 * <= maxChars, which the paragraph-only splitter in the spike did not.
 *
 * @param {string} text
 * @param {number} maxChars
 * @returns {string[]}
 */
export function packSection(text, maxChars = MAX_CHARS) {
  if (text.length <= maxChars) return [text];

  const units = [];
  for (const para of text.split(/\n\s*\n/)) {
    if (para.length <= maxChars) {
      units.push(para);
    } else {
      units.push(...hardSplit(para, maxChars));
    }
  }

  // Greedily pack units back up to the cap.
  const out = [];
  let buf = '';
  for (const unit of units) {
    if (buf && buf.length + unit.length + 2 > maxChars) {
      out.push(buf.trim());
      buf = '';
    }
    buf += (buf ? '\n\n' : '') + unit;
  }
  if (buf.trim()) out.push(buf.trim());
  return out;
}

/** Hard-split a single oversized block on line, then raw-char, boundaries. */
function hardSplit(block, maxChars) {
  const out = [];
  let buf = '';
  for (const line of block.split('\n')) {
    if (line.length > maxChars) {
      if (buf) { out.push(buf); buf = ''; }
      for (let i = 0; i < line.length; i += maxChars) out.push(line.slice(i, i + maxChars));
      continue;
    }
    if (buf && buf.length + line.length + 1 > maxChars) { out.push(buf); buf = ''; }
    buf += (buf ? '\n' : '') + line;
  }
  if (buf) out.push(buf);
  return out;
}

/**
 * Turn normalised pages into retrievable chunks.
 *
 * @param {PageRecord[]} pages
 * @param {{ siteUrl: string, maxChars?: number, skipSlugs?: Set<string> }} opts
 * @returns {Chunk[]}
 */
export function chunkPages(pages, { siteUrl, maxChars = MAX_CHARS, skipSlugs = DEFAULT_SKIP_SLUGS }) {
  const chunks = [];

  for (const page of pages) {
    const pagePath = page.url.replace(`${siteUrl}/`, '');
    const sections = splitByHeadings(page.body, page.title);

    // Per-page slugger so repeated headings ("Options", "Usage") get -1/-2
    // suffixes exactly as Nextra does. github-slugger keeps the per-page dedup
    // state; a fresh instance per page resets it, matching Nextra's per-page anchors.
    const slugger = new GithubSlugger();

    for (const section of sections) {
      const isPageIntro = section.heading === page.title;
      let anchor = '';
      if (!isPageIntro) {
        // Skip check uses the un-deduplicated base; skipped sections must not
        // advance the slugger (so they don't shift later suffixes).
        if (skipSlugs.has(slugify(section.heading))) continue;
        anchor = `#${slugger.slug(section.heading)}`;
      }

      const parts = packSection(section.text, maxChars);
      parts.forEach((part, i) => {
        chunks.push({
          id: `${pagePath}${anchor}${parts.length > 1 ? `~${i}` : ''}`,
          source: page.source,
          title: page.title,
          heading: section.heading,
          url: `${page.url}${anchor}`,
          text: part,
          tokensEst: estimateTokens(part),
        });
      });
    }
  }
  return chunks;
}
