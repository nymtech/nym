import { countryCodeMap } from "@/assets/countryCodes";
import { addSeconds } from "date-fns";
/* eslint-disable @typescript-eslint/no-explicit-any */
import type {
  CountryDataResponse,
  CurrentEpochData,
  ExplorerData,
  GatewayStatus,
  IAccountBalancesInfo,
  IPacketsAndStakingData,
  NS_NODE,
  NodeRewardDetails,
  NymTokenomics,
  ObservatoryBalance,
} from "./types";
import {
  CURRENT_EPOCH,
  CURRENT_EPOCH_REWARDS,
  SPECTREDAO_BALANCES_URL,
  NS_API_NODES,
  NYM_ACCOUNT_ADDRESS,
  SPECTREDAO_NYM_PRICES_API,
  OBSERVATORY_GATEWAYS_URL,
} from "./urls";

// Fetch function for epoch rewards
export const fetchEpochRewards = async (): Promise<
  ExplorerData["currentEpochRewardsData"]
> => {
  const response = await fetch(CURRENT_EPOCH_REWARDS, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    cache: "no-store", // Ensures fresh data on every request
  });

  if (!response.ok) {
    throw new Error("Failed to fetch epoch rewards");
  }

  return response.json();
};

// Fetch gateway status based on identity key
export const fetchGatewayStatus = async (
  identityKey: string
): Promise<GatewayStatus | null> => {
  const response = await fetch(`${OBSERVATORY_GATEWAYS_URL}/${identityKey}`);

  if (!response.ok) {
    throw new Error("Failed to fetch gateway status");
  }

  return response.json();
};

export const fetchNodeDelegations = async (
  id: number
): Promise<NodeRewardDetails[]> => {
  const response = await fetch(`${NS_API_NODES}/${id}/delegations`, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });

  if (!response.ok) {
    throw new Error("Failed to fetch delegations");
  }

  return response.json();
};

export const fetchCurrentEpoch = async () => {
  const response = await fetch(CURRENT_EPOCH, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    cache: "no-store", // Ensures fresh data on every request
  });

  if (!response.ok) {
    throw new Error("Failed to fetch current epoch data");
  }

  const data: CurrentEpochData = await response.json();
  const epochEndTime = addSeconds(
    new Date(data.current_epoch_start),
    data.epoch_length.secs
  ).toISOString();

  return { ...data, current_epoch_end: epochEndTime };
};

// Fetch balances based on the address
export const fetchBalances = async (address: string): Promise<number> => {
  const response = await fetch(`${SPECTREDAO_BALANCES_URL}/${address}`, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });

  if (!response.ok) {
    throw new Error("Failed to fetch balances");
  }

  const balances: ObservatoryBalance = await response.json();

  // Calculate total stake
  return (
    Number(balances.rewards.staking_rewards.amount) +
    Number(balances.delegated.amount)
  );
};

// Fetch function to get total staker rewards
export const fetchTotalStakerRewards = async (address: string): Promise<number> => {
  const response = await fetch(`${SPECTREDAO_BALANCES_URL}/${address}`, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });

  if (!response.ok) {
    throw new Error("Failed to fetch balances");
  }

  const balances: ObservatoryBalance = await response.json();

  // Return the staking rewards amount
  return Number(balances.rewards.staking_rewards.amount);
};

// Fetch function to get the original stake
export const fetchOriginalStake = async (address: string): Promise<number> => {
  const response = await fetch(`${SPECTREDAO_BALANCES_URL}/${address}`, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });

  if (!response.ok) {
    throw new Error("Failed to fetch balances");
  }

  const balances: ObservatoryBalance = await response.json();

  // Return the delegated amount
  return Number(balances.delegated.amount);
};

export const fetchNoise = async (): Promise<IPacketsAndStakingData[]> => {
  if (!process.env.NEXT_PUBLIC_NS_API_MIXNODES_STATS) {
    throw new Error(
      "NEXT_PUBLIC_NS_API_MIXNODES_STATS environment variable is not defined"
    );
  }
  const response = await fetch(process.env.NEXT_PUBLIC_NS_API_MIXNODES_STATS, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });

  const data: IPacketsAndStakingData[] = await response.json();
  return data;
};

// Fetch Account Balance
export const fetchAccountBalance = async (
  address: string
): Promise<IAccountBalancesInfo> => {
  const res = await fetch(`${NYM_ACCOUNT_ADDRESS}/${address}`, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });
  if (!res.ok) {
    throw new Error("Failed to fetch account balance error from api");
  }

  const data: IAccountBalancesInfo = await res.json();
  return data;
};

// 🔹 Fetch NYM Price
export const fetchNymPrice = async (): Promise<NymTokenomics> => {
  const res = await fetch(SPECTREDAO_NYM_PRICES_API, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });
  if (!res.ok) {
    throw new Error("Failed to fetch NYM price");
  }
  const data: NymTokenomics = await res.json();
  return data;
};

