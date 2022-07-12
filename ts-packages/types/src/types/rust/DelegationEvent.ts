import type { DecCoin } from "./DecCoin";
import type { DelegationEventKind } from "./DelegationEventKind";

export interface DelegationEvent {
  kind: DelegationEventKind;
  node_identity: string;
  address: string;
  amount: DecCoin | null;
  block_height: bigint;
  proxy: string | null;
}
