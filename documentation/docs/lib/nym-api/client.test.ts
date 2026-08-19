import { describe, it, expect, vi, afterEach } from 'vitest';
import {
  getCirculatingSupply,
  getNetworkSummary,
  listGateways,
  getGateway,
  getJson,
  unymToNym,
  NymApiError,
  NYM_API,
  NODE_STATUS_API,
} from './client';

// Fixtures recorded from the live APIs (shapes verified 2026-08-03), so these
// hermetic tests assert the real response structure without touching the network.
const SUPPLY = {
  total_supply: { denom: 'unym', amount: '1000000000000000' },
  mixmining_reserve: { denom: 'unym', amount: '160698805685849' },
  vesting_tokens: { denom: 'unym', amount: '0' },
  circulating_supply: { denom: 'unym', amount: '839301194314151' },
};
const SUMMARY = {
  total_nodes: 802,
  mixnodes: { bonded: { count: 59, self_described: 802, last_updated_utc: 'x' }, historical: { count: 307, last_updated_utc: 'x' } },
  gateways: { bonded: { count: 558, entry: 80, exit: 100, last_updated_utc: 'x' }, historical: { count: 907, last_updated_utc: 'x' } },
};
const SKINNY = { page: 0, size: 3, total: 558, items: [{ gateway_identity_key: '131LU7', routing_score: 0, config_score: 0, performance: 97 }] };
const GATEWAY = { gateway_identity_key: '131LU7', bonded: true, performance: 97, routing_score: 0, config_score: 0 };

function mockFetch(body: unknown, { ok = true, status = 200 } = {}) {
  const fn = vi.fn(async () => ({
    ok,
    status,
    json: async () => body,
    text: async () => (typeof body === 'string' ? body : JSON.stringify(body)),
  }));
  vi.stubGlobal('fetch', fn);
  return fn;
}

afterEach(() => vi.unstubAllGlobals());

describe('getJson', () => {
  it('throws NymApiError with status + url on non-2xx', async () => {
    mockFetch('not found', { ok: false, status: 404 });
    await expect(getJson('https://x/y')).rejects.toBeInstanceOf(NymApiError);
    await expect(getJson('https://x/y')).rejects.toMatchObject({ status: 404, url: 'https://x/y' });
  });

  it('passes an abort signal (timeout) to fetch', async () => {
    const fn = mockFetch({});
    await getJson('https://x/y');
    expect(fn.mock.calls[0][1]).toHaveProperty('signal');
  });
});

describe('token economics', () => {
  it('fetches circulating supply from the NymAPI base', async () => {
    const fn = mockFetch(SUPPLY);
    const out = await getCirculatingSupply();
    expect(fn.mock.calls[0][0]).toBe(`${NYM_API}/v1/circulating-supply`);
    expect(out.circulating_supply.amount).toBe('839301194314151');
  });

  it('converts unym to NYM', () => {
    expect(unymToNym({ denom: 'unym', amount: '1000000' })).toBe(1);
    expect(unymToNym(SUPPLY.circulating_supply)).toBeCloseTo(839301194.314151);
  });
});

describe('network summary + gateways (Node Status API)', () => {
  it('fetches the network summary from the node-status base', async () => {
    const fn = mockFetch(SUMMARY);
    const out = await getNetworkSummary();
    expect(fn.mock.calls[0][0]).toBe(`${NODE_STATUS_API}/v2/summary`);
    expect(out.gateways.bonded.entry).toBe(80);
    expect(out.total_nodes).toBe(802);
  });

  it('lists gateways with page + size in the query', async () => {
    const fn = mockFetch(SKINNY);
    const out = await listGateways(2, 50);
    expect(fn.mock.calls[0][0]).toBe(`${NODE_STATUS_API}/v2/gateways/skinny?page=2&size=50`);
    expect(out.total).toBe(558);
    expect(out.items[0].performance).toBe(97);
  });

  it('fetches a single gateway by url-encoded identity', async () => {
    const fn = mockFetch(GATEWAY);
    const out = await getGateway('131/LU7');
    expect(fn.mock.calls[0][0]).toBe(`${NODE_STATUS_API}/v2/gateways/131%2FLU7`);
    expect(out.gateway_identity_key).toBe('131LU7');
  });
});
