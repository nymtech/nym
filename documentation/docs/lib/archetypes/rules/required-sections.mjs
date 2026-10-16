import { walk, isSteps, headingText } from './util.mjs';

export const id = 'required-sections';

// Each section of the archetype with `required: true`. A section with `element`
// is that JSX element, and each one must hold a step heading. A section with
// `depth` is a heading at that depth, with the text `heading` when one is given.
export function check(tree, { report, archetype }) {
  const outer = [];
  const headings = [];
  walk(tree, (node, ancestors) => {
    if (isSteps(node) && !ancestors.some(isSteps)) outer.push(node);
    if (node.type === 'heading') headings.push(node);
  });
  for (const section of archetype.sections.filter((s) => s.required)) {
    if (section.element === 'Steps') {
      if (outer.length === 0) report(tree, 'page has no Steps element');
      for (const element of outer) {
        let hasStep = false;
        walk(element, (node) => { if (node.type === 'heading') hasStep = true; });
        if (!hasStep) report(element, 'Steps element holds no step heading');
      }
      continue;
    }
    const found = headings.some((h) => h.depth === section.depth && (section.heading === undefined || headingText(h) === section.heading));
    if (!found) report(tree, `page has no "${section.name}" section (a depth ${section.depth} heading${section.heading ? ` "${section.heading}"` : ''})`);
  }
}
