// The named items of a source file, with tree-sitter: one function for the
// snippet extractor (which cites items by symbol) and, later, the code chunker
// of the shared index (add-shared-retrieval-index decision 10).
//
//   const list = await items(text, 'rust');
//   // [{ name, qualifiedName, kind, startLine, endLine }], 1-indexed, inclusive
//
// The range of an item includes its doc comments and attributes. `qualifiedName`
// is the path of the item inside the file: `module::Type::method`. A method in
// `impl Trait for Type` is `<Type as Trait>::method`, Rust's own fully qualified
// form, and it also answers to `Type::method`.

import { createRequire } from 'node:module';
import { Parser, Language } from 'web-tree-sitter';

const require = createRequire(import.meta.url);
const GRAMMARS = { rust: 'tree-sitter-rust/tree-sitter-rust.wasm' };

// Rust node type -> item kind. `impl_item` is not an item a citation names:
// its methods are.
const RUST_KINDS = {
  function_item: 'fn',
  function_signature_item: 'fn',
  struct_item: 'struct',
  enum_item: 'enum',
  union_item: 'union',
  trait_item: 'trait',
  mod_item: 'mod',
  const_item: 'const',
  static_item: 'static',
  type_item: 'type',
  macro_definition: 'macro',
  associated_type: 'type',
};

const parsers = new Map();

async function parserFor(lang) {
  if (parsers.has(lang)) return parsers.get(lang);
  if (!GRAMMARS[lang]) throw new Error(`no grammar for language "${lang}"`);
  await Parser.init();
  const parser = new Parser();
  parser.setLanguage(await Language.load(require.resolve(GRAMMARS[lang])));
  parsers.set(lang, parser);
  return parser;
}

/**
 * One spelling for a type or trait: whitespace collapsed, none inside `<...>`
 * at the brackets, one space after a comma. `W <T>` and `W<T>` read the same,
 * and `<W<T> as Clone>` keeps the spaces around `as`.
 */
export const squash = (text) => text
  .replace(/\s+/g, ' ')
  .replace(/(\w)\s+</g, '$1<')
  .replace(/<\s+/g, '<')
  .replace(/\s+>/g, '>')
  .replace(/\s*,\s*/g, ', ')
  .trim();
/** A path with a leading `crate::`, `self::`, or `super::` removed: the item path is inside the file. */
const local = (text) => text.replace(/^(?:(?:crate|self|super)::)+/, '');
/** A type with its generic arguments removed: `Foo<T>` -> `Foo`. */
const bareType = (text) => local(squash(text)).replace(/<.*$/, '');
/** A trait with its module path removed: `fmt::Display` -> `Display`, `From<A>` stays. */
const lastSegment = (text) => { const t = squash(text); const head = t.replace(/<.*$/, ''); return head.split('::').pop() + t.slice(head.length); };

/**
 * The start row of an item with the doc comments and attributes directly above
 * it. Plain `//` comments and blank lines end the run.
 */
function startWithDocs(node) {
  let start = node.startPosition.row;
  for (let prev = node.previousNamedSibling; prev; prev = prev.previousNamedSibling) {
    // A line comment ends at column 0 of the next row, because it holds its newline.
    // An attribute can sit on the item's own row (`#[inline] fn a()`).
    const lastRow = prev.endPosition.column === 0 ? prev.endPosition.row - 1 : prev.endPosition.row;
    if (lastRow !== start - 1 && lastRow !== start) break;
    const isAttribute = prev.type === 'attribute_item';
    // `///` and `/**` are doc comments; `////`, `/**/`, and `/***` are plain comments.
    const isDoc = (prev.type === 'line_comment' || prev.type === 'block_comment') && /^(\/\/\/(?!\/)|\/\*\*(?![*/]))/.test(prev.text);
    if (!isAttribute && !isDoc) break;
    start = prev.startPosition.row;
  }
  return start;
}

/**
 * @param {string} text  source text
 * @param {string} lang  'rust'
 * @returns {Promise<{ name: string, qualifiedName: string, aliases: string[], kind: string, startLine: number, endLine: number }[]>}
 */
export async function items(text, lang) {
  const parser = await parserFor(lang);
  const tree = parser.parse(text);
  const found = [];

  const walk = (node, prefix) => {
    for (const child of node.namedChildren) {
      if (child.type === 'impl_item') {
        const type = child.childForFieldName('type');
        const trait = child.childForFieldName('trait');
        const body = child.childForFieldName('body');
        if (!type || !body) continue;
        const owner = trait ? `<${local(squash(type.text))} as ${squash(trait.text)}>` : bareType(type.text);
        // A trait-impl method also answers to `Type::method` and to the trait's last path segment.
        const aliasOwners = trait ? [bareType(type.text), `<${local(squash(type.text))} as ${lastSegment(trait.text)}>`].filter((a) => a !== owner) : [];
        for (const member of body.namedChildren) {
          const kind = RUST_KINDS[member.type];
          if (!kind) continue;
          const name = member.childForFieldName('name')?.text;
          if (!name) continue;
          found.push({
            name,
            qualifiedName: `${prefix}${owner}::${name}`,
            aliases: aliasOwners.map((a) => `${prefix}${a}::${name}`),
            kind,
            startLine: startWithDocs(member) + 1,
            endLine: member.endPosition.row + 1,
          });
        }
        continue;
      }
      // An `extern` block holds items at the level of its parent.
      if (child.type === 'foreign_mod_item') {
        const body = child.childForFieldName('body') ?? child.namedChildren.find((c) => c.type === 'declaration_list');
        if (body) walk(body, prefix);
        continue;
      }
      const kind = RUST_KINDS[child.type];
      if (!kind) continue;
      const name = child.childForFieldName('name')?.text;
      if (!name) continue;
      found.push({
        name,
        qualifiedName: `${prefix}${name}`,
        aliases: [],
        kind,
        startLine: startWithDocs(child) + 1,
        endLine: child.endPosition.row + 1,
      });
      const body = child.childForFieldName('body');
      if (body && (kind === 'mod' || kind === 'trait')) walk(body, `${prefix}${name}::`);
    }
  };
  walk(tree.rootNode, '');
  tree.delete();
  return found;
}

/** Split a path on `::`, but not inside `<...>`. */
export function pathSegments(qualified) {
  const out = [];
  let depth = 0;
  let current = '';
  for (let i = 0; i < qualified.length; i++) {
    const c = qualified[i];
    if (c === '<') depth++;
    if (c === '>') depth--;
    if (depth === 0 && qualified.startsWith('::', i)) {
      out.push(current);
      current = '';
      i++;
      continue;
    }
    current += c;
  }
  out.push(current);
  return out;
}

/**
 * The items a symbol names: each item whose qualified name or alias ends with
 * the symbol's path segments. `try_update_state` matches every method of that
 * name; `ClassicPeriodicAllowance::try_update_state` matches one.
 */
export function matchSymbol(list, symbol) {
  const want = pathSegments(squash(symbol));
  const endsWith = (qualified) => {
    const have = pathSegments(qualified);
    return have.length >= want.length && want.every((seg, i) => have[have.length - want.length + i] === seg);
  };
  return list.filter((item) => [item.qualifiedName, ...item.aliases].some(endsWith));
}
