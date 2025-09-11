import { cache } from "react";

type DeclaredRole = {
  entry?: boolean;
  exit_ipr?: boolean;
  exit_nr?: boolean;
  mixnode?: boolean;
};

type ApiNode = {
  node_id?: number;
  total_stake?: number | string;
  uptime?: number | string;
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

const MIN_STAKE = 50_000_000_000; // 50k NYM
const MAX_STAKE = 150_000_000_000; // 150k NYM
const MAX_PM = 0.2;

// require exit gw
function hasRequiredRoles(n: ApiNode): boolean {
  const r =
    n.self_description?.declared_role ?? n.description?.declared_role ?? {};
  return r.mixnode === false && !!r.entry && !!r.exit_ipr && !!r.exit_nr;
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

// fetch all pages of nodes from the API
async function fetchAllNodes(): Promise<ApiNode[]> {
  let page = 0;
  const pageSize = 200; // API default max
  let all: ApiNode[] = [];

  while (true) {
    const res = await fetch(
      `https://api.nym.spectredao.net/api/v1/nodes?page=${page}&size=${pageSize}`,
      { cache: "no-store" },
    );
    if (!res.ok) throw new Error(`Failed to fetch page ${page}`);
    const json = await res.json();
    const data: ApiNode[] = Array.isArray(json) ? json : json.data;
    if (!data || data.length === 0) break;
    all = [...all, ...data];
    if (data.length < pageSize) break; // last page
    page++;
  }

  return all;
}

async function fetchRecommendedNodes(): Promise<number[]> {
  const nodes = await fetchAllNodes();

  const baseFilter = (n: ApiNode) =>
    (n.bonded === true || n.bonded === undefined) &&
    hasRequiredRoles(n) &&
    hasGoodPM(n) &&
    stakeInRange(n);

  // prefer wg enabled nodes
  const wgCandidates = nodes
    .filter((n) => baseFilter(n) && wireguardOn(n))
    .sort(sortByUptimeDescStakeAsc);

  let picked = wgCandidates.slice(0, 10);

  // if fewer than 10, drop wg but still enforce base filter
  if (picked.length < 10) {
    const relaxed = nodes.filter(baseFilter).sort(sortByUptimeDescStakeAsc);
    const set = new Set(picked.map((n) => n.node_id));
    for (const n of relaxed) {
      if (set.size >= 10) break;
      if (!set.has(n.node_id)) {
        picked = [...picked, n];
        set.add(n.node_id);
      }
    }
  }

  return picked
    .map((n) =>
      typeof n.node_id === "number" ? n.node_id : toNumber(n.node_id, 0),
    )
    .filter((id) => Number.isFinite(id) && id > 0);
}

export const getRecommendedNodes = cache(fetchRecommendedNodes);
export const RECOMMENDED_NODES: Promise<number[]> = getRecommendedNodes();
