import type { DecCoin } from "./DecCoin";

export interface DelegationResult { source_address: string, target_address: string, amount: DecCoin | null, }