import type { MajorCurrencyAmount } from './Currency';

export interface PledgeData {
  amount: MajorCurrencyAmount;
  block_time: bigint;
}
