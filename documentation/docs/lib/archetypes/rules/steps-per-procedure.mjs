import { walk, isSteps } from './util.mjs';

export const id = 'steps-per-procedure';

// One `Steps` element is one procedure. A page with two or more puts each one
// under a section heading of its own (depth 2 to 4, as split cuts), with no
// other `Steps` in between. A tab panel counts as such a heading: its label
// names the procedure, and the tabs hold alternatives.
export function check(tree, { report }) {
  const order = [];
  walk(tree, (node, ancestors) => {
    if (ancestors.some(isSteps)) return;
    if (isSteps(node)) order.push({ kind: 'steps', node });
    else if (node.type === 'heading' && node.depth >= 2 && node.depth <= 4) order.push({ kind: 'heading' });
    else if (node.type === 'mdxJsxFlowElement' && ancestors.at(-1)?.name === 'Tabs') order.push({ kind: 'heading' });
  });
  const steps = order.filter((e) => e.kind === 'steps');
  if (steps.length < 2) return;
  let headed = false;
  for (const entry of order) {
    if (entry.kind === 'heading') headed = true;
    else {
      if (!headed) report(entry.node, `procedure ${steps.indexOf(entry) + 1} of ${steps.length} has no section heading of its own`);
      headed = false;
    }
  }
}
