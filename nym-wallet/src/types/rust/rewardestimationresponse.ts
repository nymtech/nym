export interface RewardEstimationResponse {
  estimated_total_node_reward: bigint;
  estimated_operator_reward: bigint;
  estimated_delegators_reward: bigint;
  current_interval_start: bigint;
  current_interval_end: bigint;
  current_interval_uptime: number;
  as_at: bigint;
}
