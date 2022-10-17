import type { Interval } from './Interval';
import type { RewardEstimate } from './RewardEstimate';
import type { RewardingParams } from './RewardingParams';

export interface RewardEstimationResponse {
  estimation: RewardEstimate;
  reward_params: RewardingParams;
  epoch: Interval;
  as_at: number;
}
