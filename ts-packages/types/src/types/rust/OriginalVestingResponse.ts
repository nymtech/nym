import type { MajorCurrencyAmount } from './Currency';

export interface OriginalVestingResponse {
  amount: MajorCurrencyAmount;
  number_of_periods: number;
  period_duration: bigint;
}
