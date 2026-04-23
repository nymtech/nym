import type { ExplorerNymNodeRow } from './nodeStatus';
import {
  clearNodeStatusExplorerCaches,
  fetchGatewayStatus,
  fetchGatewayStatusIfBonded,
  findExplorerNymNodeByIdentity,
  getNodeStatusBaseUrl,
  isGatewayRole,
  normalizeExplorerUptimePercent,
  parseTotalStakeToNymAmount,
} from './nodeStatus';

describe('getNodeStatusBaseUrl', () => {
  it('uses mainnet base for MAINNET and unknown', () => {
    expect(getNodeStatusBaseUrl('MAINNET')).toBe('https://mainnet-node-status-api.nymtech.cc');
    expect(getNodeStatusBaseUrl(undefined)).toBe('https://mainnet-node-status-api.nymtech.cc');
  });

  it('uses sandbox base for SANDBOX and QA', () => {
    expect(getNodeStatusBaseUrl('SANDBOX')).toBe('https://sandbox-node-status-api.nymte.ch');
    expect(getNodeStatusBaseUrl('QA')).toBe('https://sandbox-node-status-api.nymte.ch');
  });
});

describe('isGatewayRole', () => {
  it('is true only for gateway roles', () => {
    expect(isGatewayRole('entryGateway')).toBe(true);
    expect(isGatewayRole('exitGateway')).toBe(true);
    expect(isGatewayRole('layer1')).toBe(false);
    expect(isGatewayRole(undefined)).toBe(false);
  });
});

describe('parseTotalStakeToNymAmount', () => {
  it('divides base unym by 1e6', () => {
    expect(parseTotalStakeToNymAmount('1000000')).toBe(1);
    expect(parseTotalStakeToNymAmount('2500000')).toBe(2.5);
  });

  it('returns 0 for non-finite values', () => {
    expect(parseTotalStakeToNymAmount('')).toBe(0);
    expect(parseTotalStakeToNymAmount('x')).toBe(0);
  });
});

describe('normalizeExplorerUptimePercent', () => {
  it('maps ratio 0-1 to percent', () => {
    expect(normalizeExplorerUptimePercent(0.98)).toBe(98);
    expect(normalizeExplorerUptimePercent(1)).toBe(100);
    expect(normalizeExplorerUptimePercent(0)).toBe(0);
  });

  it('leaves values already in 0-100 percent scale', () => {
    expect(normalizeExplorerUptimePercent(98)).toBe(98);
    expect(normalizeExplorerUptimePercent(100)).toBe(100);
  });

  it('parses numeric strings', () => {
    expect(normalizeExplorerUptimePercent('0.98')).toBe(98);
    expect(normalizeExplorerUptimePercent('  98  ')).toBe(98);
  });

  it('clamps negatives and caps at 100', () => {
    expect(normalizeExplorerUptimePercent(-1)).toBe(0);
    expect(normalizeExplorerUptimePercent(150)).toBe(100);
  });

  it('returns 0 for non-numeric input', () => {
    expect(normalizeExplorerUptimePercent('')).toBe(0);
    expect(normalizeExplorerUptimePercent('abc')).toBe(0);
    expect(normalizeExplorerUptimePercent(Number.NaN)).toBe(0);
  });
});

function jsonResponse(ok: boolean, status: number, body: unknown): Response {
  return {
    ok,
    status,
    json: async () => body,
  } as Response;
}

