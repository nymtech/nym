import type { NextApiRequest, NextApiResponse } from "next";
import {
  EXPLORER_API,
  COSMOS_API,
  VALIDATOR_API_EPOCH,
  VALIDATOR_API_SUPPLY,
  HARBOURMASTER_API_SUMMARY,
  HARBOURMASTER_API_MIXNODES_STATS,
  HARBOURMASTER_API_BASE,
  CURRENT_EPOCH,
  CURRENT_EPOCH_REWARDS,
  CIRCULATING_NYM_SUPPLY,
} from "../urls";

export interface ExplorerData {
  circulatingNymSupplyData: any;
  nymNodesData: any;
  packetsAndStakingData: any;
  currentEpochData: any;
  currentEpochRewardsData: any;
}

export interface ExplorerCache {
  data?: ExplorerData;
  lastUpdated?: Date;
}

declare global {
  // Extend the global object with our custom property
  var explorerCache: ExplorerCache | undefined;
}

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
