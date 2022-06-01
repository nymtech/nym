import type { DecCoin } from "./DecCoin";

export interface DelegationRecord { amount: DecCoin, block_height: bigint, delegated_on_iso_datetime: string, }