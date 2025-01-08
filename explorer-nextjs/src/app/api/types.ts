export type API_RESPONSE<T> = {
  data: T[];
};

export type NodeDescription = {
  last_polled: string;
  host_information: {
    ip_address: string[];
    hostname: string;
    keys: {
      ed25519: string;
      x25519: string;
      x25519_noise: string | null;
    };
  };
  declared_role: {
    mixnode: boolean;
    entry: boolean;
    exit_nr: boolean;
    exit_ipr: boolean;
  };
  auxiliary_details: {
    location: string;
    announce_ports: {
      verloc_port: number | null;
      mix_port: number | null;
    };
    accepted_operator_terms_and_conditions: boolean;
  };
  build_information: {
    binary_name: string;
    build_timestamp: string;
    build_version: string;
    commit_sha: string;
    commit_timestamp: string;
    commit_branch: string;
    rustc_version: string;
    rustc_channel: string;
    cargo_profile: string;
    cargo_triple: string;
  };
  network_requester: {
    address: string;
    uses_exit_policy: boolean;
  };
  ip_packet_router: {
    address: string;
  };
  authenticator: {
    address: string;
  };
  wireguard: string | null;
  mixnet_websockets: {
    ws_port: number;
    wss_port: number | null;
  };
} | null;

export type BondInformation = {
  node_id: number;
  owner: string;
  original_pledge: {
    denom: string;
    amount: string;
  };
  bonding_height: number;
  is_unbonding: boolean;
  node: {
    host: string;
    custom_http_port: number;
    identity_key: string;
  };
};

export type RewardingDetails = {
  cost_params: {
    profit_margin_percent: string;
    interval_operating_cost: {
      denom: string;
      amount: string;
    };
  };
  operator: string;
  delegates: string;
  total_unit_reward: string;
  unit_delegation: string;
  last_rewarded_epoch: number;
  unique_delegations: number;
};

export type Location = {
  two_letter_iso_country_code?: string;
  three_letter_iso_country_code?: string;
  country_name?: string;
  latitude?: number;
  longitude?: number;
};

type NodeData = {
  node_id: number;
  contract_node_type: string;
  description: NodeDescription;
  bond_information: BondInformation;
  rewarding_details: RewardingDetails;
  location: Location;
};

export default NodeData;

export interface CurrencyRates {
  btc: number;
  chf: number;
  eur: number;
  timestamp: number;
  usd: number;
}

// ACCOUNT BALANCES

export interface IRewardDetails {
  amount_staked: IAmountDetails;
  node_id: number;
  node_still_fully_bonded: boolean;
  rewards: IAmountDetails;
}

export interface IAmountDetails {
  denom: string;
  amount: string;
}

export interface IDelegationDetails {
  node_id: number;
  delegated: IAmountDetails;
  height: number;
  proxy: null | string;
}

export interface IAccountBalancesInfo {
  accumulated_rewards: IRewardDetails[];
  address: string;
  balances: IAmountDetails[];
  claimable_rewards: IAmountDetails;
  delegations: IDelegationDetails[];
  operator_rewards?: null | IAmountDetails;
  total_delegations: IAmountDetails;
  total_value: IAmountDetails;
  vesting_account?: null | string;
}

export interface IObservatoryNode {
  accepted_tnc: boolean;
  bonded: boolean;
  bonding_address: string;
  description: {
    authenticator: {
      address: string;
    };
    auxiliary_details: {
      accepted_operator_terms_and_conditions: boolean;
      announce_ports: {
        mix_port: number | null;
        verloc_port: number | null;
      };
      location: string | null;
    };
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
    };
    keys: {
      ed25519: string;
      x25519: string;
      x25519_noise: string | null;
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
    wireguard: string | null;
    geoip: {
      city: string;
      country: string;
      ip_address: string;
      loc: string;
      node_id: number;
      org: string;
      postal: string;
      region: string;
    };
  };
  identity_key: string;
  ip_address: string;
  node_id: number;
  node_type: string;
  original_pledge: number;
  rewarding_details: {
    cost_params: {
      interval_operating_cost: {
        amount: string;
        denom: string;
      };
      profit_margin_percent: string;
    };
    delegates: string;
    last_rewarded_epoch: number;
    operator: string;
    total_unit_reward: string;
    unique_delegations: number;
    unit_delegation: string;
  };
  self_description: {
    details: string;
    moniker: string;
    security_contact: string;
    website: string;
  };
  total_stake: number;
  uptime: number;
}
export interface NodeRewardDetails {
  amount: {
    amount: string;
    denom: string;
  };
  cumulative_reward_ratio: string;
  height: number;
  node_id: number;
  owner: string;
}
