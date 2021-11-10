import { TransactionDetails } from "./transactiondetails";

export interface TauriTxResult {
  code: number;
  gas_wanted: bigint;
  gas_used: bigint;
  block_height: bigint;
  details: TransactionDetails;
}