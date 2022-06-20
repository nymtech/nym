import type { GasInfo } from './GasInfo';

export interface RpcTransactionResponse {
  index: number;
  tx_result_json: string;
  block_height: bigint;
  transaction_hash: string;
  gas_info: GasInfo;
}
