// Small mdast helpers shared by the rules.

import type { Heading, Node, Parent } from '../mdast.mts';

/** Call `fn(node, ancestors)` for every node, in document order. */
export function walk(node: Node, fn: (node: Node, ancestors: Parent[]) => void, ancestors: Parent[] = []): void {
  fn(node, ancestors);
  if (!('children' in node)) return;
  for (const child of node.children) walk(child, fn, [...ancestors, node]);
}

export const isSteps = (node: Node | undefined): boolean => node?.type === 'mdxJsxFlowElement' && node.name === 'Steps';

/** The plain text of a node and its children. */
export function textOf(node: Node): string {
  if ('value' in node && typeof node.value === 'string') return node.value;
  if (!('children' in node)) return '';
  const children: Node[] = node.children;
  return children.map(textOf).join('');
}

/**
 * The explicit anchor of a heading, by the Nextra 4.6.1 rule
 * (remark-custom-heading-id): `[#anchor]` at the end of the last child, when
 * that child is text.
 */
const ANCHOR = /\s*\[#([^]+?)]\s*$/;

export function explicitAnchor(heading: Heading): string | null {
  const last = heading.children.at(-1);
  if (last?.type !== 'text') return null;
  return last.value.match(ANCHOR)?.[1] ?? null;
}

/** The text of a heading as a reader sees it: no explicit anchor. */
export const headingText = (heading: Heading): string => textOf(heading).replace(ANCHOR, '').trim();

const isTabs = (node: Node | undefined): boolean => node?.type === 'mdxJsxFlowElement' && node.name === 'Tabs';

export interface Section {
  depth: number;
  heading: Heading | null;
  key: Node | null;
}

/**
 * The section of a node: the nearest heading before it on its own ancestor
 * chain (a preceding sibling at some level), so a heading inside a sibling
 * element (a `Callout`, another tab panel) does not count. Step headings, the
 * children of a `Steps` element, are not sections. `key` is that heading, or
 * the tab panel the node is in when the panel comes first: the label of a panel
 * names what it holds. `chain` is the ancestors of the node and the node.
 */
export function sectionOf(chain: Node[]): Section {
  let key: Node | null = null;
  for (let i = chain.length - 1; i > 0; i--) {
    const child = chain[i];
    const parent = chain[i - 1];
    if (child === undefined || parent === undefined || !('children' in parent)) continue;
    if (!isSteps(parent)) {
      const siblings: Node[] = parent.children;
      const heading = siblings.slice(0, siblings.indexOf(child)).findLast((n): n is Heading => n.type === 'heading');
      if (heading) return { depth: heading.depth, heading, key: key ?? heading };
    }
    if (!key && isTabs(parent)) key = child;
  }
  return { depth: 1, heading: null, key };
}
