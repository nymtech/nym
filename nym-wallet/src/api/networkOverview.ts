import { addSeconds } from 'date-fns';
import type { Network } from 'src/types';

export type NetworkOverviewEndpoints = {
  mixnodeStats: string;
  epochCurrent: string;
  epochRewards: string;
  nymPrice: string;
  observatoryNodesBase: string;
};

export const getNetworkOverviewEndpoints = (network?: Network): NetworkOverviewEndpoints => {
  if (network === 'SANDBOX' || network === 'QA') {
    return {
      mixnodeStats: 'https://sandbox-node-status-api.nymte.ch/v2/mixnodes/stats',
      epochCurrent: 'https://sandbox-nym-api1.nymtech.net/api/v1/epoch/current',
      epochRewards: 'https://sandbox-nym-api1.nymtech.net/api/v1/epoch/reward_params',
      nymPrice: 'https://api.nym.spectredao.net/api/v1/nym-price',
      observatoryNodesBase: 'https://sandbox-node-status-api.nymte.ch/explorer/v3/nym-nodes',
    };
  }
  return {
    mixnodeStats: 'https://mainnet-node-status-api.nymtech.cc/v2/mixnodes/stats',
    epochCurrent: 'https://validator.nymtech.net/api/v1/epoch/current',
    epochRewards: 'https://validator.nymtech.net/api/v1/epoch/reward_params',
    nymPrice: 'https://api.nym.spectredao.net/api/v1/nym-price',
    observatoryNodesBase: 'https://mainnet-node-status-api.nymtech.cc/explorer/v3/nym-nodes',
  };
};

export interface PacketsAndStakingPoint {
  date_utc: string;
  total_packets_received: number;
  total_packets_sent: number;
  total_packets_dropped: number;
  total_stake: number;
}

export interface CurrentEpochApiData {
  id: number;
  current_epoch_id: number;
  current_epoch_start: string;
  epoch_length: { secs: number; nanos: number };
  epochs_in_interval: number;
  total_elapsed_epochs: number;
}

export type CurrentEpochWithEnd = CurrentEpochApiData & { current_epoch_end: string };

export interface EpochRewardsData {
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
}

export interface NymTokenomics {
  quotes: {
    USD: {
      price: number;
      market_cap: number;
      volume_24h: number;
    };
  };
}

type ObservatoryPage = {
  page: number;
  size: number;
  total: number;
  items: Array<{ rewarding_details?: { unique_delegations?: number } }>;
};

const jsonHeaders = {
  Accept: 'application/json',
  'Content-Type': 'application/json; charset=utf-8',
};

export async function fetchPacketsAndStaking(url: string): Promise<PacketsAndStakingPoint[]> {
  const response = await fetch(url, { headers: jsonHeaders });
  if (!response.ok) {
    throw new Error('Failed to fetch mixnode stats');
  }
  const data: PacketsAndStakingPoint[] = await response.json();
  return data;
}

export async function fetchCurrentEpoch(url: string): Promise<CurrentEpochWithEnd> {
  const response = await fetch(url, { headers: jsonHeaders, cache: 'no-store' });
  if (!response.ok) {
    throw new Error('Failed to fetch current epoch');
  }
  const data: CurrentEpochApiData = await response.json();
  const current_epoch_end = addSeconds(new Date(data.current_epoch_start), data.epoch_length.secs).toISOString();
  return { ...data, current_epoch_end };
}

export async function fetchEpochRewards(url: string): Promise<EpochRewardsData> {
  const response = await fetch(url, { headers: jsonHeaders, cache: 'no-store' });
  if (!response.ok) {
    throw new Error('Failed to fetch epoch rewards');
  }
  return response.json();
}

export async function fetchNymPrice(url: string): Promise<NymTokenomics> {
  const response = await fetch(url, { headers: jsonHeaders });
  if (!response.ok) {
    throw new Error('Failed to fetch NYM price');
  }
  return response.json();
}

const nymPriceInflight = new Map<string, Promise<NymTokenomics>>();

/** Coalesces concurrent requests for the same price URL (e.g. Balance card + Network overview). */
export function fetchNymPriceDeduped(url: string): Promise<NymTokenomics> {
  const existing = nymPriceInflight.get(url);
  if (existing) {
    return existing;
  }
  const pending = fetchNymPrice(url).finally(() => {
    nymPriceInflight.delete(url);
  });
  nymPriceInflight.set(url, pending);
  return pending;
}

/** Paginates observatory nodes and sums unique_delegations (same idea as explorer StakersNumberCard). */
export async function fetchTotalDelegationsCount(baseUrl: string): Promise<number> {
  const all: ObservatoryPage['items'] = [];
  let page = 0;
  const PAGE_SIZE = 200;
  let hasMore = true;

  while (hasMore) {
    /* Paginated API: each request needs the previous page index. */
    // eslint-disable-next-line no-await-in-loop -- sequential pagination
    const response = await fetch(`${baseUrl}?page=${page}&size=${PAGE_SIZE}`, { headers: jsonHeaders });
    if (!response.ok) {
      throw new Error(`Failed to fetch observatory nodes (page ${page})`);
    }
    // eslint-disable-next-line no-await-in-loop -- follows fetch above
    const data: ObservatoryPage = await response.json();
    all.push(...data.items);
    const totalPages = Math.ceil(data.total / PAGE_SIZE);
    if (page >= totalPages - 1 || data.items.length < PAGE_SIZE) {
      hasMore = false;
    } else {
      page += 1;
    }
  }

  return all.reduce((sum, node) => sum + (node.rewarding_details?.unique_delegations ?? 0), 0);
}

export function formatCompactNumber(num: number): string {
  return new Intl.NumberFormat('en-US', {
    maximumFractionDigits: 2,
    notation: 'compact',
    compactDisplay: 'short',
  }).format(num);
}
