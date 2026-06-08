import { Account } from '@nymproject/types';
import { signInAndNavigateToBalance } from './signInAndNavigateToBalance';

describe('signInAndNavigateToBalance', () => {
  it('does not navigate when loading the account fails', async () => {
    const navigate = jest.fn();
    const loadAccount = jest.fn(async () => undefined);
    const signInWithPassword = jest.fn(async () => ({ client_address: 'nym1fail' } as Account));
    const setLoginType = jest.fn();

    await expect(
      signInAndNavigateToBalance({
        type: 'password',
        value: 'secret',
        network: 'MAINNET',
        signInWithMnemonic: jest.fn(async () => ({ client_address: 'nym1mnemonic' } as Account)),
        signInWithPassword,
        loadAccount,
        setLoginType,
        navigate,
      }),
    ).rejects.toThrow('Unable to load account');

    expect(signInWithPassword).toHaveBeenCalledWith('secret');
    expect(loadAccount).toHaveBeenCalledWith('MAINNET');
    expect(setLoginType).not.toHaveBeenCalled();
    expect(navigate).not.toHaveBeenCalled();
  });

  it('navigates after the account loads successfully', async () => {
    const navigate = jest.fn();
    const loadAccount = jest.fn(async () => ({ client_address: 'nym1abc' } as Account));
    const signInWithMnemonic = jest.fn(async () => ({ client_address: 'nym1mnemonic' } as Account));
    const setLoginType = jest.fn();

    await expect(
      signInAndNavigateToBalance({
        type: 'mnemonic',
        value: 'mnemonic phrase',
        network: 'MAINNET',
        signInWithMnemonic,
        signInWithPassword: jest.fn(async () => ({ client_address: 'nym1password' } as Account)),
        loadAccount,
        setLoginType,
        navigate,
      }),
    ).resolves.toBeUndefined();

    expect(signInWithMnemonic).toHaveBeenCalledWith('mnemonic phrase');
    expect(loadAccount).toHaveBeenCalledWith('MAINNET');
    expect(setLoginType).toHaveBeenCalledWith('mnemonic');
    expect(navigate).toHaveBeenCalledWith('/balance');
  });
});
