import type { IntervalRewardParams } from './IntervalRewardParams';

export interface RewardingParams {
  interval: IntervalRewardParams;
  rewarded_set_size: number;
  active_set_size: number;
}
