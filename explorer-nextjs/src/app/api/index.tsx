/* eslint-disable @typescript-eslint/no-explicit-any */
import {
  CIRCULATING_NYM_SUPPLY,
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
