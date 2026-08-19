import { describe, it, expect } from 'vitest';
// @ts-expect-error - plain ESM JS module, no type declarations
import { parseFrontmatter, pageTitle, pageDescription, stripMdx } from './mdx.mjs';

describe('parseFrontmatter (gray-matter, head-anchored)', () => {
  it('parses leading YAML frontmatter into data + body', () => {
    const { data, content } = parseFrontmatter('---\ntitle: Real Title\ndescription: A desc\n---\n\n# H1\nbody');
    expect(data.title).toBe('Real Title');
    expect(data.description).toBe('A desc');
    expect(content.trim().startsWith('# H1')).toBe(true);
  });

  it('does not treat a body `---` as frontmatter (the bug the regex versions had)', () => {
    const raw = '# Title\n\nsome text\n\n---\n\nmore text after a horizontal rule';
    const { data, content } = parseFrontmatter(raw);
    expect(Object.keys(data)).toHaveLength(0);
    expect(content).toContain('more text after a horizontal rule');
  });

  it('handles a title containing a colon (YAML, not a naive regex)', () => {
    const { data } = parseFrontmatter('---\ntitle: "nym-sdk: the Rust SDK"\n---\nbody');
    expect(data.title).toBe('nym-sdk: the Rust SDK');
  });
});

describe('pageTitle / pageDescription', () => {
  it('prefers frontmatter title, then first H1, then a humanised fallback', () => {
    expect(pageTitle({ title: 'FM' }, '# H1\nx', 'the-slug')).toBe('FM');
    expect(pageTitle({}, '# My Heading\nx', 'the-slug')).toBe('My Heading');
    expect(pageTitle({}, 'no heading here', 'the-slug')).toBe('the slug');
  });

  it('reads description from frontmatter, else empty string', () => {
    expect(pageDescription({ description: 'd' })).toBe('d');
    expect(pageDescription({})).toBe('');
  });
});

describe('stripMdx (fence-aware)', () => {
  it('strips imports and lone JSX tag lines outside fences, keeping children', () => {
    const body = ['import X from "y";', '', 'text', '', '<Callout>', 'keep me', '</Callout>'].join('\n');
    const s = stripMdx(body);
    expect(s).not.toContain('import X');
    expect(s).not.toContain('<Callout>');
    expect(s).toContain('keep me');
  });

  it('never touches lines inside a code fence (the llms-full.txt corruption bug)', () => {
    const body = ['text', '', '```ts', 'import A from "b";', '<Component/>', '```'].join('\n');
    const s = stripMdx(body);
    expect(s).toContain('import A from "b";');
    expect(s).toContain('<Component/>');
  });
});
