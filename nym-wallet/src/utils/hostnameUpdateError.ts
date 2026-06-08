import { TransactionExecuteResult } from '@nymproject/types';

export const getHostnameUpdateErrorMessage = (
  tx: TransactionExecuteResult | undefined,
  contextError?: string,
): string | undefined => {
  if (tx?.transaction_hash) {
    return undefined;
  }
  if (contextError) {
    return contextError;
  }
  return 'Unable to update node settings. Check your balance and try again.';
};
