export type RewardEstimationResponse = {
  estimated_total_node_reward: number;
  estimated_operator_reward: number;
  estimated_delegators_reward: number;
  estimated_node_profit: number;
  estimated_operator_cost: number;
  as_at: number;
};
