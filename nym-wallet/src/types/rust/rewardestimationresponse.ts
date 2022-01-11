export interface RewardEstimationResponse {
  estimated_total_node_reward: bigint;
  estimated_operator_reward: bigint;
  estimated_delegators_reward: bigint;
  current_epoch_start: bigint;
  current_epoch_end: bigint;
  current_epoch_uptime: number;
  as_at: bigint;
}