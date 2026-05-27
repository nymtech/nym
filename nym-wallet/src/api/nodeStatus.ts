import type { Network, TNodeRole } from 'src/types';

const MAINNET_BASE = 'https://mainnet-node-status-api.nymtech.cc';
const SANDBOX_BASE = 'https://sandbox-node-status-api.nymte.ch';

/** Explorer list responses are paginated; cache successful lookups to avoid repeated full scans. */
const EXPLORER_CACHE_TTL_MS = 5 * 60 * 1000;
const EXPLORER_MISS_TTL_MS = 60 * 1000;
const EXPLORER_PAGE_SIZE = 200;

type ExplorerCacheEntry = { data: ExplorerNymNodeRow | null; expiresAt: number };

const explorerCache = new Map<string, ExplorerCacheEntry>();
const explorerInflight = new Map<string, Promise<ExplorerNymNodeRow | null>>();

/**
 * Clears explorer list cache and in-flight scans. Call from tests between cases; optional for a future forced refresh UX.
 */
export function clearNodeStatusExplorerCaches(): void {
  explorerCache.clear();
  explorerInflight.clear();
}

export function getNodeStatusBaseUrl(network?: Network): string {
  if (network === 'SANDBOX' || network === 'QA') {
    return SANDBOX_BASE;
  }
  return MAINNET_BASE;
}

const jsonHeaders = {
  Accept: 'application/json',
  'Content-Type': 'application/json; charset=utf-8',
};

/** Subset of probe outcome used for charts (matches node-status API JSON shape). */
export type GatewayProbeOutcome = {
  as_entry?: { can_connect?: boolean; can_route?: boolean } | null;
  as_exit?: {
    can_connect?: boolean;
    can_route_ip_v4?: boolean;
    can_route_ip_v6?: boolean;
    can_route_ip_external_v4?: boolean;
    can_route_ip_external_v6?: boolean;
  } | null;
  socks5?: {
    can_connect_socks5?: boolean;
    https_connectivity?: { https_success?: boolean; https_latency_ms?: number };
  } | null;
  wg?: {
    can_handshake_v4?: boolean;
    can_handshake_v6?: boolean;
    can_query_metadata_v4?: boolean;
    can_register?: boolean;
    can_resolve_dns_v4?: boolean;
    can_resolve_dns_v6?: boolean;
    ping_hosts_performance_v4?: number;
    ping_hosts_performance_v6?: number;
    ping_ips_performance_v4?: number;
    ping_ips_performance_v6?: number;
    download_duration_milliseconds_v4?: number;
    download_duration_milliseconds_v6?: number;
  } | null;
  lp?: {
    can_connect?: boolean;
    can_handshake?: boolean;
    can_register?: boolean;
    error?: string | null;
  } | null;
};

export type GatewayStatusPayload = {
  gateway_identity_key: string;
  bonded: boolean;
  performance: number;
  routing_score: number;
  last_probe_result?: {
    outcome?: GatewayProbeOutcome;
  } | null;
  last_testrun_utc?: string | null;
  last_updated_utc: string;
  description?: {
    moniker?: string;
    website?: string;
    details?: string;
    security_contact?: string;
  };
  explorer_pretty_bond?: {
    location?: {
      two_letter_iso_country_code?: string;
      city?: string;
      region?: string;
    };
  };
};

/** Fields used from `GET /explorer/v3/nym-nodes` items (identity-key match). */
export type ExplorerNymNodeRow = {
  identity_key: string;
  node_id: number;
  bonded: boolean;
  /** Explorer may send percent, ratio 0-1, or a numeric string. */
  uptime: number | string;
  total_stake: string;
  original_pledge: number;
  description: {
    moniker: string;
    website: string;
    details: string;
    security_contact: string;
  };
  geoip?: {
    city?: string;
    country?: string;
    region?: string;
    two_letter_iso_country_code?: string;
  } | null;
  rewarding_details?: {
    unique_delegations?: number;
  } | null;
};

type PagedExplorerNymNodes = {
  items: ExplorerNymNodeRow[];
  page: number;
  size: number;
  total: number;
};

function explorerCacheKey(network: Network | undefined, identityKey: string): string {
  return `${network ?? 'MAINNET'}:${identityKey}`;
}

async function fetchExplorerNymNodesPage(baseUrl: string, page: number, size: number): Promise<PagedExplorerNymNodes> {
  const root = baseUrl.replace(/\/$/, '');
  const url = `${root}/explorer/v3/nym-nodes?page=${page}&size=${size}`;
  const response = await fetch(url, { headers: jsonHeaders, cache: 'no-store' });
  if (!response.ok) {
    throw new Error(`Explorer nym-nodes request failed (${response.status})`);
  }
  return response.json() as Promise<PagedExplorerNymNodes>;
}

