import { RewardEstimate } from './RewardEstimate';
import { RewardingParams } from './RewardingParams';

export type RewardEstimationResponse = {
  estimation: RewardEstimate;
  reward_params: RewardingParams;
  // epoch: Interval;
  as_at: number;
};
