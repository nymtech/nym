// Live Nym network API client. Backs the MCP server's live tools: the value an
// agent cannot get from static docs (is this gateway healthy right now? what is
// circulating supply? how big is the active set?).
//
// Two upstreams, different base paths (verified against the live OpenAPI specs;
// the docs pages currently show stale example paths):
//   NymAPI          https://validator.nymtech.net/api        -> /v1/...
//   Node Status API https://mainnet-node-status-api.nymtech.cc -> /v2/...
//
// Pure fetch, no deps. Unit-tested hermetically (client.test.ts, mocked fetch)
// and verified against the real endpoints via live-check.mjs.

export const NYM_API = 'https://validator.nymtech.net/api';
export const NODE_STATUS_API = 'https://mainnet-node-status-api.nymtech.cc';

const DEFAULT_TIMEOUT_MS = 15_000;

export class NymApiError extends Error {
  constructor(
    public url: string,
    public status: number,
    public body: string,
  ) {
    super(`Nym API ${status} for ${url}: ${body.slice(0, 200)}`);
    this.name = 'NymApiError';
  }
}

/** GET + parse JSON with a timeout; throws NymApiError on non-2xx. */
export async function getJson<T>(url: string, timeoutMs = DEFAULT_TIMEOUT_MS): Promise<T> {
  const res = await fetch(url, {
    signal: AbortSignal.timeout(timeoutMs),
    headers: { accept: 'application/json' },
  });
  if (!res.ok) throw new NymApiError(url, res.status, await res.text().catch(() => ''));
  return (await res.json()) as T;
}

// --- Token economics (NymAPI) ---------------------------------------------

export interface Coin {
  denom: string;
  amount: string;
}

export interface CirculatingSupply {
  total_supply: Coin;
  mixmining_reserve: Coin;
  vesting_tokens: Coin;
  circulating_supply: Coin;
}

export function getCirculatingSupply(): Promise<CirculatingSupply> {
  return getJson(`${NYM_API}/v1/circulating-supply`);
}

/** unym (micro-NYM, the on-chain denom) to NYM. 1 NYM = 1e6 unym. */
export function unymToNym(coin: Coin): number {
  return Number(coin.amount) / 1e6;
}

export interface ChainStatus {
  connected_nyxd: string;
  status: unknown;
}

export function getChainStatus(): Promise<ChainStatus> {
  return getJson(`${NYM_API}/v1/network/chain-status`);
}

// --- Network + gateways (Node Status API) ---------------------------------

export interface BondedCount {
  count: number;
  last_updated_utc: string;
}

export interface NetworkSummary {
  total_nodes: number;
  mixnodes: { bonded: BondedCount & { self_described: number }; historical: BondedCount };
  gateways: { bonded: BondedCount & { entry: number; exit: number }; historical: BondedCount };
}

export function getNetworkSummary(): Promise<NetworkSummary> {
  return getJson(`${NODE_STATUS_API}/v2/summary`);
}

/** A page envelope as returned by the Node Status API list endpoints. */
export interface Page<T> {
  page: number;
  size: number;
  total: number;
  items: T[];
}

export interface GatewaySkinny {
  gateway_identity_key: string;
  routing_score: number;
  config_score: number;
  performance: number;
  self_described?: unknown;
}

export function listGateways(page = 0, size = 100): Promise<Page<GatewaySkinny>> {
  return getJson(`${NODE_STATUS_API}/v2/gateways/skinny?page=${page}&size=${size}`);
}

export interface GatewayDetails {
  gateway_identity_key: string;
  bonded: boolean;
  performance: number;
  routing_score: number;
  config_score: number;
  self_described?: unknown;
  description?: unknown;
  [key: string]: unknown;
}

export function getGateway(identityKey: string): Promise<GatewayDetails> {
  return getJson(`${NODE_STATUS_API}/v2/gateways/${encodeURIComponent(identityKey)}`);
}
