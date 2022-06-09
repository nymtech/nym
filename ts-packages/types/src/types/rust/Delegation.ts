import type { MajorCurrencyAmount } from './Currency';

export interface Delegation {
  owner: string;
  node_identity: string;
  amount: MajorCurrencyAmount;
  block_height: bigint;
  proxy: string | null;
}
