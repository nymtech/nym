export interface CurrentEpoch {
  epoch_reward_pool: string;
  rewarded_set_size: string;
  active_set_size: string;
  staking_supply: string;
  sybil_resistance_percent: number;
  active_set_work_factor: number;
}

export interface Epoch {
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

export interface Node {
  reward_blockstamp: number;
  uptime: string;
  in_active_set: boolean;
}

export interface Estimation {
  total_node_reward: string;
  operator: string;
  delegates: string;
  operating_cost: string;
}

export interface RewardEstimation {
  estimation: Estimation;
  reward_params: RewardParams;
  epoch: Epoch;
  as_at: number;
}

export interface RewardParams {
  interval: Interval;
  rewarded_set_size: number;
  active_set_size: number;
}

export interface ComputeRewardEstimation {
  performance: string;
  active_in_rewarded_set: boolean;
  pledge_amount: number;
  total_delegation: number;
  interval_operating_cost: DenominationAndAmount;
  profit_margin_percent: string;
}

export interface DenominationAndAmount {
  denom: string;
  amount: string;
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

export interface StakeSaturation {
  saturation: string;
  uncapped_saturation: string;
  as_at: number;
}

export interface SingleInclusionProbability {
  in_active: number;
  in_reserve: number;
}

export interface InclusionProbability {
  mix_id: number;
  in_active: string;
  in_reserve: string;
}

export interface InclusionProbabilities {
  inclusion_probabilities: InclusionProbability[];
  samples: number;
  elapsed: Elapsed;
  delta_max: number;
  delta_l2: number;
  as_at: number;
}

export interface Elapsed {
  secs: number;
  nanos: number;
}

export interface Report {
  mix_id: number;
  identity: string;
  owner: string;
  most_recent: number;
  last_hour: number;
  last_day: number;
}

export interface History {
  date: string;
  uptime: number;
}

export interface NodeHistory {
  mix_id: number;
  identity: string;
  owner: string;
  history: History[];
}

export interface ErrorMsg {
  message: string;
}

export interface CoreCount {
  mix_id: number;
  identity: string;
  count: number;
}

export interface ActiveStatus {
  status: string;
}

export interface Gateway {
  host: string;
  mix_port: number;
  clients_port: number;
  location: string;
  sphinx_key: string;
  identity_key: string;
  version: string;
}

export interface GatewayBond {
  pledge_amount: DenominationAndAmount;
  owner: string;
  block_height: number;
  gateway: Gateway;
  proxy?: any;
}

export interface nodePerformance {
  most_recent: string;
  last_hour: string;
  last_24h: string;
}

export interface AvgUptime {
  mix_id: number;
  avg_uptime: number;
  node_performance: nodePerformance;
}

export interface GatewayUptimeResponse {
  identity: string;
  avg_uptime: number;
  node_performance: nodePerformance;
}

export interface DetailedGateway {
  gateway_bond: GatewayBond;
  performance: string;
  node_performance: nodePerformance;
}

export interface MixNode {
  host: string;
  mix_port: number;
  verloc_port: number;
  http_api_port: number;
  sphinx_key: string;
  identity_key: string;
  version: string;
}

export interface BondInformation {
  mix_id: number;
  owner: string;
  original_pledge: DenominationAndAmount;
  layer: string;
  mix_node: MixNode;
  proxy: string;
  bonding_height: number;
  is_unbonding: boolean;
}

export interface CostParams {
  profit_margin_percent: string;
  interval_operating_cost: DenominationAndAmount;
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

export interface MixnodeDetails {
  bond_information: BondInformation;
  rewarding_details: RewardingDetails;
}

export interface DetailedMixnodes {
  mixnode_details: MixnodeDetails;
  stake_saturation: string;
  uncapped_stake_saturation: string;
  performance: string;
  estimated_operator_apy: string;
  estimated_delegators_apy: string;
  family: string;
}
