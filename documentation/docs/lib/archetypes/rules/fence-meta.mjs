import { walk } from './util.mjs';

export const id = 'fence-meta';

// Meta tokens Nextra and rehype-pretty-code read: `key`, `key=value`,
// `key="value"`, a `{1,3-4}` line range, and a `/word/` highlight.
const TOKEN = /\{[^}]*\}|\/[^/]*\/|[\w-]+(?:=(?:"[^"]*"|\S+))?|\S+/g;

export function check(tree, { report, archetype }) {
  walk(tree, (node) => {
    if (node.type !== 'code' || !node.meta) return;
    for (const token of node.meta.match(TOKEN) ?? []) {
      if (/^\{.*\}$/.test(token) || /^\/.*\/$/.test(token)) continue;
      const key = token.split('=')[0];
      if (!archetype.metaKeys.includes(key)) report(node, `code block meta "${token}" is not an allowed key`);
    }
  });
}
