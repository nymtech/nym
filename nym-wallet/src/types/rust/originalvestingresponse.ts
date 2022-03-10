import type { Coin } from './coin';

export interface OriginalVestingResponse {
  amount: Coin;
  number_of_periods: number;
  period_duration: bigint;
}
