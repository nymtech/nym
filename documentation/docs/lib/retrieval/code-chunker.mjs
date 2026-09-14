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

// Kotlin and Swift top-level items, same column-0 = top-level convention as Rust
// and TypeScript. Line-regex boundary finding, not an AST (design decision 10):
// a language is a boundary regex, a doc/annotation rule (DOC_ATTR), and a symbol
// regex. Leading annotations on the same line are tolerated; annotations on their
// own preceding lines group via DOC_ATTR.
const KOTLIN_ITEM =
  /^(?:@\w+(?:\([^)]*\))?\s+)*(?:public\s+|private\s+|internal\s+|protected\s+|open\s+|final\s+|abstract\s+|sealed\s+|data\s+|inner\s+|enum\s+|annotation\s+|value\s+|external\s+|expect\s+|actual\s+|inline\s+|infix\s+|operator\s+|suspend\s+|tailrec\s+|override\s+|const\s+|lateinit\s+)*(?:fun|class|interface|object|val|var|typealias)\b/;
const SWIFT_ITEM =
  /^(?:@\w+(?:\([^)]*\))?\s+)*(?:public\s+|private\s+|internal\s+|fileprivate\s+|open\s+|final\s+|static\s+|class\s+|dynamic\s+|indirect\s+|convenience\s+|required\s+|override\s+|lazy\s+|weak\s+|unowned\s+)*(?:func|class|struct|enum|protocol|extension|actor|typealias|init|subscript|var|let)\b/;
// Symbol name after the keyword. Kotlin can put generics before the name
// (`fun <T> foo`); Swift puts them after (`func foo<T>`), so the name is the
// first identifier after the keyword in both.
// The `(?:[A-Za-z_][\w.]*\.)?` skips an extension receiver, so `fun Project.foo`
// and `val Project.bar` name `foo`/`bar`, not the pervasive receiver `Project`.
const KOTLIN_SYM = /\b(?:fun|class|interface|object|val|var|typealias)\s+(?:<[^>]*>\s+)?(?:[A-Za-z_][\w.]*\.)?([A-Za-z_]\w*)/;
const SWIFT_SYM = /\b(?:func|class|struct|enum|protocol|extension|actor|typealias|var|let)\s+([A-Za-z_][A-Za-z0-9_]*)/;
// Go: top-level func/type/var/const at column 0. The symbol skips an optional
// method receiver (`func (s *Server) Start` -> Start).
const GO_ITEM = /^(?:func|type|var|const)\b/;
const GO_SYM = /\b(?:func\s+(?:\([^)]*\)\s*)?|(?:type|var|const)\s+)([A-Za-z_]\w*)/;
// Python: top-level def/class at column 0 (indented methods stay with their class).
const PY_ITEM = /^(?:async\s+)?(?:def|class)\b/;
const PY_SYM = /\b(?:def|class)\s+([A-Za-z_]\w*)/;

export function langOf(file) {
  if (file.endsWith('.rs')) return 'rust';
  if (/\.(tsx?|jsx?|mjs|cjs)$/.test(file)) return 'typescript';
  if (/\.kts?$/.test(file)) return 'kotlin';
  if (file.endsWith('.swift')) return 'swift';
  if (file.endsWith('.go')) return 'go';
  if (file.endsWith('.py')) return 'python';
  return null;
}

