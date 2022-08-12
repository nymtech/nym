import type { DecCoin } from "./DecCoin";
import type { DelegationEventKind } from "./DelegationEventKind";

export interface DelegationEvent { kind: DelegationEventKind, mix_id: number, address: string, amount: DecCoin | null, proxy: string | null, }