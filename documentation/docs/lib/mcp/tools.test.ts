import { describe, it, expect } from 'vitest';
import { createTools } from './tools';
import type { ToolDeps } from './tools';
import type { DocIndex, EmbeddedChunk } from '../retrieval/types';

function chunk(id: string, vector: number[], text: string): EmbeddedChunk {
  return { id, source: 'nym-docs', title: 'Quick Start', heading: id, url: `https://nym.com/docs/${id}`, text, tokensEst: 1, vector };
}

const index: DocIndex = {
  schema: 1,
  generated: null,
  embedding: { provider: 'mock', model: 'mock', dim: 2 },
  chunks: [chunk('install', [1, 0], 'run npm i @nymproject/sdk'), chunk('other', [0, 1], 'unrelated')],
};

const fakeNym = {
  getNetworkSummary: async () => ({
    total_nodes: 802,
    mixnodes: { bonded: { count: 59, self_described: 802, last_updated_utc: 'x' }, historical: { count: 1, last_updated_utc: 'x' } },
    gateways: { bonded: { count: 558, entry: 80, exit: 100, last_updated_utc: 'x' }, historical: { count: 1, last_updated_utc: 'x' } },
  }),
  getCirculatingSupply: async () => ({
    total_supply: { denom: 'unym', amount: '1000000000000000' },
    mixmining_reserve: { denom: 'unym', amount: '0' },
    vesting_tokens: { denom: 'unym', amount: '0' },
    circulating_supply: { denom: 'unym', amount: '839301194314151' },
  }),
  getChainStatus: async () => ({ connected_nyxd: 'https://rpc.nymtech.net/', status: 'ok' }),
  listGateways: async (page: number, size: number) => ({
    page,
    size,
    total: 558,
    items: [{ gateway_identity_key: '131LU7', routing_score: 0, config_score: 0, performance: 97 }],
  }),
  getGateway: async (identity: string) => ({ gateway_identity_key: identity, bonded: true, performance: 97, routing_score: 0, config_score: 0 }),
};

const deps: ToolDeps = { index, embedQuery: async () => [1, 0], nym: fakeNym };
const tools = createTools(deps);
const tool = (name: string) => tools.find((t) => t.name === name)!;
const runText = async (name: string, args: Record<string, any> = {}) => (await tool(name).handler(args)).content[0].text;

describe('tool registry', () => {
  it('exposes the expected tools, each with an input schema', () => {
    const names = tools.map((t) => t.name).sort();
    expect(names).toEqual(
      ['chain_status', 'circulating_supply', 'get_gateway', 'get_section', 'list_gateways', 'network_summary', 'search_docs', 'validate_sdk_config'].sort(),
    );
    for (const t of tools) expect(t.inputSchema).toHaveProperty('type', 'object');
  });

  it('marks required args', () => {
    expect((tool('search_docs').inputSchema as any).required).toContain('query');
    expect((tool('get_gateway').inputSchema as any).required).toContain('identity');
  });
});

describe('retrieval tools', () => {
  it('search_docs embeds the query and returns the top hit with its url', async () => {
    const out = await runText('search_docs', { query: 'how to install', topK: 1 });
    expect(out).toContain('https://nym.com/docs/install');
    expect(out).toContain('run npm i');
    expect(out).not.toContain('unrelated'); // orthogonal chunk ranked out at topK=1
  });

  it('get_section resolves a section by id', async () => {
    expect(await runText('get_section', { ref: 'install' })).toContain('run npm i');
  });

  it('get_section reports a miss cleanly', async () => {
    expect(await runText('get_section', { ref: 'nope' })).toContain('No section');
  });
});

describe('live tools', () => {
  it('network_summary formats the gateway split', async () => {
    expect(await runText('network_summary')).toBe('802 nodes total. Gateways: 558 bonded (80 entry, 100 exit). Mixnodes: 59 bonded.');
  });

  it('circulating_supply converts unym to NYM', async () => {
    expect(await runText('circulating_supply')).toContain('839,301,194.314 NYM'); // toLocaleString rounds to 3dp
  });

  it('list_gateways passes pagination and formats rows', async () => {
    const out = await runText('list_gateways', { page: 1, size: 5 });
    expect(out).toContain('558 gateways total. Page 1');
    expect(out).toContain('131LU7  perf=97');
  });

  it('get_gateway reports health for the requested identity', async () => {
    expect(await runText('get_gateway', { identity: 'ABC' })).toContain('Gateway ABC: bonded=true');
  });

  it('returns an isError result instead of throwing on upstream failure', async () => {
    const failing = createTools({ ...deps, nym: { ...fakeNym, getGateway: async () => { throw new Error('502 upstream'); } } });
    const res = await failing.find((t) => t.name === 'get_gateway')!.handler({ identity: 'X' });
    expect(res.isError).toBe(true);
    expect(res.content[0].text).toContain('502 upstream');
  });
});
