import { cache } from "react";

type ApiNode = {
  node_id?: number;
  total_stake?: number | string;
  uptime?: number | string;
  description?: {
    mixnet_websockets?: { ws_port?: number | null };
    wireguard?: unknown | null;
    declared_role?: {
      entry?: boolean;
      exit_ipr?: boolean;
      exit_nr?: boolean;
    };
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

const MIN_STAKE = 50_000_000_000; // 50B

async function fetchRecommendedNodes(): Promise<number[]> {
  const url = "https://api.nym.spectredao.net/api/v1/nodes?size=3000";
  const res = await fetch(url, { cache: "no-store" });
  if (!res.ok) throw new Error(`Failed to fetch nodes: ${res.status}`);

  const data = (await res.json()) as unknown;

  if (!Array.isArray(data)) {
    throw new Error("Unexpected API shape: expected an array");
  }

  const nodes = data as ApiNode[];

  const filtered = nodes.filter((n) => {
    const ws9000 = n.description?.mixnet_websockets?.ws_port === 9000;
    const wgOn = n.description?.wireguard != null;

    const pm = toNumber(n.rewarding_details?.cost_params?.profit_margin_percent, NaN);
    const pmOk = !Number.isNaN(pm) && pm <= 0.2;

    const r = n.description?.declared_role;
    const rolesOk = !!(r?.entry && r?.exit_ipr && r?.exit_nr);

    const stakeOk = toNumber(n.total_stake, 0) > MIN_STAKE;

    return ws9000 && wgOn && pmOk && rolesOk && stakeOk;
  });

  filtered.sort((a, b) => {
    const ua = toNumber(a.uptime, 0);
    const ub = toNumber(b.uptime, 0);
    if (ub !== ua) return ub - ua; // uptime DESC

    const sa = toNumber(a.total_stake, 0);
    const sb = toNumber(b.total_stake, 0);
    return sa - sb; // stake ASC
  });

  return filtered
    .slice(0, 10)
    .map((n) => (typeof n.node_id === "number" ? n.node_id : toNumber(n.node_id, 0)))
    .filter((id) => Number.isFinite(id) && id > 0);
}

export const getRecommendedNodes = cache(fetchRecommendedNodes);
