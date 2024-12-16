/* eslint-disable @typescript-eslint/no-explicit-any */
import {
  CIRCULATING_NYM_SUPPLY,
  HARBOURMASTER_API_MIXNODES_STATS,
  HARBOURMASTER_API_SUMMARY,
  NYM_NODE_DESCRIPTION,
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

export interface IBondInfo {
  bond_information: {
    bonding_height: number;
    is_unbonding: boolean;
    node: {
      custom_http_port: number;
      host: string;
      identity_key: string;
    };
    node_id: number;
    original_pledge: {
      amount: string;
      denom: string;
    };
    owner: string;
  };
  rewarding_details: {
    cost_params: {
      profit_margin_percent: string;
      interval_operating_cost: {
        denom: string;
        amount: string;
      };
    };
    delegates: string;
    last_rewarded_epoch: number;
    operator: string;
    total_unit_reward: string;
    unique_delegations: number;
    unit_delegation: string;
  };
}

export interface INodeDescription {
  contract_node_type: string;
  description: {
    authenticator: object;
    address: string;
    auxiliary_details: object;
    accepted_operator_terms_and_conditions: boolean;
    announce_ports: {
      verloc_port: number | null;
      mix_port: number | null;
    };
    location: string;
    build_information: {
      binary_name: string;
      build_timestamp: string;
      build_version: string;
      cargo_profile: string;
      cargo_triple: string;
      commit_branch: string;
      commit_sha: string;
      commit_timestamp: string;
      rustc_channel: string;
      rustc_version: string;
    };
    declared_role: {
      entry: boolean;
      exit_ipr: boolean;
      exit_nr: boolean;
      mixnode: boolean;
    };
    host_information: {
      hostname: string | null;
      ip_address: string[];
      keys: {
        ed25519: string;
        x25519: string;
        x25519_noise: string | null;
      };
    };
    ip_packet_router: {
      address: string;
    };
    last_polled: string;
    mixnet_websockets: {
      ws_port: number;
      wss_port: number | null;
    };
    network_requester: {
      address: string;
      uses_exit_policy: boolean;
    };
    wireguard: null | object;
  };
  node_id: number;
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
