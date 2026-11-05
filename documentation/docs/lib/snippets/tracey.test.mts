import { describe, it, expect } from 'vitest';
import { codeMarkers, itemFor, specMarkers } from './tracey.mts';

describe('tracey markers', () => {
  it('reads a spec marker and the requirement block it sits in', () => {
    const spec = '## ADDED Requirements\n\n### Requirement: Each payload fills its packet\n\nr[sphinx.payload-fills]\nThe client SHALL pad.\n';
    expect(specMarkers(spec, 'nym', 'sphinx-packet-format')).toEqual([
      { traceyId: 'sphinx.payload-fills', version: null, requirement: 'nym/openspec/sphinx-packet-format/each-payload-fills-its-packet', line: 5 },
    ]);
  });

  it('reads a versioned spec marker in a blockquote', () => {
    const [marker] = specMarkers('### Requirement: Login\n\n> r[auth.login+3]\n', 'nym', 'auth');
    expect(marker?.version).toBe(3);
  });

  it('gives stacked markers to the item below all of them', () => {
    const text = 'mod tests {\n    // r[verify a.one]\n    // r[verify b.two]\n    #[test]\n    fn t() {}\n}\n';
    const list = [
      { qualifiedName: 'tests', aliases: [], startLine: 1, endLine: 6 },
      { qualifiedName: 'tests::t', aliases: [], startLine: 4, endLine: 5 },
    ];
    expect(itemFor(list, text.split('\n'), 2)?.qualifiedName).toBe('tests::t');
    expect(itemFor(list, text.split('\n'), 3)?.qualifiedName).toBe('tests::t');
  });

  it('reads code markers, with impl as the default verb', () => {
    const code = '// r[impl delay.named-settings]\nfn a() {}\n    // r[verify sphinx.size-classes+2]\n// r[sphinx.route]\n// see r[x] in prose\n';
    expect(codeMarkers(code).map((m) => [m.verb, m.traceyId, m.version, m.line])).toEqual([
      ['impl', 'delay.named-settings', null, 1],
      ['verify', 'sphinx.size-classes', 2, 3],
      ['impl', 'sphinx.route', null, 4],
    ]);
  });
});