describe('fetchGatewayStatusIfBonded', () => {
  const baseUrl = 'https://status.example';
  const identity = 'nym1abc';

  beforeEach(() => {
    global.fetch = jest.fn();
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('returns null on 404 and 400', async () => {
    const fetchMock = global.fetch as jest.MockedFunction<typeof fetch>;
    fetchMock.mockResolvedValueOnce(jsonResponse(false, 404, null));
    await expect(fetchGatewayStatusIfBonded(baseUrl, identity)).resolves.toBeNull();

    fetchMock.mockResolvedValueOnce(jsonResponse(false, 400, null));
    await expect(fetchGatewayStatusIfBonded(baseUrl, identity)).resolves.toBeNull();
  });

  it('throws on other non-OK responses', async () => {
    const fetchMock = global.fetch as jest.MockedFunction<typeof fetch>;
    fetchMock.mockResolvedValueOnce(jsonResponse(false, 500, null));
    await expect(fetchGatewayStatusIfBonded(baseUrl, identity)).rejects.toThrow('Gateway status request failed (500)');
  });

  it('returns null when bonded is false or identity mismatches', async () => {
    const fetchMock = global.fetch as jest.MockedFunction<typeof fetch>;
    fetchMock.mockResolvedValueOnce(
      jsonResponse(true, 200, {
        gateway_identity_key: identity,
        bonded: false,
        performance: 0,
        routing_score: 0,
        last_updated_utc: 't',
      }),
    );
    await expect(fetchGatewayStatusIfBonded(baseUrl, identity)).resolves.toBeNull();

    fetchMock.mockResolvedValueOnce(
      jsonResponse(true, 200, {
        gateway_identity_key: 'other',
        bonded: true,
        performance: 0,
        routing_score: 0,
        last_updated_utc: 't',
      }),
    );
    await expect(fetchGatewayStatusIfBonded(baseUrl, identity)).resolves.toBeNull();
  });

  it('returns payload when bonded and identity matches', async () => {
    const fetchMock = global.fetch as jest.MockedFunction<typeof fetch>;
    const payload = {
      gateway_identity_key: identity,
      bonded: true,
      performance: 88,
      routing_score: 1,
      last_updated_utc: 't',
    };
    fetchMock.mockResolvedValueOnce(jsonResponse(true, 200, payload));
    await expect(fetchGatewayStatusIfBonded(baseUrl, identity)).resolves.toStrictEqual(payload);
    expect(fetchMock).toHaveBeenCalledWith(
      'https://status.example/v2/gateways/nym1abc',
      expect.objectContaining({ cache: 'no-store' }),
    );
  });

  it('strips trailing slash from base URL', async () => {
    const fetchMock = global.fetch as jest.MockedFunction<typeof fetch>;
    fetchMock.mockResolvedValueOnce(
      jsonResponse(true, 200, {
        gateway_identity_key: identity,
        bonded: true,
        performance: 1,
        routing_score: 0,
        last_updated_utc: 't',
      }),
    );
    await fetchGatewayStatusIfBonded('https://status.example/', identity);
    expect(fetchMock).toHaveBeenCalledWith('https://status.example/v2/gateways/nym1abc', expect.any(Object));
  });
});

describe('fetchGatewayStatus', () => {
  beforeEach(() => {
    global.fetch = jest.fn();
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('throws when gateway is not bonded', async () => {
    const fetchMock = global.fetch as jest.MockedFunction<typeof fetch>;
    fetchMock.mockResolvedValueOnce(jsonResponse(false, 404, null));
    await expect(fetchGatewayStatus('https://x', 'id')).rejects.toThrow('Gateway status request failed (404)');
  });
});

describe('findExplorerNymNodeByIdentity', () => {
  const base = 'https://status.example';
  const identity = 'nodeIdentity1';

  const sampleRow = (key: string): ExplorerNymNodeRow => ({
    identity_key: key,
    node_id: 1,
    bonded: true,
    uptime: 0.99,
    total_stake: '1000000',
    original_pledge: 1,
    description: { moniker: 'm', website: '', details: '', security_contact: '' },
  });

  beforeEach(() => {
    clearNodeStatusExplorerCaches();
    global.fetch = jest.fn();
  });

  afterEach(() => {
    jest.restoreAllMocks();
    clearNodeStatusExplorerCaches();
  });

  it('finds row on first page', async () => {
    const fetchMock = global.fetch as jest.MockedFunction<typeof fetch>;
    const row = sampleRow(identity);
    fetchMock.mockResolvedValueOnce(jsonResponse(true, 200, { items: [row], page: 0, size: 200, total: 1 }));
    const result = await findExplorerNymNodeByIdentity('MAINNET', base, identity);
    expect(result).toStrictEqual(row);
    expect(fetchMock).toHaveBeenCalledWith(
      'https://status.example/explorer/v3/nym-nodes?page=0&size=200',
      expect.objectContaining({ cache: 'no-store' }),
    );
  });

  it('paginates until match', async () => {
    const fetchMock = global.fetch as jest.MockedFunction<typeof fetch>;
    const other = sampleRow('other');
    const row = sampleRow(identity);
    fetchMock
      .mockResolvedValueOnce(
        jsonResponse(true, 200, {
          items: Array(200).fill(other),
          page: 0,
          size: 200,
          total: 250,
        }),
      )
      .mockResolvedValueOnce(
        jsonResponse(true, 200, {
          items: [row],
          page: 1,
          size: 200,
          total: 250,
        }),
      );
    const result = await findExplorerNymNodeByIdentity('MAINNET', base, identity);
    expect(result).toStrictEqual(row);
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it('throws when explorer returns non-OK', async () => {
    const fetchMock = global.fetch as jest.MockedFunction<typeof fetch>;
    fetchMock.mockResolvedValueOnce(jsonResponse(false, 503, null));
    await expect(findExplorerNymNodeByIdentity('MAINNET', base, identity)).rejects.toThrow(
      'Explorer nym-nodes request failed (503)',
    );
  });

  it('returns null when not found and caches miss', async () => {
    const fetchMock = global.fetch as jest.MockedFunction<typeof fetch>;
    fetchMock.mockResolvedValue(jsonResponse(true, 200, { items: [], page: 0, size: 200, total: 0 }));
    await expect(findExplorerNymNodeByIdentity('MAINNET', base, identity)).resolves.toBeNull();
    await expect(findExplorerNymNodeByIdentity('MAINNET', base, identity)).resolves.toBeNull();
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('returns cached hit without refetching', async () => {
    const fetchMock = global.fetch as jest.MockedFunction<typeof fetch>;
    const row = sampleRow(identity);
    fetchMock.mockResolvedValueOnce(jsonResponse(true, 200, { items: [row], page: 0, size: 200, total: 1 }));
    await findExplorerNymNodeByIdentity('MAINNET', base, identity);
    await findExplorerNymNodeByIdentity('MAINNET', base, identity);
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('deduplicates concurrent scans for the same key', async () => {
    const fetchMock = global.fetch as jest.MockedFunction<typeof fetch>;
    let resolveFetch!: (v: Response) => void;
    const fetchPromise = new Promise<Response>((r) => {
      resolveFetch = r;
    });
    fetchMock.mockReturnValueOnce(fetchPromise);

    const p1 = findExplorerNymNodeByIdentity('MAINNET', base, identity);
    const p2 = findExplorerNymNodeByIdentity('MAINNET', base, identity);
    resolveFetch(jsonResponse(true, 200, { items: [], page: 0, size: 200, total: 0 }));
    await expect(Promise.all([p1, p2])).resolves.toStrictEqual([null, null]);
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('does not share cache between networks for the same identity', async () => {
    const fetchMock = global.fetch as jest.MockedFunction<typeof fetch>;
    const rowMainnet = sampleRow(identity);
    const rowSandbox = { ...sampleRow(identity), node_id: 99 };
    fetchMock
      .mockResolvedValueOnce(jsonResponse(true, 200, { items: [rowMainnet], page: 0, size: 200, total: 1 }))
      .mockResolvedValueOnce(jsonResponse(true, 200, { items: [rowSandbox], page: 0, size: 200, total: 1 }));
    await expect(findExplorerNymNodeByIdentity('MAINNET', base, identity)).resolves.toStrictEqual(rowMainnet);
    await expect(findExplorerNymNodeByIdentity('SANDBOX', base, identity)).resolves.toStrictEqual(rowSandbox);
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });
});
