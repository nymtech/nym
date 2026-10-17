import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { requirementBlocks, levelOf, buildRequirements } from './openspec.mts';
import { applyEdges } from './edges.mts';
import { coverage } from './coverage.mts';

const SPEC = `# x Specification\n\n## Requirements\n\n### Requirement: A thing SHALL work\n\nIt MUST be fast.\n\n#### Scenario: Fast\n- **WHEN** x\n- **THEN** y\n\n### Requirement: A second thing\n\nIt SHOULD be small.\n\n## Technical Notes\n\nnot a requirement\n`;

function repoWith(spec: string, enforced: string[] = []) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'openspec-'));
  fs.mkdirSync(path.join(root, 'openspec/specs/x'), { recursive: true });
  fs.writeFileSync(path.join(root, 'openspec/specs/x/spec.md'), spec);
  return { root, repo: 'nym', manifest: { enforced } };
}

describe('openspec adapter', () => {
  it('cuts one block for each requirement, with its scenarios, and stops at the next ## heading', () => {
    const blocks = requirementBlocks(SPEC);
    expect(blocks.map((b: { title: string }) => b.title)).toEqual(['A thing SHALL work', 'A second thing']);
    expect(blocks[0].body).toContain('#### Scenario: Fast');
    expect(blocks[1].body).not.toContain('not a requirement');
  });

  it('takes the strongest RFC 2119 keyword as the level', () => {
    expect(levelOf('It SHALL and it MUST')).toBe('MUST');
    expect(levelOf('It SHOULD')).toBe('SHOULD');
    expect(levelOf('no keyword')).toBe(null);
  });

  it('writes a statement record with level and status, enforced from the manifest', () => {
    const { records, errors } = buildRequirements(repoWith(SPEC, ['nym/openspec/x/a-thing-shall-work']));
    expect(errors).toEqual([]);
    expect(records.map((r: { id: string; meta: object }) => [r.id, r.meta])).toEqual([
      ['nym/openspec/x/a-second-thing', { spec: 'x', level: 'SHOULD', status: 'approved' }],
      ['nym/openspec/x/a-thing-shall-work', { spec: 'x', level: 'MUST', status: 'enforced' }],
    ]);
  });

  it('fails on two requirements of one spec with one slug', () => {
    const { errors } = buildRequirements(repoWith(`${SPEC}\n## Requirements\n\n### Requirement: A thing SHALL work!\n\nagain\n`));
    expect(errors[0]).toContain('has the slug a-thing-shall-work');
  });
});

describe('edges.json and coverage', () => {
  const req = { id: 'nym/openspec/x/r', kind: 'prose', refs: [] as object[], meta: { spec: 'x' } };
  const code = { id: 'nym/src/a.rs#f', kind: 'code', refs: [] as object[] };
  const prop = { id: 'nym/threat-model/property/P1', kind: 'model', refs: [] as object[] };

  it('merges authored edges into refs and rejects a bad relation, a missing id, and cites', () => {
    const records = [structuredClone(req), structuredClone(code), structuredClone(prop)];
    const errors = applyEdges([
      { from: code.id, to: req.id, rel: 'realises', at: 'e[0]' },
      { from: req.id, to: prop.id, rel: 'realises', at: 'e[1]' },
      { from: req.id, to: prop.id, rel: 'dependsOn', at: 'e[2]' },
      { from: req.id, to: 'nym/gone', rel: 'realises', at: 'e[3]' },
      { from: req.id, to: prop.id, rel: 'cites', at: 'e[4]' },
    ], records);
    expect(errors).toHaveLength(3);
    expect(records[0].refs).toEqual([{ id: prop.id, rel: 'realises' }]);
    const [line] = coverage({ requirement: [records[0]], prose: [], code: [records[1]], model: [records[2]] });
    expect(line).toEqual({ id: req.id, spec: 'x', threatModel: [prop.id], realisedBy: [code.id] });
  });
});
