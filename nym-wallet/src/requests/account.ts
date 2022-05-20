import { invoke } from '@tauri-apps/api';
import { AccountEntry } from 'src/types/rust/accountentry';
import { Account } from '../types';

export const createMnemonic = async () => {
  const res: string = await invoke('create_new_mnemonic');
  return res;
};

export const createPassword = async ({ mnemonic, password }: { mnemonic: string; password: string }) => {
  await invoke('create_password', { mnemonic, password });
};

export const signInWithMnemonic = async (mnemonic: string) => {
  const res: Account = await invoke('connect_with_mnemonic', { mnemonic });
  return res;
};

export const signInWithPassword = async (password: string) => {
  const res: Account = await invoke('sign_in_with_password', { password });
  return res;
};

export const validateMnemonic = async (mnemonic: string) => {
  const res: boolean = await invoke('validate_mnemonic', { mnemonic });
  return res;
};

export const signOut = async () => {
  await invoke('logout');
};

export const isPasswordCreated = async () => {
  const res: boolean = await invoke('does_password_file_exist');
  return res;
};

export const addAccount = async ({
  mnemonic,
  password,
  accountName,
}: {
  mnemonic: string;
  password: string;
  accountName: string;
}) => {
  const res: AccountEntry = await invoke('add_account_for_password', { mnemonic, password, accountId: accountName });
  return res;
};

export const removeAccount = async ({ password, accountName }: { password: string; accountName: string }) => {
  await invoke('remove_account_for_password', { password, innerId: accountName });
};

export const listAccounts = async () => {
  const res: AccountEntry[] = await invoke('list_accounts');
  return res;
};

export const showMnemonicForAccount = async ({ password, accountName }: { password: string; accountName: string }) => {
  const res: string = await invoke('show_mnemonic_for_account_in_password', { password, accountId: accountName });
  return res;
};

export const switchAccount = async ({ accountId, password }: { accountId: string; password: string }) => {
  await invoke('sign_in_with_password_and_account_id', { accountId, password });
};
