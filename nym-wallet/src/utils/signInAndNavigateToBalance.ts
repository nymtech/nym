import { Account } from '@nymproject/types';
import { Network } from 'src/types';

export type SignInType = 'mnemonic' | 'password';

export type SignInAndNavigateToBalanceDeps = {
  type: SignInType;
  value: string;
  network: Network;
  signInWithMnemonic: (mnemonic: string) => Promise<Account>;
  signInWithPassword: (password: string) => Promise<Account>;
  loadAccount: (network: Network) => Promise<Account | undefined>;
  setLoginType: (loginType: SignInType) => void;
  navigate: (path: string) => void;
};

export async function signInAndNavigateToBalance({
  type,
  value,
  network,
  signInWithMnemonic,
  signInWithPassword,
  loadAccount,
  setLoginType,
  navigate,
}: SignInAndNavigateToBalanceDeps): Promise<void> {
  if (value.length === 0) {
    throw new Error(`A ${type} must be provided`);
  }

  if (type === 'mnemonic') {
    await signInWithMnemonic(value);
  } else {
    await signInWithPassword(value);
  }

  const client = await loadAccount(network);
  if (!client?.client_address) {
    throw new Error('Unable to load account');
  }

  setLoginType(type);
  navigate('/balance');
}
