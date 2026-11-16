// The parsers of the lint and the anchor check, and the mdast node types they
// give. The types come through remark-parse and remark-mdx, which bring the
// mdast and MDX type packages, so this package needs no direct dependency on them.

import { unified } from 'unified';
import remarkParse from 'remark-parse';
import remarkMdx from 'remark-mdx';
import remarkFrontmatter from 'remark-frontmatter';

export const mdxParser = unified().use(remarkParse).use(remarkMdx).use(remarkFrontmatter);
export const mdParser = unified().use(remarkParse).use(remarkFrontmatter);

export type Root = ReturnType<typeof mdxParser.parse>;
/** Any node below the root: the mdast, frontmatter, and MDX nodes. */
export type Content = Root['children'][number];
export type Node = Root | Content;
export type Parent = Extract<Node, { children: unknown }>;
export type Heading = Extract<Node, { type: 'heading' }>;
export type JsxElement = Extract<Node, { type: 'mdxJsxFlowElement' | 'mdxJsxTextElement' }>;
export type JsxAttribute = Extract<JsxElement['attributes'][number], { type: 'mdxJsxAttribute' }>;

export const isJsx = (node: Node | undefined): node is JsxElement =>
  node?.type === 'mdxJsxFlowElement' || node?.type === 'mdxJsxTextElement';

/** The file a node came from, which inline.mts sets, so a fault names the file of its text. */
export const fileOf = (node: Node): string | undefined => (node.data as { file?: string } | undefined)?.file;

export function setFile(node: Node, file: string): void {
  // `Data` has no `file` field: each node keeps it next to the fields that mdast declares.
  (node as { data?: object }).data = { ...node.data, file };
}

export const messageOf = (error: unknown): string => (error instanceof Error ? error.message : String(error));
