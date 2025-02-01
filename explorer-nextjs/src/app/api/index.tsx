import { addSeconds } from "date-fns";
/* eslint-disable @typescript-eslint/no-explicit-any */
import type {
  ExplorerData,
  IObservatoryNode,
  IPacketsAndStakingData,
  ObservatoryBalance,
} from "./types";
import {
  CURRENT_EPOCH,
  CURRENT_EPOCH_REWARDS,
  DATA_OBSERVATORY_BALANCES_URL,
  DATA_OBSERVATORY_NODES_URL,
  HARBOURMASTER_API_MIXNODES_STATS,
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

export const fetchNodeInfo = async (
  id: number,
): Promise<IObservatoryNode | null> => {
  const response = await fetch(DATA_OBSERVATORY_NODES_URL, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
  });

  if (!response.ok) {
    throw new Error("Failed to fetch observatory nodes");
  }

  const nodes: IObservatoryNode[] = await response.json();
  return nodes.find((node) => node.node_id === id) || null;
};

export const fetchNodeDelegations = async (id: number) => {
  const response = await fetch(
    `${DATA_OBSERVATORY_NODES_URL}/${id}/delegations`,
    {
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json; charset=utf-8",
      },
      next: { revalidate: 60 },
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
    next: { revalidate: 30 },
  });

  if (!response.ok) {
    throw new Error("Failed to fetch current epoch data");
  }

  const data = await response.json();
  const dateTime = addSeconds(
    new Date(data.current_epoch_start),
    data.epoch_length.secs,
  );

  return { data, dateTime };
};

// Fetch balances based on the address
export const fetchBalances = async (address: string): Promise<number> => {
  const response = await fetch(`${DATA_OBSERVATORY_BALANCES_URL}/${address}`, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
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
    next: { revalidate: 60 },
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
    next: { revalidate: 60 },
  });

  if (!response.ok) {
    throw new Error("Failed to fetch balances");
  }

  const balances: ObservatoryBalance = await response.json();

  // Return the delegated amount
  return Number(balances.delegated.amount);
};

export const fetchNoise = async (): Promise<IPacketsAndStakingData[]> => {
  const response = await fetch(HARBOURMASTER_API_MIXNODES_STATS, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
  });

  const data: IPacketsAndStakingData[] = await response.json();
  return data;
};
