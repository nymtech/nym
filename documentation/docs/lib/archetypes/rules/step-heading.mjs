import { walk, isSteps, headingText, explicitAnchor } from './util.mjs';

// `Steps` numbers each heading from depth 2 to 6 with a CSS counter, so a number
// in the text shows twice.
const LEADING_NUMBER = /^\s*(?:step\s+)?\d+(?:\.\d+)*\s*[.):]?\s+/i;

export const id = 'step-heading';

export function check(tree, { report }) {
  walk(tree, (node, ancestors) => {
    if (node.type !== 'heading' || !ancestors.some(isSteps)) return;
    const text = headingText(node);
    if (node.depth !== 3) report(node, `step heading is at depth ${node.depth}, use depth 3`);
    if (LEADING_NUMBER.test(text)) report(node, `step heading "${text}" holds a number; Steps numbers it`);
    if (!explicitAnchor(node)) report(node, `step heading "${text}" has no explicit anchor ([#anchor])`);
  });
}
