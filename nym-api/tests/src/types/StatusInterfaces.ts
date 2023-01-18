export type CurrentEpoch = {
  epoch_reward_pool: string;
  rewarded_set_size: string;
  active_set_size: string;
  staking_supply: string;
  sybil_resistance_percent: number;
  active_set_work_factor: number;
};


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

export type Node = {
  reward_blockstamp: number;
  uptime: string;
  in_active_set: boolean;
};

export interface RewardEstimation {
  estimation: Estimation;
  reward_params: RewardParams;
  epoch: Epoch;
  as_at: number;
}

export interface Estimation {
  total_node_reward: string;
  operator: string;
  delegates: string;
  operating_cost: string;
}

export interface RewardParams {
  interval: Interval;
  rewarded_set_size: number;
  active_set_size: number;
}

export interface ComputeRewardEstimation {
  performance: string,
  active_in_rewarded_set: boolean,
  pledge_amount: number,
  total_delegation: number,
  interval_operating_cost: IntervalOperatingCost;
  profit_margin_percent: string
}

export type IntervalOperatingCost = {
  denom: string,
  amount: string
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

export type StakeSaturation = {
  saturation: string;
  uncapped_saturation: string;
  as_at: number;
};

export type AvgUptime = {
  mix_id: number;
  avg_uptime: number;
};

export interface SingleInclusionProbability {
  in_active: number;
  in_reserve: number;
}

export type InclusionProbability = {
  mix_id: number;
  in_active: string;
  in_reserve: string;
};

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

export type Report = {
  mix_id: number;
  identity: string;
  owner: string;
  most_recent: number;
  last_hour: number;
  last_day: number;
};

export type History = {
  date: string;
  uptime: number;
};

export type NodeHistory = {
  mix_id: number,
  identity: string;
  owner: string;
  history: History[];
};

export type CoreCount = {
  mix_id: number,
  identity: string;
  count: number;
};

export type ActiveStatus = {
  status: string;
};

export type DetailedGateway = {
  owner: string;

}
