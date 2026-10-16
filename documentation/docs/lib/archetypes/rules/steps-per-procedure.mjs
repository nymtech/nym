import { walk, isSteps, sectionOf } from './util.mjs';

export const id = 'steps-per-procedure';

// One `Steps` element is one procedure. A page with two or more puts each one
// in a section of its own: a heading at depth 2 to 4 (as split cuts) on its own
// ancestor chain, or a tab panel (its label names the procedure, and the tabs
// hold alternatives). Two `Steps` in one section are a fault on the second.
export function check(tree, { report }) {
  const procedures = [];
  walk(tree, (node, ancestors) => {
    if (isSteps(node) && !ancestors.some(isSteps)) procedures.push({ node, section: sectionOf([...ancestors, node]) });
  });
  if (procedures.length < 2) return;
  const seen = new Set();
  procedures.forEach(({ node, section }, i) => {
    const { key, heading } = section;
    const headed = key && (key !== heading || (heading.depth >= 2 && heading.depth <= 4));
    if (!headed || seen.has(key)) report(node, `procedure ${i + 1} of ${procedures.length} has no section heading of its own`);
    seen.add(key);
  });
}
