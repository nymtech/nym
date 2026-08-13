// Citation handling for the chat widget: rewriting inline `[n]` markers into
// links, and working out which sections an answer actually used.
//
// Pure string work with no React or Next import, so it lives here under lib/
// where vitest runs it. The widget is a .tsx that pulls in react-markdown at
// module load, and this logic decides what the reader sees, so it should not be
// reachable only through a rendered component.

import type { Citation } from './context';

/**
 * Splits an answer into alternating prose and code segments, prose at the even
 * indices. Shared so that linkifyCitations and citedNumbers agree on where a
 * `[n]` is a citation and where it is part of an example.
 *
 * Limits, all cosmetic. Only ``` fences and single-backtick spans are recognised,
 * so `[1]` inside a 4-space indented block, a ~~~ fence, or a ``double-backtick``
 * span reads as prose. A fence mid-stream has no closing ```, so its contents
 * read as prose until the closing chunk arrives, or permanently if the answer is
 * truncated first. A rehype plugin skipping nodes under code/pre would avoid all
 * of this by working on the parsed tree.
 */
const CODE_SPANS = /(```[\s\S]*?```|`[^`\n]*`)/g;

/** A bare `[n]`. The lookahead leaves real markdown links (`[1](url)`) alone. */
const CITE_MARKER = /\[(\d+)\](?!\()/g;

/**
 * Chunk URLs are absolute (`https://nym.com/docs/...`), baked in at index time.
 * Strip the origin so a citation keeps the reader inside whatever deployment
 * they are on: localhost and Vercel previews would otherwise jump to production.
 *
 * The `/docs` basePath comes off too, because these are handed to next/link,
 * which re-adds it. Leaving it on yields `/docs/docs/...`.
 */
export function docsHref(url: string): string {
  const path = url.replace(/^https?:\/\/[^/]+/, '');
  // The lookahead keeps a docs-root chunk (`/docs`, `/docs#anchor`) from being
  // mangled, and stops `/docsomething` matching.
  return path.replace(/^\/docs(?=[/#?]|$)/, '') || '/';
}

/**
 * Turn the model's inline `[n]` markers into markdown links to the cited
 * section, so react-markdown renders them as anchors.
 *
 * `citations` must be the full list the route sent. Lookup is positional, so a
 * filtered list would resolve `[7]` against the wrong entry, or drop it.
 */
export function linkifyCitations(text: string, citations: Citation[]): string {
  if (citations.length === 0) return text;
  return text
    .split(CODE_SPANS)
    .map((segment, i) =>
      i % 2 === 1
        ? segment
        : segment.replace(CITE_MARKER, (whole, n: string) => {
            const c = citations[Number(n) - 1];
            return c ? `[${n}](${docsHref(c.url)})` : whole;
          }),
    )
    .join('');
}

/**
 * The citation numbers the answer actually uses.
 *
 * Retrieval hands the model more sections than it needs, and cosine similarity
 * cannot tell a relevant section from a merely nearby one: an off-topic question
 * still returns a full set of hits, above any floor that leaves real questions
 * their sources. Listing all of them puts a row of confident-looking sources
 * under an answer that declines to use them.
 *
 * So the model decides. It reads the sections and cites what it used; anything
 * it ignored is not shown. A refusal cites nothing and gets no source list.
 */
export function citedNumbers(text: string): Set<number> {
  const used = new Set<number>();
  text
    .split(CODE_SPANS)
    .filter((_, i) => i % 2 === 0)
    .forEach((segment) => {
      for (const m of segment.matchAll(CITE_MARKER)) used.add(Number(m[1]));
    });
  return used;
}

/**
 * The citations an answer cited, in the order the route numbered them.
 *
 * Numbering is preserved rather than compacted: the answer's inline markers refer
 * to these numbers, so renumbering a filtered list to 1..n would point every
 * marker at the wrong source.
 */
export function citedSources(text: string, citations: Citation[]): Citation[] {
  const used = citedNumbers(text);
  return citations.filter((c) => used.has(c.n));
}
