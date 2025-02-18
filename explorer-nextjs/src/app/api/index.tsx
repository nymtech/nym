import { addSeconds } from "date-fns";
/* eslint-disable @typescript-eslint/no-explicit-any */
import type {
  CurrentEpochData,
  ExplorerData,
  GatewayStatus,
  IAccountBalancesInfo,
  IObservatoryNode,
  IPacketsAndStakingData,
  NodeData,
  NodeRewardDetails,
  NymTokenomics,
  ObservatoryBalance,
} from "./types";
import {
  CURRENT_EPOCH,
  CURRENT_EPOCH_REWARDS,
  DATA_OBSERVATORY_BALANCES_URL,
  DATA_OBSERVATORY_NODES_DELEGATIONS_URL,
  DATA_OBSERVATORY_NODES_URL,
  NS_API_MIXNODES_STATS,
  NYM_ACCOUNT_ADDRESS,
  NYM_NODES,
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

export const fetchNodeInfo = async (
  id: number,
): Promise<IObservatoryNode | undefined> => {
  const nodes = await fetchObservatoryNodes();
  return nodes?.find((node) => node.node_id === id);
};

export const fetchNodeDelegations = async (
  id: number,
): Promise<NodeRewardDetails[]> => {
  const response = await fetch(
    `${DATA_OBSERVATORY_NODES_DELEGATIONS_URL}/${id}/delegations`,
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
  const response = await fetch(NS_API_MIXNODES_STATS, {
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

// 🔹 Fetch Nodes
export const fetchNodes = async (): Promise<NodeData[]> => {
  const res = await fetch(NYM_NODES, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });
  if (!res.ok) {
    throw new Error("Failed to fetch nodes");
  }
  const data: NodeData[] = await res.json();
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
