// Parse a docs page into one mdast tree with each imported `.mdx` / `.md` file
// spliced in where its tag is, at every level. Each node keeps the file it came
// from (mdast.mts `fileOf`), so a lint fault names the file and line where the
// text is, not the page that imports it.
//
// This is the mdast counterpart of `inlineMdxPartials` in lib/retrieval/mdx.mjs,
// which works on text and so loses the source position of each line. Both
// resolve import paths with the same `resolveImport`.

import fs from 'node:fs';
import { resolveImport } from '../retrieval/mdx.mjs';
import type { Content, JsxElement, Node, Root } from './mdast.mts';
import { isJsx, mdParser, mdxParser, messageOf, setFile } from './mdast.mts';
import type { LintWarning } from './types.mts';

// Elements whose children stay in the mdast, so they hide no procedure.
const TRANSPARENT_SOURCES = new Set(['nextra/components']);

export interface InlineOptions {
  /** The root for bare import paths: the docs root. */
  root: string;
  /** Import aliases such as `@snippets/nym/` (repo.mts `openRepo`). */
  aliases?: Record<string, string>;
}

export interface InlinedPage {
  tree: Root;
  /** The TSX components with no children that the page renders: their content is not in the tree, so the lint cannot see it. */
  unchecked: string[];
  warnings: LintWarning[];
}

interface Context {
  root: string;
  aliases: Record<string, string>;
  seen: Set<string>;
  unchecked: Set<string>;
  warnings: LintWarning[];
}

export function inlinePage(filePath: string, { root, aliases = {} }: InlineOptions): InlinedPage {
  const raw = fs.readFileSync(filePath, 'utf-8');
  const unchecked = new Set<string>();
  const warnings: LintWarning[] = [];
  const tree = parseFile(filePath, raw, { root, aliases, seen: new Set([filePath]), unchecked, warnings });
  return { tree, unchecked: [...unchecked].sort(), warnings };
}

function parseFile(filePath: string, raw: string, ctx: Context): Root {
  const parser = filePath.endsWith('.md') ? mdParser : mdxParser;
  let tree: Root;
  try {
    tree = parser.parse(raw);
  } catch (error) {
    throw new Error(`${filePath}: ${messageOf(error)}`, { cause: error });
  }
  markFile(tree, filePath);

  const imports = importsOf(tree);
  splice(tree, (node) => {
    // `<Foo.Bar />` comes from the import of `Foo`. A fragment (`<>`) has no name.
    if (!node.name) return null;
    const spec = imports.get(node.name.split('.')[0] ?? node.name);
    if (spec === undefined) return null;
    if (/\.mdx?$/.test(spec)) {
      const resolved: string = resolveImport(spec, { filePath, root: ctx.root, aliases: ctx.aliases });
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
function importsOf(tree: Root): Map<string, string> {
  const map = new Map<string, string>();
  for (const node of tree.children) {
    if (node.type !== 'mdxjsEsm') continue;
    for (const [, clause, spec] of node.value.matchAll(/^import\s+([^'"]+?)\s+from\s+['"]([^'"]+)['"]/gm)) {
      if (clause === undefined || spec === undefined) continue;
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
function splice(node: Node, replace: (element: JsxElement) => Content[] | null): void {
  if (!('children' in node)) return;
  const out: Node[] = [];
  for (const child of node.children) {
    const replacement = isJsx(child) ? replace(child) : null;
    if (replacement) {
      out.push(...replacement);
    } else {
      splice(child, replace);
      out.push(child);
    }
  }
  // An imported file can put any kind of node in any parent, so the children
  // here are wider than the mdast type of the parent allows.
  (node as { children: Node[] }).children = out;
}

function markFile(node: Node, file: string): void {
  setFile(node, file);
  if (!('children' in node)) return;
  for (const child of node.children) markFile(child, file);
}
