import type { MajorCurrencyAmount } from './Currency';
import type { MixNode } from './Mixnode';

export interface MixNodeBond {
  pledge_amount: MajorCurrencyAmount;
  total_delegation: MajorCurrencyAmount;
  owner: string;
  layer: string;
  block_height: bigint;
  mix_node: MixNode;
  proxy: string | null;
  accumulated_rewards: MajorCurrencyAmount | null;
}
