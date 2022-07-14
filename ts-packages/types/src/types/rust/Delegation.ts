import type { DecCoin } from './DecCoin';

export interface Delegation {
  owner: string;
  node_identity: string;
  amount: DecCoin;
  block_height: bigint;
  proxy: string | null;
}
