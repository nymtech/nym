// Parse a docs page into one mdast tree with each imported `.mdx` / `.md` file
// spliced in where its tag is, at every level. Each node keeps the file it came
// from in `data.file`, so a lint fault names the file and line where the text
// is, not the page that imports it.
//
// This is the mdast counterpart of `inlineMdxPartials` in lib/retrieval/mdx.mjs,
// which works on text and so loses the source position of each line. Both
// resolve import paths with the same `resolveImport`.

import fs from 'node:fs';
import { unified } from 'unified';
import remarkParse from 'remark-parse';
import remarkMdx from 'remark-mdx';
import remarkFrontmatter from 'remark-frontmatter';
import { resolveImport } from '../retrieval/mdx.mjs';

// Elements whose children stay in the mdast, so they hide no procedure.
const TRANSPARENT_SOURCES = new Set(['nextra/components']);

const mdxParser = unified().use(remarkParse).use(remarkMdx).use(remarkFrontmatter);
const mdParser = unified().use(remarkParse).use(remarkFrontmatter);

/**
 * @param {string} filePath  the page file
 * @param {{ root: string }} opts  root for bare import paths (the docs root)
 * @returns {{ tree: object, unchecked: string[], warnings: object[] }}
 *   `unchecked` lists the TSX components with no children that the page renders:
 *   their content is not in the tree, so the lint cannot see it.
 */
export function inlinePage(filePath, { root }) {
  const raw = fs.readFileSync(filePath, 'utf-8');
  const unchecked = new Set();
  const warnings = [];
  const tree = parseFile(filePath, raw, { root, seen: new Set([filePath]), unchecked, warnings });
  return { tree, unchecked: [...unchecked].sort(), warnings };
}

function parseFile(filePath, raw, ctx) {
  const parser = filePath.endsWith('.md') ? mdParser : mdxParser;
  let tree;
  try {
    tree = parser.parse(raw);
  } catch (error) {
    throw new Error(`${filePath}: ${error.message}`, { cause: error });
  }
  markFile(tree, filePath);

  const imports = importsOf(tree);
  splice(tree, (node) => {
    // `<Foo.Bar />` comes from the import of `Foo`.
    const spec = imports.get(node.name?.split('.')[0]);
    if (spec === undefined) return null;
    if (/\.mdx?$/.test(spec)) {
      const resolved = resolveImport(spec, { filePath, root: ctx.root });
      if (!fs.existsSync(resolved)) {
        ctx.warnings.push({ file: filePath, line: node.position?.start.line, message: `cannot resolve the import '${spec}'` });
        return null;
      }
      if (ctx.seen.has(resolved)) return null;
      const nested = parseFile(resolved, fs.readFileSync(resolved, 'utf-8'), {
        ...ctx,
        seen: new Set([...ctx.seen, resolved]),
      });
      return nested.children.filter((n) => n.type !== 'yaml' && n.type !== 'mdxjsEsm');
    }
    // A component with children leaves them in the mdast; one with none renders
    // content the lint cannot see.
    if (!TRANSPARENT_SOURCES.has(spec) && node.children.length === 0) ctx.unchecked.add(node.name);
    return null;
  });
  return tree;
}

/** Name -> import specifier, for every default and named import in the file. */
function importsOf(tree) {
  const map = new Map();
  for (const node of tree.children) {
    if (node.type !== 'mdxjsEsm') continue;
    for (const [, clause, spec] of node.value.matchAll(/^import\s+([^'"]+?)\s+from\s+['"]([^'"]+)['"]/gm)) {
      // `A`, `{ A, B as C }`, `* as D`: the local name is the last word of each part.
      for (const part of clause.replace(/[{}]/g, ',').split(',')) {
        const local = part.trim().split(/\s+/).pop();
        if (local) map.set(local, spec);
      }
    }
  }
  return map;
}

/**
 * Walk the tree. For each JSX element, `replace(node)` returns the nodes to put
 * in its place, or null to keep it (and walk its children).
 */
function splice(node, replace) {
  if (!node.children) return;
  const out = [];
  for (const child of node.children) {
    const isJsx = child.type === 'mdxJsxFlowElement' || child.type === 'mdxJsxTextElement';
    const replacement = isJsx ? replace(child) : null;
    if (replacement) {
      out.push(...replacement);
    } else {
      splice(child, replace);
      out.push(child);
    }
  }
  node.children = out;
}

function markFile(node, file) {
  node.data = { ...node.data, file };
  for (const child of node.children ?? []) markFile(child, file);
}
