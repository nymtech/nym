// NOTE: removed the cache() import to avoid sticky results across requests
// import { cache } from "react";

type DeclaredRole = {
  entry?: boolean;
  exit_ipr?: boolean;
  exit_nr?: boolean;
  mixnode?: boolean;
};

type ApiNode = {
  node_id?: number;
  total_stake?: number | string;
  uptime?: number | string; // fraction 0..1
  description?: {
    wireguard?: unknown | null;
    declared_role?: DeclaredRole;
  };
  self_description?: {
    declared_role?: DeclaredRole;
  };
  rewarding_details?: {
    cost_params?: {
      profit_margin_percent?: string | number;
    };
  };
  bonded?: boolean;
};

function toNumber(x: unknown, fallback = 0): number {
  const n =
    typeof x === "string" || typeof x === "number" ? Number(x) : Number.NaN;
  return Number.isFinite(n) ? n : fallback;
}

const MIN_STAKE = 50_000_000_000; // 50k NYM (uNYM)
const MAX_STAKE = 150_000_000_000; // 150k NYM (uNYM)
const MAX_PM = 0.2; // ≤ 20%
const MIN_UPTIME = 0.95; // ≥ 95%

// require gateway roles: entry + exit_ipr + exit_nr; NOT a mixnode
function hasRequiredRoles(n: ApiNode): boolean {
  const r =
    n.self_description?.declared_role ?? n.description?.declared_role ?? {};
  const mixnodeFalse = r.mixnode === false || r.mixnode === undefined;
  return mixnodeFalse && !!r.entry && !!r.exit_ipr && !!r.exit_nr;
}

function hasGoodPM(n: ApiNode): boolean {
  const pm = toNumber(
    n.rewarding_details?.cost_params?.profit_margin_percent,
    Number.NaN,
  );
  return !Number.isNaN(pm) && pm <= MAX_PM;
}

function stakeInRange(n: ApiNode): boolean {
  const s = toNumber(n.total_stake, 0);
  return s > MIN_STAKE && s < MAX_STAKE;
}

function meetsUptime(n: ApiNode): boolean {
  const u = toNumber(n.uptime, -1);
  return u >= MIN_UPTIME;
}

function wireguardOn(n: ApiNode): boolean {
  return n.description?.wireguard != null;
}

function sortByUptimeDescStakeAsc(a: ApiNode, b: ApiNode): number {
  const ua = toNumber(a.uptime, 0);
  const ub = toNumber(b.uptime, 0);
  if (ub !== ua) return ub - ua; // higher uptime first
  const sa = toNumber(a.total_stake, 0);
  const sb = toNumber(b.total_stake, 0);
  return sa - sb; // then lower stake first
}

// fetch all for the nodes API
async function fetchAllNodes(): Promise<ApiNode[]> {
  const base = "https://api.nym.spectredao.net/api/v1/nodes";

  // 1. Try limit/offset
  {
    const limit = 1000;
    const all: ApiNode[] = [];
    let offset = 0;
    for (let i = 0; i < 200; i++) {
      const res = await fetch(`${base}?limit=${limit}&offset=${offset}`, {
        cache: "no-store",
      });
      if (!res.ok) break;
      const js = await res.json();
      const data: ApiNode[] = Array.isArray(js) ? js : Array.isArray(js?.data) ? js.data : [];
      if (!data.length) break;
      all.push(...data);
      if (data.length < limit) return all;
      offset += limit;
    }
    if (all.length) return all;
  }

  // 2. try page/size
  {
    const size = 1000;
    const all: ApiNode[] = [];
    let page = 0;
    for (let i = 0; i < 200; i++) {
      const res = await fetch(`${base}?page=${page}&size=${size}`, {
        cache: "no-store",
      });
      if (!res.ok) break;
      const js = await res.json();
      const data: ApiNode[] = Array.isArray(js) ? js : Array.isArray(js?.data) ? js.data : [];
      if (!data.length) break;
      all.push(...data);
      const total = Number(js?.pagination?.total ?? Number.NaN);
      if (Number.isFinite(total) && all.length >= total) return all;
      page += 1;
    }
    if (all.length) return all;
  }

  // 3. fallback single-shot
  const res = await fetch(base, { cache: "no-store" });
  if (!res.ok) throw new Error(`Failed to fetch nodes: ${res.status}`);
  const js = await res.json();
  return Array.isArray(js) ? js : Array.isArray(js?.data) ? js.data : [];
}

async function fetchRecommendedNodes(): Promise<number[]> {
  const nodes = await fetchAllNodes();

  const baseFilter = (n: ApiNode) =>
    (n.bonded === true || n.bonded === undefined) &&
    hasRequiredRoles(n) &&
    hasGoodPM(n) &&
    stakeInRange(n) &&
    meetsUptime(n); // uptime hard floor

  // prefer wg-enabled nodes first
  const wgCandidates = nodes
    .filter((n) => baseFilter(n) && wireguardOn(n))
    .sort(sortByUptimeDescStakeAsc);

  let picked = wgCandidates.slice(0, 10);

  // if fewer than 10, drop wg pref but keep base filter
  if (picked.length < 10) {
    const relaxed = nodes.filter(baseFilter).sort(sortByUptimeDescStakeAsc);
    const have = new Set(picked.map((n) => n.node_id));
    for (const n of relaxed) {
      if (have.size >= 10) break;
      const id = typeof n.node_id === "number" ? n.node_id : toNumber(n.node_id, 0);
      if (!have.has(id)) {
        picked = [...picked, n];
        have.add(id);
      }
    }
  }

  return picked
    .map((n) =>
      typeof n.node_id === "number" ? n.node_id : toNumber(n.node_id, 0),
    )
    .filter((id) => Number.isFinite(id) && id > 0);
}

// keep the same exports the code expects:
export async function getRecommendedNodes(): Promise<number[]> {
  return fetchRecommendedNodes();
}
export const RECOMMENDED_NODES: Promise<number[]> = getRecommendedNodes();
