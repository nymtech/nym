// Small mdast helpers shared by the rules.

/** Call `fn(node, ancestors)` for every node, in document order. */
export function walk(node, fn, ancestors = []) {
  fn(node, ancestors);
  for (const child of node.children ?? []) walk(child, fn, [...ancestors, node]);
}

export const isSteps = (node) => node.type === 'mdxJsxFlowElement' && node.name === 'Steps';

/** The plain text of a node and its children. */
export function textOf(node) {
  if (typeof node.value === 'string') return node.value;
  return (node.children ?? []).map(textOf).join('');
}

/**
 * The explicit anchor of a heading, by the Nextra 4.6.1 rule
 * (remark-custom-heading-id): `[#anchor]` at the end of the last child, when
 * that child is text.
 */
const ANCHOR = /\s*\[#([^]+?)]\s*$/;

export function explicitAnchor(heading) {
  const last = heading.children?.at(-1);
  if (last?.type !== 'text') return null;
  return last.value.match(ANCHOR)?.[1] ?? null;
}

/** The text of a heading as a reader sees it: no explicit anchor. */
export const headingText = (heading) => textOf(heading).replace(ANCHOR, '').trim();
