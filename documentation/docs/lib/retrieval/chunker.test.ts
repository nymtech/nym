import { describe, it, expect } from 'vitest';
// @ts-expect-error - plain ESM JS module, no type declarations
import { chunkPages, splitByHeadings, packSection, slugify, MAX_CHARS } from './chunker.mjs';

const SITE = 'https://nym.com/docs';

function page(body: string, extra: Partial<{ title: string; url: string; source: string }> = {}) {
  return {
    source: extra.source ?? 'nym-docs',
    title: extra.title ?? 'Test Page',
    url: extra.url ?? `${SITE}/test/page`,
    body,
  };
}

describe('slugify', () => {
  it('lowercases, strips punctuation, hyphenates spaces', () => {
    expect(slugify('Running your Nym API instance!')).toBe('running-your-nym-api-instance');
  });
});

describe('splitByHeadings (fence-aware)', () => {
  it('does not treat ## inside a fenced code block as a heading', () => {
    const body = ['intro text', '```sh', '## this is a comment, not a heading', 'echo hi', '```', '', '## Real Heading', 'body'].join('\n');
    const sections = splitByHeadings(body, 'Page');
    const headings = sections.map((s: { heading: string }) => s.heading);
    expect(headings).toContain('Real Heading');
    expect(headings).not.toContain('this is a comment, not a heading');
  });

  it('attaches pre-heading content to the page title', () => {
    const sections = splitByHeadings('lead paragraph\n\n## Section', 'My Page');
    expect(sections[0].heading).toBe('My Page');
    expect(sections[0].text).toContain('lead paragraph');
  });
});

describe('packSection (hard-cap fallback)', () => {
  it('splits a single oversized block so no part exceeds the cap', () => {
    const giant = 'x'.repeat(MAX_CHARS * 3);
    const parts = packSection(giant, MAX_CHARS);
    expect(parts.length).toBeGreaterThan(1);
    for (const p of parts) expect(p.length).toBeLessThanOrEqual(MAX_CHARS);
  });

  it('leaves small text as a single part', () => {
    expect(packSection('short', MAX_CHARS)).toEqual(['short']);
  });
});

describe('chunkPages', () => {
  it('de-duplicates repeated heading anchors with -1/-2 suffixes', () => {
    const body = '## Options\nfirst\n\n## Options\nsecond\n\n## Options\nthird';
    const chunks = chunkPages([page(body)], { siteUrl: SITE });
    const anchors = chunks.map((c: { url: string }) => c.url.split('#')[1]);
    expect(anchors).toEqual(['options', 'options-1', 'options-2']);
  });

  it('skips auto-generated fig-spec sections', () => {
    const body = '## Usage\nreal content\n\n## generate-fig-spec\nnoise noise noise';
    const chunks = chunkPages([page(body)], { siteUrl: SITE });
    const headings = chunks.map((c: { heading: string }) => c.heading);
    expect(headings).toContain('Usage');
    expect(headings).not.toContain('generate-fig-spec');
  });

  it('never emits a chunk larger than the cap', () => {
    const body = `## Big\n\`\`\`json\n${'a'.repeat(MAX_CHARS * 4)}\n\`\`\``;
    const chunks = chunkPages([page(body)], { siteUrl: SITE });
    for (const c of chunks) expect(c.text.length).toBeLessThanOrEqual(MAX_CHARS);
  });

  it('builds ids and deep-link urls from the page path and anchor', () => {
    const chunks = chunkPages([page('## Install\nrun npm i', { url: `${SITE}/developers/quick-start` })], { siteUrl: SITE });
    const hit = chunks.find((c: { heading: string }) => c.heading === 'Install');
    expect(hit.id).toBe('developers/quick-start#install');
    expect(hit.url).toBe(`${SITE}/developers/quick-start#install`);
  });

  it('carries the source tag through to every chunk', () => {
    const chunks = chunkPages([page('## H\nbody', { source: 'confluence' })], { siteUrl: SITE });
    for (const c of chunks) expect(c.source).toBe('confluence');
  });
});