export const fetchNSApiNodes = async (): Promise<NS_NODE[]> => {
  if (!NS_API_NODES) {
    throw new Error("NS_API_NODES URL is not defined");
  }

  const allNodes: any[] = [];
  let page = 0;
  const PAGE_SIZE = 200;
  let totalItems = 0;
  let hasMoreData = true;

  while (hasMoreData) {
    const response = await fetch(
      `${NS_API_NODES}?page=${page}&size=${PAGE_SIZE}`,
      {
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json; charset=utf-8",
        },
      },
    );

    if (!response.ok) {
      throw new Error(
        `Failed to fetch NS API nodes (page ${page}): ${response.statusText}`,
      );
    }

    const data = await response.json();
    const nodes: any[] = data.items || [];
    allNodes.push(...nodes);

    // Get total count from response
    totalItems = data.total || 0;

    // Check if we've fetched all items
    if (allNodes.length >= totalItems) {
      hasMoreData = false;
    } else {
      page++; // Move to the next page
    }
  }

  return allNodes;
};

export const fetchWorldMapCountries = async (): Promise<{
  countries: CountryDataResponse;
  totalCountries: number;
  uniqueLocations: number;
  totalServers: number;
}> => {
  // Fetch all nodes from the NS API
  const nodes = await fetchNSApiNodes();

  // Create a map to count nodes by country
  const countryCounts: Record<string, number> = {};
  // Set to track unique cities
  const uniqueCities = new Set<string>();

  // Process each node
  for (const node of nodes) {
    // Get the 2-letter country code from the node's geoip data
    const twoLetterCode = node.geoip?.country;

    if (twoLetterCode) {
      // Convert to 3-letter country code
      const threeLetterCode = countryCodeMap[twoLetterCode] || twoLetterCode;

      // Increment the count for this country
      countryCounts[threeLetterCode] =
        (countryCounts[threeLetterCode] || 0) + 1;

      // Add city to unique cities set if it exists
      if (node.geoip?.city) {
        uniqueCities.add(node.geoip.city);
      }
    } else {
      // If no geoip data, count it as unknown
      countryCounts[""] = (countryCounts[""] || 0) + 1;
    }
  }

  // Convert the counts to the required format
  const result: CountryDataResponse = {};

  for (const [threeLetterCode, count] of Object.entries(countryCounts)) {
    result[threeLetterCode] = {
      ISO3: threeLetterCode,
      nodes: count,
    };
  }

  return {
    countries: result,
    totalCountries: Object.keys(countryCounts).length,
    uniqueLocations: uniqueCities.size,
    totalServers: nodes.length,
  };
};

export const getRecommendedNodes = (nodes: NS_NODE[]): number[] => {
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
  function hasRequiredRoles(n: NS_NODE): boolean {
    const r = n.self_description?.declared_role;
    if (!r) return false;
    const mixnodeFalse = r.mixnode === false || r.mixnode === undefined;
    return mixnodeFalse && !!r.entry && !!r.exit_ipr && !!r.exit_nr;
  }

  function hasGoodPM(n: NS_NODE): boolean {
    const pm = toNumber(
      n.rewarding_details?.cost_params?.profit_margin_percent,
      Number.NaN
    );
    return !Number.isNaN(pm) && pm <= MAX_PM;
  }

  function stakeInRange(n: NS_NODE): boolean {
    const s = toNumber(n.total_stake, 0);
    return s > MIN_STAKE && s < MAX_STAKE;
  }

  function meetsUptime(n: NS_NODE): boolean {
    const u = toNumber(n.uptime, -1);
    return u >= MIN_UPTIME;
  }

  function wireguardOn(n: NS_NODE): boolean {
    return n.self_description?.wireguard != null;
  }

  function sortByUptimeDescStakeAsc(a: NS_NODE, b: NS_NODE): number {
    const ua = toNumber(a.uptime, 0);
    const ub = toNumber(b.uptime, 0);
    if (ub !== ua) return ub - ua; // higher uptime first
    const sa = toNumber(a.total_stake, 0);
    const sb = toNumber(b.total_stake, 0);
    return sa - sb; // then lower stake first
  }
  const baseFilter = (n: NS_NODE) =>
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
      const id =
        typeof n.node_id === "number" ? n.node_id : toNumber(n.node_id, 0);
      if (!have.has(id)) {
        picked = [...picked, n];
        have.add(id);
      }
    }
  }

  return picked
    .map((n) =>
      typeof n.node_id === "number" ? n.node_id : toNumber(n.node_id, 0)
    )
    .filter((id) => Number.isFinite(id) && id > 0);
};
