import type { DecCoin } from "./DecCoin";
import type { Gateway } from "./Gateway";

export interface GatewayBond { pledge_amount: DecCoin, owner: string, block_height: bigint, gateway: Gateway, proxy: string | null, }