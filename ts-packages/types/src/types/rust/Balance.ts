import type { MajorCurrencyAmount } from './Currency';

export interface Balance {
  amount: MajorCurrencyAmount;
  printable_balance: string;
}
