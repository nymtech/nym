import type { DecCoin } from './DecCoin';
import type { MixNode } from './Mixnode';
export interface MixNodeBond {
    mix_id: number;
    owner: string;
    original_pledge: DecCoin;
    layer: string;
    mix_node: MixNode;
    proxy: string | null;
    bonding_height: bigint;
    is_unbonding: boolean;
}
//# sourceMappingURL=MixNodeBond.d.ts.map