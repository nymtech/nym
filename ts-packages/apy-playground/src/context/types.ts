export interface RewardParams {
  epoch_reward_pool: string;
  rewarded_set_size: string;
  active_set_size: string;
  staking_supply: string;
  sybil_resistance_percent: number;
  active_set_work_factor: number;
}

export interface RewardEstimationParams {
  uptime?: number;
  is_active?: boolean;
  pledge_amount?: number;
  total_delegation?: number;
}

export interface RewardEstimationParamsForSliders {
  uptime: number;
  is_active: boolean;
  pledge_amount: number;
  total_delegation: number;
}

export interface RewardEstimation {
  estimated_total_node_reward: number;
  estimated_operator_reward: number;
  estimated_delegators_reward: number;
  estimated_node_profit: number;
  estimated_operator_cost: number;
  reward_params: any;
  as_at: number;
}

export interface RewardEstimationWithAPY {
  estimated_total_node_reward: number;
  estimated_operator_reward: number;
  estimated_delegators_reward: number;
  estimated_node_profit: number;
  estimated_operator_cost: number;
  reward_params: any;
  as_at: number;
  estimates: {
    majorAmountToUseInCalcs: number;
    nodeApy: number;
    operator: {
      apy: number;
      rewardMajorAmount: {
        daily: number;
        monthly: number;
        yearly: number;
      };
    };
    delegator: {
      apy: number;
      rewardMajorAmount: {
        daily: number;
        monthly: number;
        yearly: number;
      };
    };
  };
}

export interface MixNodeBondWithDetails {
  mixnode_bond: {
    pledge_amount: {
      denom: string;
      amount: string;
    };
    total_delegation: {
      denom: string;
      amount: string;
    };
    owner: string;
    layer: number;
    block_height: number;
    mix_node: {
      host: string;
      mix_port: number;
      verloc_port: number;
      http_api_port: number;
      sphinx_key: string;
      identity_key: string;
      version: string;
      profit_margin_percent: number;
    };
    proxy: null;
    accumulated_rewards: string;
  };
  stake_saturation: number;
  uptime: number;
  estimated_operator_apy: number;
  estimated_delegators_apy: number;
  status?: string;
  inclusion_probability?: InclusionProbability;
}

export interface InclusionProbability {
  in_active: string;
  in_reserve: string;
}
