import type { Coin } from "./Coin";

export interface CosmosFee { amount: Array<Coin>, gas_limit: bigint, payer: string | null, granter: string | null, }