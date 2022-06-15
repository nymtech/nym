import type { MajorCurrencyAmount } from './Currency';

export interface TransactionDetails {
  amount: MajorCurrencyAmount;
  from_address: string;
  to_address: string;
}
