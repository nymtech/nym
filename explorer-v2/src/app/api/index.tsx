import { countryCodeMap } from "@/assets/countryCodes";
import { addSeconds } from "date-fns";
/* eslint-disable @typescript-eslint/no-explicit-any */
import type {
  CountryDataResponse,
  CurrentEpochData,
  Environment,
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
  DATA_OBSERVATORY_BALANCES_URL,
  NS_API_MIXNODES_STATS,
  NS_API_NODES,
  NYM_ACCOUNT_ADDRESS,
  NYM_PRICES_API,
  OBSERVATORY_GATEWAYS_URL,
  SANDBOX_CURRENT_EPOCH,
  SANDBOX_CURRENT_EPOCH_REWARDS,
  SANDBOX_NS_API_MIXNODES_STATS,
  SANDBOX_NS_API_NODES,
} from "./urls";

// Fetch function for epoch rewards
export const fetchEpochRewards = async (
  environment: Environment
): Promise<ExplorerData["currentEpochRewardsData"]> => {
  const baseUrl =
    environment === "sandbox"
      ? SANDBOX_CURRENT_EPOCH_REWARDS
      : CURRENT_EPOCH_REWARDS;

  if (!baseUrl) {
    throw new Error("CURRENT_EPOCH_REWARDS URL is not defined");
  }
  const response = await fetch(baseUrl, {
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
  environment: Environment,
  id: number
): Promise<NodeRewardDetails[]> => {
  const baseUrl =
    environment === "sandbox" ? SANDBOX_NS_API_NODES : NS_API_NODES;

  if (!baseUrl) {
    throw new Error("NS_API_NODES URL is not defined");
  }
  const response = await fetch(`${baseUrl}/${id}/delegations`, {
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

export const fetchCurrentEpoch = async (environment: Environment) => {
  const baseUrl =
    environment === "sandbox" ? SANDBOX_CURRENT_EPOCH : CURRENT_EPOCH;

  if (!baseUrl) {
    throw new Error("NS_API_NODES URL is not defined");
  }
  const response = await fetch(baseUrl, {
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
  const response = await fetch(`${DATA_OBSERVATORY_BALANCES_URL}/${address}`, {
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
export const fetchTotalStakerRewards = async (
  address: string
): Promise<number> => {
  const response = await fetch(`${DATA_OBSERVATORY_BALANCES_URL}/${address}`, {
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
  const response = await fetch(`${DATA_OBSERVATORY_BALANCES_URL}/${address}`, {
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

export const fetchNoise = async (
  environment: Environment
): Promise<IPacketsAndStakingData[]> => {
  const baseUrl =
    environment === "sandbox"
      ? SANDBOX_NS_API_MIXNODES_STATS
      : NS_API_MIXNODES_STATS;

  if (!baseUrl) {
    throw new Error("NS_API_MIXNODES_STATS URL is not defined");
  }

  const response = await fetch(baseUrl, {
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
  const res = await fetch(NYM_PRICES_API, {
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

export const fetchNSApiNodes = async (
  environment: Environment
): Promise<NS_NODE[]> => {
  const baseUrl =
    environment === "sandbox" ? SANDBOX_NS_API_NODES : NS_API_NODES;

  if (!baseUrl) {
    throw new Error("NS_API_NODES URL is not defined");
  }

  const allNodes: any[] = [];
  let page = 0;
  const PAGE_SIZE = 200;
  let totalItems = 0;
  let hasMoreData = true;

  while (hasMoreData) {
    const response = await fetch(`${baseUrl}?page=${page}&size=${PAGE_SIZE}`, {
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json; charset=utf-8",
      },
    });

    if (!response.ok) {
      throw new Error(
        `Failed to fetch NS API nodes (page ${page}): ${response.statusText}`
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

export const fetchWorldMapCountries = async (
  environment: Environment
): Promise<{
  countries: CountryDataResponse;
  totalCountries: number;
  uniqueLocations: number;
  totalServers: number;
}> => {
  // Fetch all nodes from the NS API
  const nodes = await fetchNSApiNodes(environment);

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
    environment,
  };
};


