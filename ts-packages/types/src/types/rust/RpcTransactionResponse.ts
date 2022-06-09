import type { GasInfo } from './GasInfo';
import type { MajorCurrencyAmount } from './Currency';

export interface RpcTransactionResponse {
  index: number;
  tx_result_json: string;
  block_height: bigint;
  transaction_hash: string;
  gas_info: GasInfo;
  fee: MajorCurrencyAmount;
}
