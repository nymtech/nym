import type { MajorCurrencyAmount } from './Currency';

export interface DelegationRecord {
  amount: MajorCurrencyAmount;
  block_height: bigint;
  delegated_on_iso_datetime: string;
  uses_vesting_contract_tokens: boolean;
}
