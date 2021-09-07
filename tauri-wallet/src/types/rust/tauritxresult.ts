import { TransactionDetails } from "./transactiondetails";

export interface TauriTxResult {
  code: number;
  gas_wanted: number;
  gas_used: number;
  block_height: number;
  details: TransactionDetails;
}