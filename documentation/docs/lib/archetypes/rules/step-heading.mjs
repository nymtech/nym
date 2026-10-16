import { walk, isSteps, headingText, explicitAnchor, sectionOf } from './util.mjs';

// `Steps` numbers each heading from depth 2 to 6 with a CSS counter, so a number
// in the text shows twice.
const LEADING_NUMBER = /^\s*(?:step\s+)?\d+(?:\.\d+)*\s*[.):]?\s+/i;

// split cuts a page at depth 2 to 4, and a step is a cut. A step sits one level
// below the section that holds its `Steps`, and at depth 3 when that section is
// the page title or a depth 2 heading.
const DEEPEST_STEP = 4;

export const id = 'step-heading';

export function check(tree, { report }) {
  walk(tree, (node, ancestors) => {
    if (node.type !== 'heading') return;
    const steps = ancestors.findLastIndex(isSteps);
    if (steps === -1) return;
    const sectionDepth = sectionOf(ancestors.slice(0, steps + 1)).depth;
    const text = headingText(node);
    const want = Math.max(3, sectionDepth + 1);
    if (want > DEEPEST_STEP) report(node, `step heading is under a section at depth ${sectionDepth}; raise that section to depth ${DEEPEST_STEP - 1} and put its steps at depth ${DEEPEST_STEP}`);
    else if (node.depth !== want) report(node, `step heading is at depth ${node.depth}, use depth ${want}`);
    if (LEADING_NUMBER.test(text)) report(node, `step heading "${text}" holds a number; Steps numbers it`);
    if (!explicitAnchor(node)) report(node, `step heading "${text}" has no explicit anchor ([#anchor])`);
  });
}
