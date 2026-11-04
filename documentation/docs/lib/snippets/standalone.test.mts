import { describe, it, expect } from 'vitest';
import { standaloneFaults } from './standalone.mts';

const kinds = (raw: string) => standaloneFaults(raw).map((f: { kind: string; text: string }) => [f.kind, f.text]);

describe('standalone lint', () => {
  it('finds a body that refers to text around it', () => {
    expect(kinds('## Keys [#keys]\n\nAs described above, the key rotates (see below).\n')).toEqual([['reference', 'As described above']]);
    expect(kinds('## A [#a]\n\nThe table below lists them.\n')).toEqual([['reference', 'table below']]);
    expect(kinds('This document covers the struct.\n')).toEqual([['reference', 'This document']]);
  });

  it('does not flag above or below as plain words, in code, or in a fence', () => {
    expect(kinds('## A [#a]\n\nLatency below 10 ms is above the threshold. Use `--above.`\n\n```sh\nsee below.\n```\n')).toEqual([]);
  });

  it('finds a template title with no subject, and gives the line in the file', () => {
    const [fault] = standaloneFaults('---\nrefs: []\n---\n\n## Code [#code]\n\nbody\n');
    expect(fault).toEqual({ line: 5, kind: 'title', text: 'Code' });
  });

  it('finds plurals, "everything below", and "this section"', () => {
    expect(kinds('See the sections above for the keys.\n')).toEqual([['reference', 'sections above']]);
    expect(kinds('Everything below assumes a node.\n')).toEqual([['reference', 'Everything below']]);
    expect(kinds('This section covers the chain.\n')).toEqual([['reference', 'This section']]);
  });

  it('finds "below is", "below are", and "in later sections"', () => {
    expect(kinds('Below is a simple way to open a pull request.\n')).toEqual([['reference', 'Below is']]);
    expect(kinds('Install it first. Below are steps to set up the environment.\n')).toEqual([['reference', '. Below are']]);
    expect(kinds('In later sections, you set up a secure websocket.\n')).toEqual([['reference', 'In later sections']]);
  });

  it('does not flag "below" as a comparison inside a sentence', () => {
    expect(kinds('Delays below are clamped, and a rate below is ignored.\n')).toEqual([]);
  });

  it('does not flag a reference to a block in the same snippet', () => {
    expect(kinds('The options are listed below:\n\n| a | b |\n|---|---|\n')).toEqual([]);
    expect(kinds('```sh\nx\n```\n\nThe example above sets it.\n')).toEqual([]);
    expect(kinds('The following table lists the supply.\n\n<Output name="token-table" />\n')).toEqual([]);
  });
});

