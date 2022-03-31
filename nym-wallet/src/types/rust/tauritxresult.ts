import type { TransactionDetails } from './transactiondetails';

export interface TauriTxResult {
  block_height: bigint;
  code: number;
  details: TransactionDetails;
  gas_used: bigint;
  gas_wanted: bigint;
  tx_hash: string;
}
