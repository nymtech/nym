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
};

function toNumber(x: unknown, fallback = 0): number {
  const n = typeof x === "string" || typeof x === "number" ? Number(x) : NaN;
  return Number.isFinite(n) ? n : fallback;
}

const MIN_STAKE = 50_000_000_000;   // 50k NYM
const MAX_STAKE = 150_000_000_000;  // 150k NYM
const MAX_PM = 0.2;

function hasRequiredRoles(n: ApiNode): boolean {
  const r = n.self_description?.declared_role ?? n.description?.declared_role ?? {};
  return !!(r.entry && r.exit_ipr && r.exit_nr);
}

function hasGoodPM(n: ApiNode): boolean {
  const pm = toNumber(n.rewarding_details?.cost_params?.profit_margin_percent, NaN);
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
  if (ub !== ua) return ub - ua;
  const sa = toNumber(a.total_stake, 0);
  const sb = toNumber(b.total_stake, 0);
  return sa - sb;
}

async function fetchRecommendedNodes(): Promise<number[]> {
  const url = "https://api.nym.spectredao.net/api/v1/nodes?size=3000";
  const res = await fetch(url, { cache: "no-store" });
  if (!res.ok) throw new Error(`Failed to fetch nodes: ${res.status}`);

  const json = (await res.json()) as unknown;
  if (!Array.isArray(json)) throw new Error("Unexpected API shape: expected an array");
  const nodes = json as ApiNode[];

  // core predicate stake check mandatory
  const baseFilter = (n: ApiNode) => hasRequiredRoles(n) && hasGoodPM(n) && stakeInRange(n);

  // prefer WireGuard-enabled nodes
  const wgCandidates = nodes.filter((n) => baseFilter(n) && wireguardOn(n)).sort(sortByUptimeDescStakeAsc);

  let picked = wgCandidates.slice(0, 10);

  // if fewer than 10, relax WireGuard but still enforce stake range
  if (picked.length < 10) {
    const relaxed = nodes.filter(baseFilter).sort(sortByUptimeDescStakeAsc);
    const set = new Set(picked.map((n) => n.node_id));
    for (const n of relaxed) {
      if (set.size >= 10) break;
      if (!set.has(n.node_id)) {
        picked.push(n);
        set.add(n.node_id);
      }
    }
  }

  return picked
    .map((n) => (typeof n.node_id === "number" ? n.node_id : toNumber(n.node_id, 0)))
    .filter((id) => Number.isFinite(id) && id > 0);
}

export const getRecommendedNodes = cache(fetchRecommendedNodes);
