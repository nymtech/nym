import { addSeconds } from "date-fns";
/* eslint-disable @typescript-eslint/no-explicit-any */
import type { IObservatoryNode, ObservatoryBalance } from "./types";
import {
  CIRCULATING_NYM_SUPPLY,
  CURRENT_EPOCH,
  CURRENT_EPOCH_REWARDS,
  DATA_OBSERVATORY_BALANCES_URL,
  DATA_OBSERVATORY_NODES_URL,
  HARBOURMASTER_API_MIXNODES_STATS,
  NYM_NODES_DESCRIBED,
  NYM_NODE_DESCRIPTION,
} from "./urls";

type Denom = "unym" | "nym";

export interface IPacketsAndStakingData {
  date_utc: string;
  total_packets_received: number;
  total_packets_sent: number;
  total_packets_dropped: number;
  total_stake: number;
}

export interface CurrentEpochData {
  id: number;
  current_epoch_id: number;
  current_epoch_start: string;
  epoch_length: { secs: number; nanos: number };
  epochs_in_interval: number;
  total_elapsed_epochs: number;
}
export interface ExplorerData {
  circulatingNymSupplyData: {
    circulating_supply: { denom: Denom; amount: string };
    mixmining_reserve: { denom: Denom; amount: string };
    total_supply: { denom: Denom; amount: string };
    vesting_tokens: { denom: Denom; amount: string };
  };
  nymNodesData: {
    gateways: {
      bonded: { count: number; last_updated_utc: string };
      blacklisted: { count: number; last_updated_utc: string };
      historical: { count: number; last_updated_utc: string };
      explorer: { count: number; last_updated_utc: string };
    };
    mixnodes: {
      bonded: {
        count: number;
        active: number;
        inactive: number;
        reserve: number;
        last_updated_utc: string;
      };
      blacklisted: {
        count: number;
        last_updated_utc: string;
      };
      historical: { count: number; last_updated_utc: string };
    };
  };
  packetsAndStakingData: IPacketsAndStakingData[];

  currentEpochRewardsData: {
    interval: {
      reward_pool: string;
      staking_supply: string;
      staking_supply_scale_factor: string;
      epoch_reward_budget: string;
      stake_saturation_point: string;
      active_set_work_factor: string;
      interval_pool_emission: string;
      sybil_resistance: string;
    };
    rewarded_set: {
      entry_gateways: number;
      exit_gateways: number;
      mixnodes: number;
      standby: number;
    };
  };
}

const CACHE_TIME_SECONDS = 60 * 5; // 5 minutes

export interface ExplorerCache {
  explorerCache?: {
    data?: ExplorerData;
    lastUpdated?: Date;
  };
}

const getExplorerData = async () => {
  // FETCH NYMNODES
  const fetchNymNodes = await fetch(NYM_NODES_DESCRIBED, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    // refresh event list cache at given interval
    next: { revalidate: Number(process.env.NEXT_PUBLIC_REVALIDATE_CACHE) },
  });

  // FETCH CIRCULATING NYM SUPPLY
  const fetchCirculatingNymSupply = await fetch(CIRCULATING_NYM_SUPPLY, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    // refresh event list cache at given interval
    next: { revalidate: Number(process.env.NEXT_PUBLIC_REVALIDATE_CACHE) },
  });

  // FETCH PACKETS AND STAKING
  const fetchPacketsAndStaking = await fetch(HARBOURMASTER_API_MIXNODES_STATS, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    // refresh event list cache at given interval
    next: { revalidate: Number(process.env.NEXT_PUBLIC_REVALIDATE_CACHE) },
  });

  const fetchNymNodeDescription = await fetch(NYM_NODE_DESCRIPTION, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    // refresh event list cache at given interval
    next: { revalidate: Number(process.env.NEXT_PUBLIC_REVALIDATE_CACHE) },
  });

  const [
    circulatingNymSupplyRes,
    nymNodesRes,
    packetsAndStakingRes,
    nymNodeDescriptionRes,
  ] = await Promise.all([
    fetchCirculatingNymSupply,
    fetchNymNodes,
    fetchPacketsAndStaking,
    fetchNymNodeDescription,
  ]);

  const [
    circulatingNymSupplyData,
    nymNodesData,
    packetsAndStakingData,
    nymNodeDescriptionData,
  ] = await Promise.all([
    circulatingNymSupplyRes.json(),
    nymNodesRes.json(),
    packetsAndStakingRes.json(),
    nymNodeDescriptionRes.json(),
  ]);

  return [
    circulatingNymSupplyData,
    nymNodesData,
    packetsAndStakingData,
    nymNodeDescriptionData,
  ];
};

export async function ensureCacheExists() {
  // makes sure the cache exists in global memory
  let doUpdate = false;
  const now = new Date();
  if (!(global as ExplorerCache).explorerCache) {
    (global as any).explorerCache = {};
    doUpdate = true;
  }
  if (
    (global as ExplorerCache)?.explorerCache?.lastUpdated &&
    now.getDate() - (global as any).explorerCache.lastUpdated.getDate() >
      CACHE_TIME_SECONDS
  ) {
    doUpdate = true;
  }

  // if the cache has expired or never existed, get it from API's
  if (doUpdate) {
    const [
      circulatingNymSupplyData,
      nymNodesData,
      packetsAndStakingData,
      currentEpochData,
      currentEpochRewardsData,
    ] = await getExplorerData();

    packetsAndStakingData.pop();

    (global as any).explorerCache.data = {
      circulatingNymSupplyData,
      nymNodesData,
      packetsAndStakingData,
      currentEpochData,
      currentEpochRewardsData,
    };
    (global as any).explorerCache.lastUpdated = now;
  }
}

export async function getCacheExplorerData() {
  await ensureCacheExists();

  if (!(global as ExplorerCache).explorerCache?.data) {
    return null;
  }

  return (global as ExplorerCache)?.explorerCache?.data || null;
}

// Fetch function for epoch rewards
export const fetchEpochRewards = async (): Promise<
  ExplorerData["currentEpochRewardsData"]
> => {
  const response = await fetch(CURRENT_EPOCH_REWARDS, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
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
