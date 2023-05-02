// TODO Add other transaction types later
type TTransactionType = 'send';

export type TTransaction = {
  type: TTransactionType;
  txHash?: string;
  status: 'loading' | 'success' | 'error';
  message?: string;
};
