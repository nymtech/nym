import { Account, Balance, AccountEntry } from '@nymproject/types';
import { invokeWrapper } from './wrapper';

export const signInWithMnemonic = async (mnemonic: string): Promise<Account> =>
  invokeWrapper('connect_with_mnemonic', { mnemonic });

export const userBalance = async () => invokeWrapper<Balance>('get_balance');

export const createMnemonic = async () => invokeWrapper<string>('create_new_mnemonic');

export const validateMnemonic = async (mnemonic: string) => invokeWrapper<boolean>('validate_mnemonic', { mnemonic });

export const signOut = async () => invokeWrapper<void>('logout');

export const isPasswordCreated = async () => invokeWrapper<boolean>('does_password_file_exist');

export const createPassword = async ({ mnemonic, password }: { mnemonic: string; password: string }) =>
  invokeWrapper<void>('create_password', { mnemonic, password });

export const updatePassword = async ({
  currentPassword,
  newPassword,
}: {
  currentPassword: string;
  newPassword: string;
}) => invokeWrapper<void>('update_password', { currentPassword, newPassword });

export const signInWithPassword = async (password: string) =>
  invokeWrapper<Account>('sign_in_with_password', { password });

export const switchAccount = async ({ accountId, password }: { accountId: string; password: string }) =>
  invokeWrapper<Account>('sign_in_with_password_and_account_id', { accountId, password });

export const addAccount = async ({
  mnemonic,
  password,
  accountName,
}: {
  mnemonic: string;
  password: string;
  accountName: string;
}) => invokeWrapper<AccountEntry>('add_account_for_password', { mnemonic, password, accountId: accountName });

export const removeAccount = async ({ password, accountName }: { password: string; accountName: string }) =>
  invokeWrapper<void>('remove_account_for_password', { password, innerId: accountName });

export const listAccounts = async () => invokeWrapper<AccountEntry[]>('list_accounts');

export const archiveWalletFile = async () => invokeWrapper<void>('archive_wallet_file');

export const showMnemonicForAccount = async ({ password, accountName }: { password: string; accountName: string }) =>
  invokeWrapper<string>('show_mnemonic_for_account_in_password', { password, accountId: accountName });

export const renameAccount = async ({
  password,
  accountName,
  newAccountName,
}: {
  password: string;
  accountName: string;
  newAccountName: string;
}) =>
  invokeWrapper<AccountEntry>('rename_account_for_password', {
    password,
    accountId: accountName,
    newAccountId: newAccountName,
  });
