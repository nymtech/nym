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
};

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
