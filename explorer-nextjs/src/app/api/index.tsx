import {
  CIRCULATING_NYM_SUPPLY,
  CURRENT_EPOCH,
  CURRENT_EPOCH_REWARDS,
  HARBOURMASTER_API_MIXNODES_STATS,
  HARBOURMASTER_API_SUMMARY,
} from "./urls";

type Denom = "unym" | "nym";

export interface IPacketsAndStakingData {
  date_utc: string;
  total_packets_received: number;
  total_packets_sent: number;
  total_packets_dropped: number;
  total_stake: number;
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
  currentEpochData: {
    id: number;
    current_epoch_id: number;
    current_epoch_start: string;
    epoch_length: { secs: number; nanos: number };
    epochs_in_interval: number;
    total_elapsed_epochs: number;
  };
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

export interface ExplorerCache {
  data?: ExplorerData;
  lastUpdated?: Date;
}

// declare global {
//   // Extend the global object with our custom property
//   let explorerCache: ExplorerCache | undefined;
// }

const CACHE_TIME_SECONDS = 60 * 5; // 5 minutes

const getExplorerData = async () => {
  // FETCH NYMNODES
  const fetchNymNodes = await fetch(HARBOURMASTER_API_SUMMARY, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    // refresh event list cache at given interval
    next: { revalidate: Number(process.env.NEXT_PUBLIC_REVALIDATE_CACHE) },
  });

  // FETCH CURRENT EPOCH
  const fetchCurrentEpoch = await fetch(CURRENT_EPOCH, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    // refresh event list cache at given interval
    next: { revalidate: Number(process.env.NEXT_PUBLIC_REVALIDATE_CACHE) },
  });

  // FETCH CURRENT EPOCH REWARDS
  const fetchCurrentEpochRewards = await fetch(CURRENT_EPOCH_REWARDS, {
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

  const [
    circulatingNymSupplyRes,
    nymNodesRes,
    packetsAndStakingRes,
    currentEpochRes,
    currentEpochRewardsRes,
  ] = await Promise.all([
    fetchCirculatingNymSupply,
    fetchNymNodes,
    fetchPacketsAndStaking,
    fetchCurrentEpoch,
    fetchCurrentEpochRewards,
  ]);

  const [
    circulatingNymSupplyData,
    nymNodesData,
    packetsAndStakingData,
    currentEpochData,
    currentEpochRewardsData,
  ] = await Promise.all([
    circulatingNymSupplyRes.json(),
    nymNodesRes.json(),
    packetsAndStakingRes.json(),
    currentEpochRes.json(),
    currentEpochRewardsRes.json(),
  ]);

  return [
    circulatingNymSupplyData,
    nymNodesData,
    packetsAndStakingData,
    currentEpochData,
    currentEpochRewardsData,
  ];
};

export async function ensureCacheExists() {
  // makes sure the cache exists in global memory
  let doUpdate = false;
  const now = new Date();
  if (!global.explorerCache) {
    global.explorerCache = {};
    doUpdate = true;
  }
  if (
    global.explorerCache.lastUpdated &&
    now.getDate() - global.explorerCache.lastUpdated.getDate() >
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

    global.explorerCache.data = {
      circulatingNymSupplyData,
      nymNodesData,
      packetsAndStakingData,
      currentEpochData,
      currentEpochRewardsData,
    };
    global.explorerCache.lastUpdated = now;
  }
}

export async function getCacheExplorerData() {
  await ensureCacheExists();

  if (!global.explorerCache?.data) {
    return null;
  }

  console.log("global.explorerCache.data :>> ", global.explorerCache.data);
  return global.explorerCache.data || null;
}

/**
 * This is a custom API route that returns metadata from Strapi about images: height, width, strapi download url.
 *
 * The response from Strapi is cached in memory for CACHE_TIME_SECONDS.
 */
// export default async function handler(
//   req: NextApiRequest,
//   res: NextApiResponse
// ) {
//   // return cached data
//   const data = await getCacheExplorerData();
//   if (data) {
//     res.status(200).json(data);
//     res.end();
//   }

//   // catch-all
//   res.status(404).end();
// }
