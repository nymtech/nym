import { describe, it, expect, beforeAll } from 'vitest';
// @ts-expect-error - plain ESM JS modules, no type declarations
import { loadProjections, loadDocValues } from './projections.mjs';
// @ts-expect-error - plain ESM JS module
import { stripMdx } from './mdx.mjs';

// These guard the defect the MCP trial surfaced: the threat-model pages are two
// sentences of prose plus a component, and the component held every definition.
// The chunker dropped the tag, so the canonical actors and vectors were absent
// from the index while every other page referred to them by name.

let expand: (t: string, a: string, c?: Record<string, unknown>) => string | null;

beforeAll(async () => {
  expand = await loadProjections();
});

describe('component projections', () => {
  it('projects every actor, not just the ones prose mentions', () => {
    const out = expand('ActorsReference', '') ?? '';
    for (const id of ['L2', 'L3L', 'L3G']) expect(out).toContain(id);
    expect(out).toContain('The destination');
    expect(out).toContain('Global network observer');
  });

  it('projects vectors with their countermeasures and layer tags', () => {
    const out = expand('VectorsReference', '') ?? '';
    for (const id of ['V1', 'V2', 'V3']) expect(out).toContain(id);
    expect(out).toContain('transport layer');
  });

  it('projects both unlinkability properties with definitions', () => {
    const out = expand('PropertiesReference', '') ?? '';
    expect(out).toContain('P1');
    expect(out).toContain('P2');
    expect(out).toMatch(/unlinkability/i);
  });

  it('returns null for a component with no text to project', () => {
    // Animated diagrams carry no prose. Dropping them is correct; inventing a
    // projection would put words in the index that appear on no page.
    expect(expand('SomeAnimatedThing', '')).toBeNull();
  });

  it('projects a page scenario once, not once per facet component', () => {
    const ctx: Record<string, unknown> = { scenarioId: 'end-to-end' };
    const first = expand('GenericScenarioView', '', ctx);
    const second = expand('MetadataPanel', '', ctx);
    expect(first).toContain('End to end');
    expect(second).toBeNull();
  });
});

describe('stripMdx with projections', () => {
  const page = [
    "import { ActorsReference } from 'components/threat-model/ThreatModelReference'",
    '',
    '# Threat actors',
    '',
    'Some preamble.',
    '',
    '<ActorsReference />',
  ].join('\n');

  it('replaces a component with its projected content', () => {
    const out = stripMdx(page, { expand });
    expect(out).toContain('L3G');
    expect(out).toContain('Some preamble.');
  });

  it('still drops the component when no expander is supplied', () => {
    expect(stripMdx(page)).not.toContain('L3G');
  });

  it('drops page wiring that was reaching the index as prose', () => {
    const src = "export const scenario = requireGenericScenario('mixnet')\n\nReal text.";
    const out = stripMdx(src, { expand });
    expect(out).not.toContain('requireGenericScenario');
    expect(out).toContain('Real text.');
  });

  it('substitutes an interpolated constant instead of emitting its source', async () => {
    // `{RUST_MSRV}` reached readers verbatim, so the docs stated a version
    // requirement that could not be read. It is inline, not on its own line.
    const values = await loadDocValues();
    const out = stripMdx('**Minimum Rust version:** {RUST_MSRV}+', { expand, values });
    expect(out).not.toContain('RUST_MSRV');
    expect(out).toMatch(/Minimum Rust version:\*\* \d+\.\d+\+/);
  });

  it('leaves an unknown interpolation alone rather than inventing a value', () => {
    const out = stripMdx('Version {NOT_A_REAL_CONSTANT} here.', { expand, values: { RUST_MSRV: '1.87' } });
    expect(out).toContain('{NOT_A_REAL_CONSTANT}');
  });

  it('drops a bare unresolved expression line, which renders as nothing', () => {
    const out = stripMdx('Before\n\n{someExpression}\n\nAfter', { expand });
    expect(out).not.toContain('someExpression');
    expect(out).toContain('Before');
  });

  it('leaves braces and tags inside fenced code alone', () => {
    const src = ['```js', 'const x = { a: 1 };', '<Foo />', '```'].join('\n');
    expect(stripMdx(src, { expand })).toBe(src);
  });

  it('picks the scenario up from the page that declares it', () => {
    const src = [
      "export const scenario = requireGenericScenario('end-to-end')",
      '',
      '# End to end',
      '',
      '<GenericScenarioView scenario={scenario} />',
    ].join('\n');
    const out = stripMdx(src, { expand });
    expect(out).toContain('both ends run Nym');
  });
});

describe('projected anchors match the rendered page', () => {
  // ThreatModelReference renders id="actor-L2", id="vector-V1", id="prop-P1",
  // and six pages deep-link to those. A projected heading slugified from its own
  // text would produce an anchor that exists only in the index, so every citation
  // would land on the top of the page instead of the section.
  it('carries the component ids as explicit heading anchors', () => {
    expect(expand('ActorsReference', '')).toContain('{#actor-L2}');
    expect(expand('VectorsReference', '')).toContain('{#vector-V1}');
    expect(expand('PropertiesReference', '')).toContain('{#prop-P1}');
  });
});

describe('MDX comments', () => {
  // A multi-line `{/* ... */}` used to reach the index intact. The swizzle page
  // carried a note addressed to editors ("MAINTENANCE NOTE, delete when it no
  // longer applies") and served it to readers as documentation; a trial agent
  // reported it as a page shipped unfinished. Single-line comments were already
  // handled, which is exactly why this went unnoticed.
  it('drops a multi-line comment entirely', () => {
    const src = ['Before.', '', '{/*', '  A note to editors.', '  Step 1: do a thing.', '*/}', '', 'After.'].join('\n');
    const out = stripMdx(src, { expand });
    expect(out).not.toMatch(/note to editors|Step 1/);
    expect(out).toContain('Before.');
    expect(out).toContain('After.');
  });

  it('still drops a single-line comment', () => {
    expect(stripMdx('A\n\n{/* hidden */}\n\nB', { expand })).not.toContain('hidden');
  });

  it('does not swallow the rest of the page after a closing comment', () => {
    const src = ['{/*', 'note', '*/}', '', '## Real heading', '', 'Real text.'].join('\n');
    const out = stripMdx(src, { expand });
    expect(out).toContain('Real heading');
    expect(out).toContain('Real text.');
  });
});
