import type { CurrencyDenom } from './CurrencyDenom';
import type { MajorAmountString } from './CurrencyStringMajorAmount';

export interface MajorCurrencyAmount {
  amount: MajorAmountString;
  denom: CurrencyDenom;
}
