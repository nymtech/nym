import { TransactionExecuteResult } from '@nymproject/types';
import { isTransactionExecuteSuccessful } from './transactionExecuteSuccess';

const baseTx = {
  transaction_hash: 'abc123',
  logs_json: '[]',
  msg_responses_json: '[]',
  gas_info: {
    gas_wanted: { gas_units: BigInt(100) },
    gas_used: { gas_units: BigInt(50) },
  },
  fee: { amount: '1', denom: 'nym' },
} as TransactionExecuteResult;

describe('isTransactionExecuteSuccessful', () => {
  it('returns true for a structurally valid execution result', () => {
    expect(isTransactionExecuteSuccessful(baseTx)).toBe(true);
  });

  it('returns true when log and response JSON are empty strings', () => {
    expect(
      isTransactionExecuteSuccessful({
        ...baseTx,
        logs_json: '',
        msg_responses_json: '',
      }),
    ).toBe(true);
  });

  it('returns false when transaction_hash is empty', () => {
    expect(isTransactionExecuteSuccessful({ ...baseTx, transaction_hash: '' })).toBe(false);
  });

  it('returns false when gas_used is zero', () => {
    expect(
      isTransactionExecuteSuccessful({
        ...baseTx,
        gas_info: {
          gas_wanted: { gas_units: BigInt(100) },
          gas_used: { gas_units: BigInt(0) },
        },
      }),
    ).toBe(false);
  });

  it('returns false when msg_responses_json is not valid JSON', () => {
    expect(isTransactionExecuteSuccessful({ ...baseTx, msg_responses_json: 'not-json' })).toBe(false);
  });

  it('returns false when logs_json is not a JSON array', () => {
    expect(isTransactionExecuteSuccessful({ ...baseTx, logs_json: '{"error":true}' })).toBe(false);
  });
});
