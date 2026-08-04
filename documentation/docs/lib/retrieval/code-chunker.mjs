// Code chunker: splits source files into retrieval-sized units at top-level item
// boundaries (fn / struct / impl / class / export ...), with a hard size cap.
// Language-aware only enough to find boundaries and a symbol name; it does not
// parse an AST. Good enough for semantic code search with a code-tuned embedder.
//
// Emits chunks shaped like the docs Chunk (title/heading/url/text/source) so the
// same retrieval.search() works over the code index unchanged.

const MAX_CHARS = 2400;
// Lower than the docs floor: small code items (a one-line fn, a tiny struct) are
// legitimate and worth indexing; we only want to drop empty/near-empty blocks.
const MIN_CHARS = 16;
const GITHUB = 'https://github.com/nymtech/nym/blob/develop';

// Top-level item starts, anchored at column 0 (not indented = top-level).
const RUST_ITEM =
  /^(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(?:const\s+)?(?:fn|struct|enum|trait|impl|mod|type|static|macro_rules!)\b(.*)/;
const TS_ITEM =
  /^(?:export\s+)?(?:default\s+)?(?:async\s+)?(?:function\*?|class|interface|type|enum|const|abstract\s+class)\s+([A-Za-z0-9_$]+)/;

export function langOf(file) {
  if (file.endsWith('.rs')) return 'rust';
  if (/\.(tsx?|jsx?|mjs)$/.test(file)) return 'typescript';
  return null;
}

/** Best-effort symbol name from a boundary line, for the chunk heading. */
function symbolOf(line, lang) {
  if (lang === 'typescript') {
    const m = line.match(TS_ITEM);
    return m ? m[1] : '';
  }
  // rust: grab the identifier after the keyword
  const m = line.match(/\b(?:fn|struct|enum|trait|mod|type|static|const)\s+([A-Za-z0-9_]+)/) || line.match(/\bimpl\b[^{]*?\b([A-Za-z0-9_]+)/);
  return m ? m[1] : '';
}

function isBoundary(line, lang) {
  return lang === 'rust' ? RUST_ITEM.test(line) : TS_ITEM.test(line);
}

/**
 * Split source into { text, symbol, startLine } blocks at top-level items,
 * capping block size. Preamble before the first item becomes its own block.
 */
export function chunkCode(content, lang) {
  const lines = content.split('\n');
  const bounds = [];
  lines.forEach((l, i) => {
    if (isBoundary(l, lang)) bounds.push(i);
  });
  if (bounds.length === 0 || bounds[0] !== 0) bounds.unshift(0);

  const out = [];
  for (let b = 0; b < bounds.length; b++) {
    const start = bounds[b];
    const end = b + 1 < bounds.length ? bounds[b + 1] : lines.length;
    const symbol = symbolOf(lines[start] ?? '', lang);
    let block = lines.slice(start, end).join('\n');
    let lineOffset = 0;
    // Hard-split oversized blocks so no chunk blows the embedder's budget. Cut at
    // the last newline within the cap; if a single line is longer than the cap
    // (minified/generated code), cut by chars so we always make progress.
    while (block.trim().length > MAX_CHARS) {
      let cut = block.lastIndexOf('\n', MAX_CHARS);
      if (cut <= 0) cut = MAX_CHARS;
      const piece = block.slice(0, cut);
      if (piece.trim().length >= MIN_CHARS) out.push({ text: piece.trim(), symbol, startLine: start + 1 + lineOffset });
      lineOffset += (piece.match(/\n/g) || []).length;
      block = block.slice(cut).replace(/^\n/, '');
    }
    if (block.trim().length >= MIN_CHARS) out.push({ text: block.trim(), symbol, startLine: start + 1 + lineOffset });
  }
  return out;
}

/**
 * Chunk one file into retrieval records tagged source: 'nym-code'.
 * @param {string} repoPath repo-relative path, e.g. common/nymsphinx/src/lib.rs
 */
export function chunkCodeFile(content, repoPath) {
  const lang = langOf(repoPath);
  if (!lang) return [];
  return chunkCode(content, lang).map((c, i) => ({
    id: `${repoPath}#${i}`,
    source: 'nym-code',
    title: repoPath,
    heading: c.symbol || `${repoPath.split('/').pop()}`,
    url: `${GITHUB}/${repoPath}#L${c.startLine}`,
    text: c.text,
    lang,
  }));
}

export { MAX_CHARS, MIN_CHARS };
