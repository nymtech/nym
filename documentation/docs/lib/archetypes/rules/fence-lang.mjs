import { walk } from './util.mjs';

export const id = 'fence-lang';

export function check(tree, { report, archetype }) {
  walk(tree, (node) => {
    if (node.type !== 'code') return;
    if (!node.lang) return report(node, 'code block has no language');
    const fix = archetype.languageFixes[node.lang];
    if (fix) return report(node, `code block language "${node.lang}"`, fix);
    if (!archetype.languages.includes(node.lang)) report(node, `code block language "${node.lang}" is not in the archetype list`);
  });
}
