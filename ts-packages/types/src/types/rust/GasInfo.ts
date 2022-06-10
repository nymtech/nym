import type { MajorCurrencyAmount } from './Currency';

export interface GasInfo {
  gas_wanted: bigint;
  gas_used: bigint;
  fee: MajorCurrencyAmount;
}
