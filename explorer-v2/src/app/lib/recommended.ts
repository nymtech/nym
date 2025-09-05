import { ApiNode } from "../api/types";

const API_URL = "https://api.nym.spectredao.net/api/v1/nodes?per_page=3000";

// filter logic
function baseFilter(node: ApiNode): boolean {
  const profitMargin = Number(node.rewarding_details?.cost_params.profit_margin_percent ?? "1");
  const totalStake = Number(node.total_stake);
  const hasRoles =
    node.self_description?.declared_role.entry === true &&
    node.self_description?.declared_role.exit_ipr === true &&
    node.self_description?.declared_role.exit_nr === true;

  return (
    node.bonded === true &&
    profitMargin <= 0.2 &&
    hasRoles &&
    totalStake > 50_000_000_000 &&
    totalStake <= 150_000_000_000
  );
}

function wireguardOn(node: ApiNode): boolean {
  return node.description?.wireguard != null;
}

function sortByUptimeDescStakeAsc(a: ApiNode, b: ApiNode): number {
  if (a.uptime !== b.uptime) {
    return b.uptime - a.uptime; // higher uptime first
  }
  return Number(a.total_stake) - Number(b.total_stake); // lower stake first
}

async function fetchRecommendedNodes(): Promise<number[]> {
  const res = await fetch(API_URL, { cache: "no-store" });
  if (!res.ok) throw new Error(`Failed to fetch nodes: ${res.status}`);
  const nodes: ApiNode[] = await res.json();

  // refer wg enabled nodes
  const wgCandidates = nodes
    .filter((n) => baseFilter(n) && wireguardOn(n))
    .sort(sortByUptimeDescStakeAsc);

  let picked: ApiNode[] = wgCandidates.slice(0, 10);

  // if < 10, relax wg but still enforce stake range
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

  return picked.map((n) => n.node_id);
}

// export as a promise so constants.ts and pages can await it
export const RECOMMENDED_NODES: Promise<number[]> = fetchRecommendedNodes();
