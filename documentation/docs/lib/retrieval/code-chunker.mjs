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

/**
 * The type an `impl` block is about. The generic list has to be skipped by
 * counting angle brackets rather than matched, because it nests
 * (`impl<T: Into<String>> ...`). What remains is either `Type` or
 * `Trait for Type`, and the concrete type is the useful name in both.
 */
function implTarget(line) {
  const at = line.match(/\bimpl\b/);
  if (!at) return '';
  let j = at.index + 4;
  while (j < line.length && /\s/.test(line[j])) j++;
  if (line[j] === '<') {
    let depth = 0;
    for (; j < line.length; j++) {
      if (line[j] === '<') depth++;
      else if (line[j] === '>' && --depth === 0) {
        j++;
        break;
      }
    }
  }
  const rest = line.slice(j);
  const forImpl = rest.match(/\bfor\s+(?:&\s*)?(?:'[A-Za-z0-9_]+\s+)?(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)/);
  if (forImpl) return forImpl[1];
  const head = rest.match(/([A-Za-z_][A-Za-z0-9_]*)/);
  return head ? head[1] : '';
}

/** Best-effort symbol name from a boundary line, for the chunk heading. */
export function symbolOf(line, lang) {
  if (lang === 'typescript') {
    const m = line.match(TS_ITEM);
    return m ? m[1] : '';
  }
  // rust: the identifier after the item keyword. The negative lookahead makes
  // `const fn foo` bind to `fn` rather than capturing `fn` as the name of a
  // const; `mut` is consumed so `static mut BUF` still yields `BUF`.
  const m = line.match(
    /\b(?:fn|struct|enum|trait|mod|type|static|const)\s+(?:mut\s+)?(?!fn\b|unsafe\b|extern\b|async\b)([A-Za-z_][A-Za-z0-9_]*)/,
  );
  return m ? m[1] : implTarget(line);
}

function isBoundary(line, lang) {
  return lang === 'rust' ? RUST_ITEM.test(line) : TS_ITEM.test(line);
}

// Lines that document or annotate the item directly below them: Rust `///`/`//!`
// doc comments and `#[...]`/`#![...]` attributes; TS `//`, `/*`, `/**` and their
// ` *` continuation lines.
const DOC_ATTR = {
  rust: /^\s*(\/\/\/|\/\/!|#!?\[)/,
  typescript: /^\s*(\/\/|\/\*|\*)/,
};

// Walk backward from an item over its contiguous doc-comment / attribute lines
// (stopping at the previous item, and at the first non-doc line) so they group
// with the item they describe instead of falling into the previous chunk.
function precedingDocStart(lines, itemLine, lang, floor) {
  const re = DOC_ATTR[lang];
  if (!re) return itemLine;
  let s = itemLine;
  while (s - 1 > floor && re.test(lines[s - 1] ?? '')) s--;
  return s;
}

/**
 * Split source into { text, symbol, startLine } blocks at top-level items,
 * capping block size. Preamble before the first item becomes its own block.
 */
export function chunkCode(content, lang) {
  const lines = content.split('\n');
  const items = [];
  lines.forEach((l, i) => {
    if (isBoundary(l, lang)) items.push(i);
  });
  if (items.length === 0 || items[0] !== 0) items.unshift(0);

  // Each block starts at the item's preceding doc-comment/attribute lines, so
  // those embed with the item they describe rather than the previous one. The
  // symbol name still comes from the item line itself.
  const starts = items.map((item, b) =>
    b === 0 ? item : precedingDocStart(lines, item, lang, items[b - 1]),
  );

  const out = [];
  for (let b = 0; b < items.length; b++) {
    const start = starts[b];
    const end = b + 1 < items.length ? starts[b + 1] : lines.length;
    const symbol = symbolOf(lines[items[b]] ?? '', lang);
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
