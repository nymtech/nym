export interface IntervalRewardingParamsUpdate {
  reward_pool: string | null;
  staking_supply: string | null;
  sybil_resistance_percent: string | null;
  active_set_work_factor: string | null;
  interval_pool_emission: string | null;
  rewarded_set_size: number | null;
}
