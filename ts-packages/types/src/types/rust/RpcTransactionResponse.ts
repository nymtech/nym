import type { DecCoin } from './DecCoin';
import type { Gas } from './Gas';

export interface RpcTransactionResponse {
  index: number;
  tx_result_json: string;
  block_height: bigint;
  transaction_hash: string;
  gas_used: Gas;
  gas_wanted: Gas;
  fee: DecCoin | null;
}
