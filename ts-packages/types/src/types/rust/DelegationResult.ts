import type { MajorCurrencyAmount } from './Currency';

export interface DelegationResult {
  source_address: string;
  target_address: string;
  amount: MajorCurrencyAmount | null;
}
