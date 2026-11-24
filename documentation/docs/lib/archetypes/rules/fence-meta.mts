import type { Root } from '../mdast.mts';
import type { RuleContext } from '../types.mts';
import { walk } from './util.mts';

export const id = 'fence-meta';

// Meta tokens Nextra and rehype-pretty-code read: `key`, `key=value`,
// `key="value"`, a `{1,3-4}` line range, and a `/word/` highlight.
const TOKEN = /\{[^}]*\}|\/[^/]*\/|[\w-]+(?:=(?:"[^"]*"|\S+))?|\S+/g;

export function check(tree: Root, { report, archetype }: RuleContext): void {
  walk(tree, (node) => {
    if (node.type !== 'code' || !node.meta) return;
    for (const token of node.meta.match(TOKEN) ?? []) {
      if (/^\{.*\}$/.test(token) || /^\/.*\/$/.test(token)) continue;
      const key = token.split('=')[0] ?? token;
      if (!archetype.metaKeys.includes(key)) report(node, `code block meta "${token}" is not an allowed key`);
    }
  });
}
