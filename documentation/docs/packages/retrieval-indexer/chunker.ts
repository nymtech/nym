// Heading-scoped chunker for the docs retrieval index.
//
// Source-agnostic: chunkPages() takes normalised PageRecord objects, so the
// docs adapter and a future Confluence adapter feed the same pipeline.

import GithubSlugger, { slug } from 'github-slugger';
import { HEADING_ANCHOR } from './mdx.ts';
import type { Chunk, PageRecord } from './types.ts';

export interface Section {
  heading: string;
  /** an explicit `{#id}` on the heading, or null */
  anchor: string | null;
  level: number;
  text: string;
}

// Chunking targets. A chunk is one heading section; sections larger than
// MAX_CHARS are split, first on paragraph boundaries and then, if a single
// paragraph or code block is still too big, on a hard character cap so no
// chunk can blow the retrieval budget. Tuned for ~400-600 token chunks.
export const MAX_CHARS = 2400;
export const MIN_CHARS = 120;

// Heading slugs whose sections are auto-generated, prose-free noise (CLI shell
// completion specs). Skipped entirely: near-zero retrieval value.
export const DEFAULT_SKIP_SLUGS = new Set(['generate-fig-spec']);

// Heading slug for deep-link anchors. Delegates to github-slugger, the same
// slugger Nextra renders its on-page anchors with (via rehype-slug), so our
// retrieval deep links match the real anchors exactly rather than approximating
// them. This is the stateless form (no de-duplication); chunkPages() uses a
// per-page GithubSlugger instance for the -1/-2 suffixes.
export function slugify(heading: string): string {
  return slug(heading);
}

/** chars/4 is the standard rough token estimate; a real tokeniser comes later. */
export function estimateTokens(text: string): number {
  return Math.ceil(text.length / 4);
}

/**
 * Split a Markdown body into heading-scoped sections. Fence-aware: `##` lines
 * inside fenced code blocks (``` or ~~~) are treated as code, not headings, so
 * a `## foo` comment in a bash example never creates a false section boundary.
 * Content before the first heading attaches to the page title.
 */
export function splitByHeadings(body: string, pageTitle: string): Section[] {
  const lines = body.split('\n');
  const sections: Section[] = [];
  let current = { heading: pageTitle, anchor: null as string | null, level: 1, lines: [] as string[] };
  let fence: string | null = null; // the fence marker that opened the current code block

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
        sections.push({ heading: current.heading, anchor: current.anchor ?? null, level: current.level, text: current.lines.join('\n').trim() });
      }
      const explicit = headingMatch[2].match(HEADING_ANCHOR);
      current = {
        heading: explicit ? headingMatch[2].slice(0, explicit.index).trim() : headingMatch[2],
        anchor: explicit ? (explicit[1] ?? explicit[2]) : null,
        level: headingMatch[1].length,
        lines: [],
      };
    } else {
      current.lines.push(line);
    }
  }
  if (current.lines.join('\n').trim().length > 0) {
    sections.push({ heading: current.heading, anchor: current.anchor ?? null, level: current.level, text: current.lines.join('\n').trim() });
  }
  return sections;
}

/**
 * Split oversized text so no part exceeds maxChars. Greedily packs paragraphs;
 * any single paragraph still over the cap (e.g. one long code block) is
 * hard-split on line then raw character boundaries. Guarantees every part is
 * <= maxChars.
 */
export function packSection(text: string, maxChars = MAX_CHARS): string[] {
  if (text.length <= maxChars) return [text];

  const units: string[] = [];
  for (const para of text.split(/\n\s*\n/)) {
    if (para.length <= maxChars) {
      units.push(para);
    } else {
      units.push(...hardSplit(para, maxChars));
    }
  }

  // Greedily pack units back up to the cap.
  const out: string[] = [];
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
function hardSplit(block: string, maxChars: number): string[] {
  const out: string[] = [];
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

/** Turn normalised pages into retrievable chunks. */
export function chunkPages(
  pages: PageRecord[],
  { siteUrl, maxChars = MAX_CHARS, skipSlugs = DEFAULT_SKIP_SLUGS }: { siteUrl: string; maxChars?: number; skipSlugs?: Set<string> },
): Chunk[] {
  const chunks: Chunk[] = [];

  for (const page of pages) {
    const pagePath = page.url.replace(`${siteUrl}/`, '');
    const sections = splitByHeadings(page.body, page.title);

    // Per-page slugger so repeated headings ("Options", "Usage") get -1/-2
    // suffixes exactly as Nextra does. github-slugger keeps the per-page dedup
    // state; a fresh instance per page resets it, matching Nextra's per-page anchors.
    const slugger = new GithubSlugger();

    for (const section of sections) {
      // The intro is the text before the first heading (level 1 from
      // splitByHeadings), not any section whose heading reads like the title:
      // a `## Centralised VPN` on a page titled "Centralised VPN" is a section.
      const isPageIntro = section.level === 1;
      let anchor = '';
      if (!isPageIntro) {
        // Skip check uses the un-deduplicated base; skipped sections must not
        // advance the slugger (so they don't shift later suffixes).
        if (skipSlugs.has(slugify(section.heading))) continue;
        // An explicit `{#id}` wins. Projected content (projections.mjs) carries
        // the id the component actually renders, so a citation lands on the
        // section a reader can see rather than on a slug that exists only here.
        anchor = section.anchor ? `#${section.anchor}` : `#${slugger.slug(section.heading)}`;
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
