import { countryCodeMap } from "@/assets/countryCodes";
import { addSeconds } from "date-fns";
/* eslint-disable @typescript-eslint/no-explicit-any */
import type {
  CountryDataResponse,
  CurrentEpochData,
  ExplorerData,
  GatewayStatus,
  IAccountBalancesInfo,
  IObservatoryNode,
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
  DATA_OBSERVATORY_NODES_URL,
  NS_API_NODES,
  NYM_ACCOUNT_ADDRESS,
  NYM_PRICES_API,
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
  identityKey: string,
): Promise<GatewayStatus | null> => {
  const response = await fetch(`${OBSERVATORY_GATEWAYS_URL}/${identityKey}`);

  if (!response.ok) {
    throw new Error("Failed to fetch gateway status");
  }

  return response.json();
};

export const fetchNodeDelegations = async (
  id: number,
): Promise<NodeRewardDetails[]> => {
  const response = await fetch(
    `${DATA_OBSERVATORY_NODES_URL}/${id}/delegations`,
    {
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json; charset=utf-8",
      },
    },
  );

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
    data.epoch_length.secs,
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
  address: string,
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

export const fetchNoise = async (): Promise<IPacketsAndStakingData[]> => {
  if (!process.env.NEXT_PUBLIC_NS_API_MIXNODES_STATS) {
    throw new Error(
      "NEXT_PUBLIC_NS_API_MIXNODES_STATS environment variable is not defined",
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
  address: string,
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

export const fetchObservatoryNodes = async (): Promise<IObservatoryNode[]> => {
  const allNodes: IObservatoryNode[] = [];
  let page = 1;
  const PAGE_SIZE = 200;
  let hasMoreData = true;

  while (hasMoreData) {
    const response = await fetch(
      `${DATA_OBSERVATORY_NODES_URL}?page=${page}&limit=${PAGE_SIZE}`,
      {
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json; charset=utf-8",
        },
      },
    );

    if (!response.ok) {
      throw new Error(`Failed to fetch observatory nodes (page ${page})`);
    }

    const nodes: IObservatoryNode[] = await response.json();
    allNodes.push(...nodes);

    if (nodes.length < PAGE_SIZE) {
      hasMoreData = false; // Stop fetching when the last page has fewer than 200 items
    } else {
      page++; // Move to the next page
    }
  }
  return allNodes;
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
      `${NS_API_NODES}?page=${page}&limit=${PAGE_SIZE}`,
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

export const fetchWorldMapCountries =
  async (): Promise<CountryDataResponse> => {
    // Fetch all nodes from the NS API
    const nodes = await fetchNSApiNodes();

    // Create a map to count nodes by country
    const countryCounts: Record<string, number> = {};

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

    return result;
  };
