import { walk, isSteps } from './util.mjs';

export const id = 'steps-per-procedure';

// One `Steps` element is one procedure. A page with two or more puts each one
// under its own depth 2 heading, with no other `Steps` in between.
export function check(tree, { report }) {
  const order = [];
  walk(tree, (node, ancestors) => {
    if (ancestors.some(isSteps)) return;
    if (isSteps(node)) order.push({ kind: 'steps', node });
    else if (node.type === 'heading' && node.depth === 2) order.push({ kind: 'h2' });
  });
  const steps = order.filter((e) => e.kind === 'steps');
  if (steps.length < 2) return;
  let headed = false;
  for (const entry of order) {
    if (entry.kind === 'h2') headed = true;
    else {
      if (!headed) report(entry.node, `procedure ${steps.indexOf(entry) + 1} of ${steps.length} has no depth 2 heading of its own`);
      headed = false;
    }
  }
}
