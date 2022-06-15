import type { DelegationEvent } from './DelegationEvent';
import type { DelegationRecord } from './DelegationRecord';
import type { MajorCurrencyAmount } from './Currency';

export interface DelegationWithEverything {
  owner: string;
  node_identity: string;
  amount: MajorCurrencyAmount;
  total_delegation: MajorCurrencyAmount | null;
  pledge_amount: MajorCurrencyAmount | null;
  block_height: bigint;
  delegated_on_iso_datetime: string;
  profit_margin_percent: number | null;
  avg_uptime_percent: number | null;
  stake_saturation: number | null;
  uses_vesting_contract_tokens: boolean;
  accumulated_rewards: MajorCurrencyAmount | null;
  pending_events: Array<DelegationEvent>;
  history: Array<DelegationRecord>;
}
