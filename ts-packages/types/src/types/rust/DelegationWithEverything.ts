import type { DecCoin } from './DecCoin';
import type { DelegationEvent } from './DelegationEvent';
import type { DelegationRecord } from './DelegationRecord';

export interface DelegationWithEverything {
  owner: string;
  node_identity: string;
  amount: DecCoin;
  total_delegation: DecCoin | null;
  pledge_amount: DecCoin | null;
  block_height: bigint;
  delegated_on_iso_datetime: string;
  profit_margin_percent: number | null;
  avg_uptime_percent: number | null;
  stake_saturation: number | null;
  uses_vesting_contract_tokens: boolean;
  accumulated_rewards: DecCoin | null;
  pending_events: Array<DelegationEvent>;
  history: Array<DelegationRecord>;
}
