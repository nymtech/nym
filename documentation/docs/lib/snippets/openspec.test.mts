import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { requirementBlocks, levelOf, buildRequirements } from './openspec.mts';
import { applyEdges } from './edges.mts';
import { coverage } from './coverage.mts';
import { fixtureRecord, fixtureRepo, must } from './testing.mts';

const SPEC = `# x Specification\n\n## Requirements\n\n### Requirement: A thing SHALL work\n\nIt MUST be fast.\n\n#### Scenario: Fast\n- **WHEN** x\n- **THEN** y\n\n### Requirement: A second thing\n\nIt SHOULD be small.\n\n## Technical Notes\n\nnot a requirement\n`;

function repoWith(spec: string, enforced: string[] = []) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'openspec-'));
  fs.mkdirSync(path.join(root, 'openspec/specs/x'), { recursive: true });
  fs.writeFileSync(path.join(root, 'openspec/specs/x/spec.md'), spec);
  return fixtureRepo(root, { enforced });
}

describe('openspec adapter', () => {
  it('cuts one block for each requirement, with its scenarios, and stops at the next ## heading', () => {
    const blocks = requirementBlocks(SPEC);
    expect(blocks.map((b) => b.title)).toEqual(['A thing SHALL work', 'A second thing']);
    expect(must(blocks[0], 'block 0').body).toContain('#### Scenario: Fast');
    expect(must(blocks[1], 'block 1').body).not.toContain('not a requirement');
  });

  it('takes the strongest RFC 2119 keyword as the level', () => {
    expect(levelOf('It SHALL and it MUST')).toBe('MUST');
    expect(levelOf('It SHOULD')).toBe('SHOULD');
    expect(levelOf('no keyword')).toBe(null);
  });

  it('writes a statement record with level and status, enforced from the manifest', () => {
    const { records, errors } = buildRequirements(repoWith(SPEC, ['nym/openspec/x/a-thing-shall-work']));
    expect(errors).toEqual([]);
    expect(records.map((r) => [r.id, r.meta])).toEqual([
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
  const req = fixtureRecord({ id: 'nym/openspec/x/r', kind: 'prose', meta: { spec: 'x' } });
  const code = fixtureRecord({ id: 'nym/src/a.rs#f', kind: 'code' });
  const prop = fixtureRecord({ id: 'nym/threat-model/property/P1', kind: 'model' });

  it('merges authored edges into refs and rejects a bad relation, a missing id, and cites', () => {
    const [requirement, codeRecord, model] = [structuredClone(req), structuredClone(code), structuredClone(prop)];
    const records = [requirement, codeRecord, model];
    const errors = applyEdges([
      { from: code.id, to: req.id, rel: 'realises', at: 'e[0]' },
      { from: req.id, to: prop.id, rel: 'realises', at: 'e[1]' },
      { from: req.id, to: prop.id, rel: 'dependsOn', at: 'e[2]' },
      { from: req.id, to: 'nym/gone', rel: 'realises', at: 'e[3]' },
      { from: req.id, to: prop.id, rel: 'cites', at: 'e[4]' },
    ], records);
    expect(errors).toHaveLength(3);
    expect(requirement.refs).toEqual([{ id: prop.id, rel: 'realises' }]);
    const [line] = coverage({ requirement: [requirement], prose: [], code: [codeRecord], model: [model] });
    expect(line).toEqual({ id: req.id, spec: 'x', threatModel: [prop.id], realisedBy: [code.id] });
  });
});
