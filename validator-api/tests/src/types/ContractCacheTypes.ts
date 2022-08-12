export type AllMixnodes = {
  pledge_amount: PledgeAmount;
  total_delegation: TotalDelegation;
  owner: string;
  layer: string;
  block_height: number;
  mix_node: Mixnode;
  proxy: string;
  accumulated_rewards: string;
};

export type PledgeAmount = {
  denom: string;
  amount: string;
};

export type TotalDelegation = {
  denom: string;
  amount: string;
};

export type Mixnode = {
  host: string;
  mix_port: number;
  verloc_port: number;
  http_api_port: number;
  sphinx_key: string;
  identity_key: string;
  version: string;
  profit_margin_percent: number;
};

export type MixnodesDetailed = {
  mixnode_bond: AllMixnodes;
  stake_saturation: number;
  uptime: number;
  estimated_operator_apy: number;
  estimated_delegators_apy: number;
};

export type AllGateways = {
  pledge_amount: PledgeAmount;
  owner: string;
  block_height: number;
  gateway: Gateway;
  proxy: string;
};

export type Gateway = {
  host: string;
  mix_port: number;
  clients_port: number;
  location: string;
  sphinx_key: string;
  identity_key: string;
  version: string;
};

export type BlacklistedMixnodes = {
};

export type BlacklistedGateways = {
};

export type EpochRewardParams = {
  epoch_reward_pool: string;
  rewarded_set_size: string;
  active_set_size: string;
  staking_supply: string;
  sybil_resistance_percent: number;
  active_set_work_factor: number;
};

export type CurrentEpoch = {
  id: number;
  start: string;
  length: Length;
};

export type Length = {
  secs: number;
  nanos: number;
};

