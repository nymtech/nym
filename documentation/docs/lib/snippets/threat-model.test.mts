import { describe, it, expect } from 'vitest';
import { buildThreatModel } from './threat-model.mts';
import { must } from './testing.mts';
import type { Scenario, Vector } from '../privacy-model/types.ts';

describe('threat-model adapter', async () => {
  const { records, errors, warnings } = await buildThreatModel();
  const byId = new Map(records.map((r) => [r.id, r]));
  const get = (id: string) => must(byId.get(id), id);

  it('writes one model record for each entity, with no integrity error', () => {
    expect(errors).toEqual([]);
    expect(records.every((r) => r.kind === 'model')).toBe(true);
    for (const id of ['actor/L2', 'actor/L3L', 'actor/L3G', 'vector/V1', 'property/P1', 'scenario/mixnet', 'example/wallet/actor/L1', 'example/wallet/scenario/mixnet-ipr']) {
      expect(byId.has(`nym/threat-model/${id}`)).toBe(true);
    }
  });

  it('keeps the source order in meta.order, for a table of a whole kind', () => {
    const kind = (prefix: string) => records
      .filter((r) => r.id.startsWith(prefix) && !r.id.slice(prefix.length).includes('/'))
      .sort((a, b) => Number(a.meta.order) - Number(b.meta.order))
      .map((r) => r.id.slice(prefix.length));
    expect(kind('nym/threat-model/actor/')).toEqual(['L2', 'L3L', 'L3G']);
    const scenarios = kind('nym/threat-model/scenario/');
    expect(scenarios[0]).toBe('unprotected');
    expect(scenarios.at(-1)).toBe('end-to-end');
  });

  it('writes a scenario variant as a drawing of its base, with no claim of its own', () => {
    const v = get('nym/threat-model/variant/service-provider');
    expect(v.refs).toContainEqual({ id: 'nym/threat-model/scenario/end-to-end', rel: 'specialises' });
    const node = (v.meta.data as Scenario).topology?.nodes.find((n) => n.id === 'p');
    expect(node?.kind).toBe('service-provider');
    expect(records.some((r) => r.id.startsWith('nym/threat-model/claim/service-provider'))).toBe(false);
  });

  it('takes the edges from the data: observes, assessed-against, specialises', () => {
    const v2 = get('nym/threat-model/vector/V2');
    for (const actor of (v2.meta.data as Vector).observableFrom) {
      expect(get(`nym/threat-model/actor/${actor}`).refs).toContainEqual({ id: 'nym/threat-model/vector/V2', rel: 'observes' });
    }
    expect(get('nym/threat-model/scenario/mixnet').refs).toContainEqual({ id: 'nym/threat-model/actor/L2', rel: 'assessed-against' });
    expect(get('nym/threat-model/example/messaging/scenario/mixnet').refs).toContainEqual({ id: 'nym/threat-model/scenario/mixnet', rel: 'specialises' });
  });

  it('states each route hop by hop, and what the destination sees', () => {
    const { body } = get('nym/threat-model/scenario/mixnet');
    expect(body).toMatch(/Routes, hop by hop:\n\n- .+: client -> .+ -> destination/);
    expect(body).toContain('Against L3G:');
  });

  it('puts end-to-end / L2 on the exception list, not in the errors', () => {
    expect(warnings).toContainEqual(expect.stringContaining('scenario end-to-end: no L2 assessment, on the exception list'));
  });

  it('writes one claim for each matrix cell, with the verdict and its condition', () => {
    const claim = get('nym/threat-model/claim/unprotected/P1/L2');
    expect(claim.body).toBe('Unprotected: P1 against L2 is "no", identity and contents arrive together.'.replace('Unprotected', must(claim.body.split(':')[0], 'the scenario title of the claim')));
    expect(claim.meta.status).toBe('approved');
    expect(claim.refs).toContainEqual({ id: 'nym/threat-model/scenario/unprotected', rel: 'relates-to' });
    const cells = records.filter((r) => r.id.includes('/claim/')).length;
    const matrixCells = records.filter((r) => /\/scenario\//.test(r.id)).reduce<number>((n, r) => n + Object.keys((r.meta.data as Scenario).matrix ?? {}).length, 0);
    expect(cells).toBe(matrixCells);
  });
});
