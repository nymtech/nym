export type Epoch = {
  epoch_reward_pool: string;
  rewarded_set_size: string;
  active_set_size: string;
  staking_supply: string;
  sybil_resistance_percent: number;
  active_set_work_factor: number;
};

export type Node = {
  reward_blockstamp: number;
  uptime: string;
  in_active_set: boolean;
};

export type RewardParams = {
  epoch: Epoch;
  node: Node;
};

export type EstimatedReward = {
  estimated_total_node_reward: number;
  estimated_operator_reward: number;
  estimated_delegators_reward: number;
  estimated_node_profit: number;
  estimated_operator_cost: number;
  reward_params: RewardParams;
  as_at: number;
};

export type StakeSaturation = {
  saturation: number;
  as_at: number;
};

export type AvgUptime = {
  identity: string;
  avg_uptime: number;
};

export type InclusionProbability = {
  in_active: string;
  in_reserve: string;
};

export type Report = {
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
  identity: string;
  owner: string;
  history: History[];
};

export type CoreCount = {
  identity: string;
  count: number;
};

export type ActiveStatus = {
  status: string;
};