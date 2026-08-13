import { describe, it, expect } from 'vitest';
import { docsHref, linkifyCitations, citedNumbers, citedSources } from './citations';
import type { Citation } from './context';

const cite = (n: number, url: string): Citation => ({ n, url, title: `T${n}`, heading: `H${n}` });

const CITATIONS = [
  cite(1, 'https://nym.com/docs/network/threat-model/actors#l2'),
  cite(2, 'https://nym.com/docs/developers/clients'),
  cite(3, 'https://nym.com/docs/network/traffic'),
];

describe('docsHref', () => {
  it('strips the origin and the /docs basePath, which next/link re-adds', () => {
    expect(docsHref('https://nym.com/docs/developers/clients')).toBe('/developers/clients');
  });

  it('keeps the anchor a citation depends on', () => {
    expect(docsHref('https://nym.com/docs/network/actors#l2')).toBe('/network/actors#l2');
  });

  it('maps the docs root to /, rather than an empty href', () => {
    expect(docsHref('https://nym.com/docs')).toBe('/');
  });

  it('does not mangle a path that merely starts with the letters docs', () => {
    expect(docsHref('https://nym.com/docsomething')).toBe('/docsomething');
  });
});

describe('citedNumbers', () => {
  it('collects the markers an answer uses', () => {
    expect(citedNumbers('Mixing happens at each hop [1], and again later [3].')).toEqual(new Set([1, 3]));
  });

  it('ignores array indexing inside a fenced code block', () => {
    const text = 'As shown [2]:\n\n```rust\nlet x = arr[0];\nlet y = arr[9];\n```\n';
    expect(citedNumbers(text)).toEqual(new Set([2]));
  });

  it('ignores indexing inside an inline code span', () => {
    expect(citedNumbers('Read `parts[0]` first [1].')).toEqual(new Set([1]));
  });

  it('ignores a marker that is already a markdown link', () => {
    expect(citedNumbers('See [1](/somewhere) for more.')).toEqual(new Set());
  });

  it('returns nothing for an answer that cites nothing', () => {
    // The refusal path: this is what stops sources appearing under "not covered".
    expect(citedNumbers('That is not covered in the documentation.')).toEqual(new Set());
  });
});

describe('citedSources', () => {
  it('drops retrieved sections the answer did not use', () => {
    expect(citedSources('Only this one [2].', CITATIONS)).toEqual([CITATIONS[1]]);
  });

  it('keeps the original numbering so inline markers still match the list', () => {
    // Compacting to 1..n here would leave the answer's [3] pointing at row 2.
    expect(citedSources('First [1], then [3].', CITATIONS).map((c) => c.n)).toEqual([1, 3]);
  });

  it('returns nothing when the model declines to use the context', () => {
    expect(citedSources('The docs do not cover that.', CITATIONS)).toEqual([]);
  });

  it('ignores a marker with no matching citation', () => {
    expect(citedSources('Beyond the list [9].', CITATIONS)).toEqual([]);
  });
});

describe('linkifyCitations', () => {
  it('rewrites a marker into a link to the cited section', () => {
    expect(linkifyCitations('See [1].', CITATIONS)).toBe('See [1](/network/threat-model/actors#l2).');
  });

  it('resolves positionally, so it needs the full list rather than a filtered one', () => {
    // The filtered list is for display only. Passing it here would make [3]
    // resolve against the wrong entry.
    const filtered = citedSources('Only [3].', CITATIONS);
    expect(linkifyCitations('Only [3].', filtered)).not.toContain('/network/traffic');
    expect(linkifyCitations('Only [3].', CITATIONS)).toContain('/network/traffic');
  });

  it('leaves code untouched', () => {
    const text = 'Use `arr[1]` here.';
    expect(linkifyCitations(text, CITATIONS)).toBe(text);
  });

  it('leaves an existing markdown link alone', () => {
    const text = 'See [1](/already) for more.';
    expect(linkifyCitations(text, CITATIONS)).toBe(text);
  });

  it('leaves a marker with no matching citation as plain text', () => {
    expect(linkifyCitations('Beyond the list [9].', CITATIONS)).toBe('Beyond the list [9].');
  });
});
