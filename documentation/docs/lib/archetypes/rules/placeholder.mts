import type { Root } from '../mdast.mts';
import type { RuleContext } from '../types.mts';
import { walk } from './util.mts';

export const id = 'placeholder';

// The D17 token grammar: `<` + `[A-Z][A-Za-z0-9_./-]*` + `>`. A placeholder
// starts with a capital letter (decided 2026-10-16), so a hostname such as
// `<api.nyx.yourdomain.tld>` or a word such as `<file>` is not one. `(?<!<)`
// skips a heredoc (`<<EOF`), and `(?<!\/)` a self-closing tag (`<br/>`).
const TOKEN = /(?<!<)<([A-Z][\w./-]*?)(?<!\/)>/g;
const HTML_ELEMENTS = new Set(['div', 'br', 'code', 'strong', 'b', 'a', 'span', 'p', 'img', 'details', 'summary']);
// Blocks whose `<...>` are generics or markup, not placeholders.
const SKIP_LANGS = new Set(['rust', 'ts', 'html', 'xml']);

export function check(tree: Root, { report, placeholders }: RuleContext): void {
  walk(tree, (node) => {
    const text = node.type === 'inlineCode' || (node.type === 'code' && !SKIP_LANGS.has(node.lang ?? '')) ? node.value : null;
    if (text === null) return;
    for (const [token, name] of text.matchAll(TOKEN)) {
      if (name === undefined || HTML_ELEMENTS.has(name)) continue;
      const entry = placeholders.byToken.get(token);
      if (entry?.name === token) continue;
      if (entry) report(node, `placeholder ${token} is an alias`, entry.name);
      else report(node, `placeholder ${token} is not in placeholders.json`);
    }
  });
}
