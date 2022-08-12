import type { DecCoin } from "./DecCoin";

export interface Delegation { owner: string, mix_id: number, amount: DecCoin, height: bigint, proxy: string | null, }