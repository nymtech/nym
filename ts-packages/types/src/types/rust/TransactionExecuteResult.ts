import type { GasInfo } from './GasInfo';
import type { MajorCurrencyAmount } from './Currency';

export interface TransactionExecuteResult {
  logs_json: string;
  data_json: string;
  transaction_hash: string;
  gas_info: GasInfo;
  fee: MajorCurrencyAmount;
}
