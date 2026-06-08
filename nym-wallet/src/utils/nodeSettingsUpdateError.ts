import { TransactionExecuteResult } from '@nymproject/types';
import { isTransactionExecuteSuccessful } from './transactionExecuteSuccess';

export const getNodeSettingsUpdateErrorMessage = (
  tx: TransactionExecuteResult | undefined,
  contextError?: string,
): string | undefined => {
  if (isTransactionExecuteSuccessful(tx)) {
    return undefined;
  }
  if (contextError) {
    return contextError;
  }
  return 'Unable to update node settings. Check your balance and try again.';
};
