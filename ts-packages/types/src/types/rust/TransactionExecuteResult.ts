import type { DecCoin } from './DecCoin';
import type { GasInfo } from './GasInfo';

export interface TransactionExecuteResult {
  logs_json: string;
  data_json: string;
  transaction_hash: string;
  gas_info: GasInfo;
  fee: DecCoin | null;
}
