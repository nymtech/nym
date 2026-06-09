import { TransactionExecuteResult } from '@nymproject/types';

const parseJsonArray = (raw: string): unknown[] | null => {
  const trimmed = raw.trim();
  if (!trimmed) {
    return [];
  }
  try {
    const parsed = JSON.parse(trimmed);
    return Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
};

/**
 * Rust IPC only returns `TransactionExecuteResult` after DeliverTx succeeds (code 0).
 * This helper centralizes the TS-side interpretation: non-empty hash plus structurally
 * valid execution metadata, not hash presence alone.
 */
export const isTransactionExecuteSuccessful = (tx: TransactionExecuteResult | undefined): boolean => {
  if (!tx?.transaction_hash || tx.transaction_hash.length === 0) {
    return false;
  }

  const gasUsed = tx.gas_info?.gas_used?.gas_units;
  if (gasUsed === undefined || gasUsed <= 0n) {
    return false;
  }

  if (parseJsonArray(tx.msg_responses_json) === null) {
    return false;
  }

  if (parseJsonArray(tx.logs_json) === null) {
    return false;
  }

  return true;
};
