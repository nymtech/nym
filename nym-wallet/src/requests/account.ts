import { invoke } from '@tauri-apps/api';
import { Account } from '../types';

export const createMnemonic = async (): Promise<string> => invoke('create_mnemonic');

export const signInWithMnemonic = async (mnemonic: string): Promise<Account> => {
  const res: Account = await invoke('connect_with_mnemonic', { mnemonic });
  return res;
};

export const signOut = async () => invoke('logout');
