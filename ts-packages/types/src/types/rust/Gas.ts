import type { MajorCurrencyAmount } from './Currency';

export interface Gas {
  gas_units: bigint;
  amount: MajorCurrencyAmount;
}
