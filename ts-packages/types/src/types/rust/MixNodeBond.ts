import type { DecCoin } from "./DecCoin";
import type { MixNode } from "./Mixnode";

export interface MixNodeBond { pledge_amount: DecCoin, total_delegation: DecCoin, owner: string, layer: string, block_height: bigint, mix_node: MixNode, proxy: string | null, accumulated_rewards: DecCoin | null, }