/** `crate::nyxd::TxResponse` names the type `TxResponse`, not the crate. */
const lastSegment = (p) => p.split('::').filter(Boolean).pop() ?? '';

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
  let generics = '';
  if (line[j] === '<') {
    const open = j;
    let depth = 0;
    for (; j < line.length; j++) {
      if (line[j] === '<') depth++;
      else if (line[j] === '>' && --depth === 0) {
        j++;
        break;
      }
    }
    generics = line.slice(open, j);
  }
  // Type parameters declared by this impl. Lifetimes never match, since `'` is
  // not a leading identifier character.
  const declared = new Set(
    [...generics.matchAll(/(?:^|[<,])\s*(?:const\s+)?([A-Za-z_][A-Za-z0-9_]*)/g)].map((m) => m[1]),
  );
  // Stop at the body or a trailing comment. `impl Foo { // for Bar` otherwise
  // reads the comment's `for` and names the block after Bar.
  let rest = line.slice(j);
  const stop = rest.search(/\{|\/\//);
  if (stop !== -1) rest = rest.slice(0, stop);

  const forImpl = rest.match(/\bfor\s+(?:&\s*)?(?:'[A-Za-z0-9_]+\s+)?(?:mut\s+)?([A-Za-z_][A-Za-z0-9_:]*)/);
  if (forImpl) {
    const target = lastSegment(forImpl[1]);
    // `impl<C> DkgQueryClient for C` is a blanket impl over a type parameter.
    // The parameter is not a name anyone searches for, so fall through to the
    // trait, which is what the block is actually about.
    if (!declared.has(target)) return target;
  }
  const head = rest.match(/([A-Za-z_][A-Za-z0-9_:]*)/);
  return head ? lastSegment(head[1]) : '';
}

/** Best-effort symbol name from a boundary line, for the chunk heading. */
export function symbolOf(line, lang) {
  if (lang === 'typescript') {
    const m = line.match(TS_ITEM);
    return m ? m[1] : '';
  }
  if (lang === 'kotlin') {
    const m = line.match(KOTLIN_SYM);
    return m ? m[1] : '';
  }
  if (lang === 'swift') {
    const m = line.match(SWIFT_SYM);
    return m ? m[1] : '';
  }
  if (lang === 'go') {
    const m = line.match(GO_SYM);
    return m ? m[1] : '';
  }
  if (lang === 'python') {
    const m = line.match(PY_SYM);
    return m ? m[1] : '';
  }
  // impl first: an impl line can carry an item keyword inside its generic list
  // (`impl<const N: usize>`), and the keyword branch below would then name the
  // block after the generic parameter.
  if (/^\s*(?:default\s+)?(?:unsafe\s+)?impl\b/.test(line)) return implTarget(line);
  // rust: the identifier after the item keyword. The negative lookahead makes
  // `const fn foo` bind to `fn` rather than capturing `fn` as the name of a
  // const; `mut` is consumed so `static mut BUF` still yields `BUF`.
  const m = line.match(
    /\b(?:fn|struct|enum|trait|mod|type|static|const)\s+(?:mut\s+)?(?!fn\b|unsafe\b|extern\b|async\b)([A-Za-z_][A-Za-z0-9_]*)/,
  );
  return m ? m[1] : implTarget(line);
}

const ITEM_RE = { rust: RUST_ITEM, typescript: TS_ITEM, kotlin: KOTLIN_ITEM, swift: SWIFT_ITEM, go: GO_ITEM, python: PY_ITEM };
function isBoundary(line, lang) {
  return ITEM_RE[lang].test(line);
}

// Lines that document or annotate the item directly below them: Rust `///`/`//!`
// doc comments and `#[...]`/`#![...]` attributes; TS `//`, `/*`, `/**` and their
// ` *` continuation lines.
const DOC_ATTR = {
  rust: /^\s*(\/\/\/|\/\/!|#!?\[)/,
  typescript: /^\s*(\/\/|\/\*|\*)/,
  // KDoc `/** */`, line comments, and annotations (`@Foo`) precede the item.
  kotlin: /^\s*(\/\/|\/\*|\*|@)/,
  // Swift doc `///` and `/** */`, and attributes (`@objc`, `@MainActor`).
  swift: /^\s*(\/\/|\/\*|\*|@)/,
  go: /^\s*(\/\/|\/\*|\*)/,
  // Python decorators (`@app.route`) and comments precede the item; docstrings
  // sit inside and ride along in the body.
  python: /^\s*(#|@)/,
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
 * Chunk one file into retrieval records.
 *
 * Defaults reproduce the single-repo nym behaviour exactly (source 'nym-code',
 * nymtech/nym deep links, unprefixed ids), so the existing docs build and tests
 * are unaffected. A multi-repo caller passes `opts` to tag the shard: `repo`
 * namespaces the id so two repos cannot mint the same one, `source` labels the
 * shard, and `github` is that repo's deep-link base.
 *
 * @param {string} content
 * @param {string} repoPath repo-relative path, e.g. common/nymsphinx/src/lib.rs
 * @param {{ repo?: string, source?: string, github?: string }} [opts]
 */
export function chunkCodeFile(content, repoPath, opts = {}) {
  const { repo, source = 'nym-code', github = GITHUB } = opts;
  const lang = langOf(repoPath);
  if (!lang) return [];
  const idPrefix = repo ? `${repo}/` : '';
  return chunkCode(content, lang).map((c, i) => ({
    id: `${idPrefix}${repoPath}#${i}`,
    source,
    title: repoPath,
    heading: c.symbol || `${repoPath.split('/').pop()}`,
    url: `${github}/${repoPath}#L${c.startLine}`,
    text: c.text,
    lang,
  }));
}

export { MAX_CHARS, MIN_CHARS };
