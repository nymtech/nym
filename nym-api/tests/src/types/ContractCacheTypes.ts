export interface AllMixnodes {
  bond_information: BondInformation;
  rewarding_details: RewardingDetails;
}

export interface BondInformation {
  mix_id: number;
  owner: string;
  original_pledge: OriginalPledge;
  layer: number;
  mix_node: Mixnode;
  proxy: string;
  bonding_height: number;
  is_unbonding: boolean;
}

export interface RewardingDetails {
  cost_params: CostParams;
  operator: string;
  delegates: string;
  total_unit_reward: string;
  unit_delegation: string;
  last_rewarded_epoch: number;
  unique_delegations: number;
}

export interface CostParams {
  profit_margin_percent: string;
  interval_operating_cost: IntervalOperatingCost;
}

export interface IntervalOperatingCost {
  denom: string;
  amount: string;
}

export interface OriginalPledge {
  denom: string;
  amount: string;
}

export interface TotalDelegation {
  denom: string;
  amount: string;
}

export interface Mixnode {
  host: string;
  mix_port: number;
  verloc_port: number;
  http_api_port: number;
  sphinx_key: string;
  identity_key: string;
  version: string;
}

export interface MixnodeBond {
  pledge_amount: OriginalPledge;
  total_delegation: TotalDelegation;
  owner: string;
  layer: string;
  block_height: string;
  mix_node: Mixnode;
  proxy: string;
  accumulated_rewards: string;
}

export interface MixnodesDetailed {
  mixnode_details: AllMixnodes;
  stake_saturation: string;
  uncapped_stake_saturation: string;
  performance: string;
  estimated_operator_apy: string;
  estimated_delegators_apy: string;
  family: string;
}

export interface BlacklistedMixnodes {}

export interface BlacklistedGateways {}

export interface Gateway {
  host: string;
  mix_port: number;
  clients_port: number;
  location: string;
  sphinx_key: string;
  identity_key: string;
  version: string;
}

export interface AllGateways {
  pledge_amount: OriginalPledge;
  owner: string;
  block_height: number;
  gateway: Gateway;
  proxy: string;
}

export interface EpochRewardParams {
  interval: Interval;
  rewarded_set_size: number;
  active_set_size: number;
}

export interface Interval {
  reward_pool: string;
  staking_supply: string;
  staking_supply_scale_factor: string;
  epoch_reward_budget: string;
  stake_saturation_point: string;
  sybil_resistance: string;
  active_set_work_factor: string;
  interval_pool_emission: string;
}

export interface CurrentEpoch {
  id: number;
  epochs_in_interval: number;
  current_epoch_start: string;
  current_epoch_id: number;
  epoch_length: EpochLength;
  total_elapsed_epochs: number;
}

export interface EpochLength {
  secs: number;
  nanos: number;
}