/**
 * Paginates `GET /explorer/v3/nym-nodes` until `identity_key` matches or the list ends.
 * Results are cached per network + identity (hits: 5 min, misses: 60 s). Concurrent calls share one in-flight scan.
 */
export function findExplorerNymNodeByIdentity(
  network: Network | undefined,
  baseUrl: string,
  identityKey: string,
): Promise<ExplorerNymNodeRow | null> {
  const key = explorerCacheKey(network, identityKey);
  const now = Date.now();
  const cached = explorerCache.get(key);
  if (cached && cached.expiresAt > now) {
    return Promise.resolve(cached.data);
  }

  const pending = explorerInflight.get(key);
  if (pending) {
    return pending;
  }

  const promise = (async (): Promise<ExplorerNymNodeRow | null> => {
    try {
      let page = 0;
      let total = Number.POSITIVE_INFINITY;
      while (page * EXPLORER_PAGE_SIZE < total) {
        // eslint-disable-next-line no-await-in-loop -- sequential pagination required
        const data = await fetchExplorerNymNodesPage(baseUrl, page, EXPLORER_PAGE_SIZE);
        total = data.total;
        const found = data.items.find((item) => item.identity_key === identityKey);
        if (found) {
          explorerCache.set(key, { data: found, expiresAt: Date.now() + EXPLORER_CACHE_TTL_MS });
          return found;
        }
        const totalPages = Math.ceil(data.total / EXPLORER_PAGE_SIZE);
        if (page >= totalPages - 1 || data.items.length < EXPLORER_PAGE_SIZE) {
          break;
        }
        page += 1;
      }
      explorerCache.set(key, { data: null, expiresAt: Date.now() + EXPLORER_MISS_TTL_MS });
      return null;
    } finally {
      explorerInflight.delete(key);
    }
  })();

  explorerInflight.set(key, promise);
  return promise;
}

export function isGatewayRole(role: TNodeRole | undefined): boolean {
  return role === 'entryGateway' || role === 'exitGateway';
}

/**
 * When the status API lists this identity as a bonded gateway, returns the payload.
 * Otherwise `null` (404, not bonded, or identity mismatch) so callers can fall back to explorer / mixnode paths.
 * Still throws on other HTTP errors so real failures surface.
 */
export async function fetchGatewayStatusIfBonded(
  baseUrl: string,
  identityKey: string,
): Promise<GatewayStatusPayload | null> {
  const root = baseUrl.replace(/\/$/, '');
  const url = `${root}/v2/gateways/${encodeURIComponent(identityKey)}`;
  const response = await fetch(url, { headers: jsonHeaders, cache: 'no-store' });
  // Mixnodes are not in the gateway index. The API returns 404 or, for some identities, 400 with a short body.
  // Treat both as "no gateway row" so the wallet can fall back to explorer. Other 4xx are unexpected; 5xx throws below.
  if (response.status === 404 || response.status === 400) {
    return null;
  }
  if (!response.ok) {
    throw new Error(`Gateway status request failed (${response.status})`);
  }
  const data = (await response.json()) as GatewayStatusPayload;
  if (!data.bonded || data.gateway_identity_key !== identityKey) {
    return null;
  }
  return data;
}

export async function fetchGatewayStatus(baseUrl: string, identityKey: string): Promise<GatewayStatusPayload> {
  const g = await fetchGatewayStatusIfBonded(baseUrl, identityKey);
  if (!g) {
    throw new Error('Gateway status request failed (404)');
  }
  return g;
}

/** Parse API `total_stake` string (base unym) to NYM display amount. */
export function parseTotalStakeToNymAmount(totalStake: string): number {
  const n = Number.parseFloat(totalStake);
  if (!Number.isFinite(n)) {
    return 0;
  }
  return n / 1_000_000;
}

/**
 * API may return 0-100 (percent), 0-1 (ratio), or numeric strings. Values in [0, 1] are treated as a ratio.
 */
export function normalizeExplorerUptimePercent(raw: unknown): number {
  const n = typeof raw === 'string' ? Number.parseFloat(raw.trim()) : Number(raw);
  if (!Number.isFinite(n) || n < 0) {
    return 0;
  }
  if (n <= 1) {
    return Math.min(100, n * 100);
  }
  return Math.min(100, n);
}
