import { walk } from './util.mjs';

export const id = 'placeholder';

// The D17 token grammar: `<` + `[A-Za-z][A-Za-z0-9_./-]*` + `>`. `(?<!<)` skips a
// heredoc (`<<EOF`), and `(?<!\/)` a self-closing tag (`<br/>`). A closing tag
// and a tag with attributes do not match at all.
const TOKEN = /(?<!<)<([A-Za-z][\w./-]*?)(?<!\/)>/g;
const HTML_ELEMENTS = new Set(['div', 'br', 'code', 'strong', 'b', 'a', 'span', 'p', 'img', 'details', 'summary']);
// Blocks whose `<...>` are generics or markup, not placeholders.
const SKIP_LANGS = new Set(['rust', 'ts', 'html', 'xml']);

export function check(tree, { report, placeholders }) {
  walk(tree, (node) => {
    const inCode = node.type === 'code' && !SKIP_LANGS.has(node.lang);
    if (!inCode && node.type !== 'inlineCode') return;
    for (const [token, name] of node.value.matchAll(TOKEN)) {
      if (HTML_ELEMENTS.has(name)) continue;
      const entry = placeholders.byToken.get(token);
      if (entry?.name === token) continue;
      if (entry) report(node, `placeholder ${token} is an alias`, entry.name);
      else report(node, `placeholder ${token} is not in placeholders.json`);
    }
  });
}
