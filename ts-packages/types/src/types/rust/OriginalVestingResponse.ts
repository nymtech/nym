import type { DecCoin } from './DecCoin';

export interface OriginalVestingResponse {
  amount: DecCoin;
  number_of_periods: number;
  period_duration: bigint;
}
