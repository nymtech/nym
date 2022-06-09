import type { DelegationEventKind } from './DelegationEventKind';
import type { MajorCurrencyAmount } from './Currency';

export interface DelegationEvent {
  kind: DelegationEventKind;
  node_identity: string;
  address: string;
  amount: MajorCurrencyAmount | null;
  block_height: bigint;
}
