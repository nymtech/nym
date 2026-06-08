import { getNodeSettingsUpdateErrorMessage } from './nodeSettingsUpdateError';

describe('getNodeSettingsUpdateErrorMessage', () => {
  it('returns undefined when the transaction succeeded', () => {
    expect(
      getNodeSettingsUpdateErrorMessage({
        transaction_hash: 'abc123',
        logs_json: '[]',
        msg_responses_json: '[]',
        gas_info: {
          gas_wanted: { gas_units: BigInt(1) },
          gas_used: { gas_units: BigInt(1) },
        },
        fee: { amount: '1', denom: 'nym' },
      }),
    ).toBeUndefined();
  });

  it('returns context error when provided', () => {
    expect(getNodeSettingsUpdateErrorMessage(undefined, 'an error occurred: insufficient funds')).toBe(
      'an error occurred: insufficient funds',
    );
  });

  it('returns a generic message when the update failed without context error', () => {
    expect(getNodeSettingsUpdateErrorMessage(undefined)).toBe(
      'Unable to update node settings. Check your balance and try again.',
    );
  });

  it('returns an error message when transaction_hash is an empty string', () => {
    expect(
      getNodeSettingsUpdateErrorMessage({
        transaction_hash: '',
        logs_json: '[]',
        msg_responses_json: '[]',
        gas_info: {
          gas_wanted: { gas_units: BigInt(1) },
          gas_used: { gas_units: BigInt(1) },
        },
        fee: { amount: '1', denom: 'nym' },
      }),
    ).toBe('Unable to update node settings. Check your balance and try again.');
  });

  it('returns an error when gas_used is zero despite a non-empty hash', () => {
    expect(
      getNodeSettingsUpdateErrorMessage({
        transaction_hash: 'abc123',
        logs_json: '[]',
        msg_responses_json: '[]',
        gas_info: {
          gas_wanted: { gas_units: BigInt(1) },
          gas_used: { gas_units: BigInt(0) },
        },
        fee: { amount: '1', denom: 'nym' },
      }),
    ).toBe('Unable to update node settings. Check your balance and try again.');
  });
});
