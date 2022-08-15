import type { DecCoin } from './DecCoin';
import type { Gas } from './Gas';
import type { TransactionDetails } from './TransactionDetails';

export interface SendTxResult {
  block_height: bigint;
  code: number;
  details: TransactionDetails;
  gas_used: Gas;
  gas_wanted: Gas;
  tx_hash: string;
  fee: DecCoin | null;
}
