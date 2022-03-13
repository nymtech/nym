import { invoke } from '@tauri-apps/api';
import { Account, TCreateAccount } from '../types';

export const createAccount = async (): Promise<TCreateAccount> => {
  const res: TCreateAccount = await invoke('create_new_account');
  return res;
};

export const signInWithMnemonic = async (mnemonic: string): Promise<Account> => {
  const res: Account = await invoke('connect_with_mnemonic', { mnemonic });
  return res;
};

export const signOut = async (): Promise<void> => {
  await invoke('logout');
};
