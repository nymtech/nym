import React, { createContext, Dispatch, SetStateAction, useContext, useEffect, useMemo, useState } from 'react';
import { AccountEntry } from '@nymproject/types';
import { addAccount as addAccountRequest, renameAccount, showMnemonicForAccount } from '../requests';
import { useSnackbar } from 'notistack';
import { AppContext } from './main';

type TAccounts = {
  accounts?: AccountEntry[];
  selectedAccount?: AccountEntry;
  accountToEdit?: AccountEntry;
  dialogToDisplay?: TAccountsDialog;
  isLoading: boolean;
  error?: string;
  accountMnemonic: TAccountMnemonic;
  setError: Dispatch<SetStateAction<string | undefined>>;
  setAccountMnemonic: Dispatch<SetStateAction<TAccountMnemonic>>;
  handleAddAccount: (data: { accountName: string; mnemonic: string; password: string }) => void;
  setDialogToDisplay: (dialog?: TAccountsDialog) => void;
  handleSelectAccount: (data: { accountName: string; password: string }) => Promise<boolean>;
  handleAccountToEdit: (accountId: string | undefined) => void;
  handleEditAccount: ({
    account,
    newAccountName,
    password,
  }: {
    account: AccountEntry;
    newAccountName: string;
    password: string;
  }) => Promise<void>;
  handleImportAccount: (account: AccountEntry) => void;
  handleGetAccountMnemonic: (data: { password: string; accountName: string }) => void;
};

export type TAccountsDialog = 'Accounts' | 'Add' | 'Edit' | 'Import' | 'Mnemonic';
export type TAccountMnemonic = { value?: string; accountName?: string };

export const AccountsContext = createContext({} as TAccounts);

export const AccountsProvider: FCWithChildren = ({ children }) => {
  const [accounts, setAccounts] = useState<AccountEntry[]>([]);
  const [selectedAccount, setSelectedAccount] = useState<AccountEntry>();
  const [accountToEdit, setAccountToEdit] = useState<AccountEntry>();
  const [dialogToDisplay, setDialogToDisplay] = useState<TAccountsDialog>();
  const [accountMnemonic, setAccountMnemonic] = useState<TAccountMnemonic>({
    value: undefined,
    accountName: undefined,
  });
  const [error, setError] = useState<string>();
  const [isLoading, setIsLoading] = useState(false);
  const { onAccountChange, storedAccounts } = useContext(AppContext);
  const { enqueueSnackbar } = useSnackbar();

  const handleAddAccount = async ({
    accountName,
    mnemonic,
    password,
  }: {
    accountName: string;
    mnemonic: string;
    password: string;
  }) => {
    setIsLoading(true);
    try {
      const newAccount = await addAccountRequest({
        accountName,
        mnemonic,
        password,
      });
      setAccounts((accs) => [...accs, newAccount]);
      enqueueSnackbar('New account created', { variant: 'success' });
    } catch (e) {
      setError(`Error adding account: ${e}`);
      throw new Error();
    } finally {
      setIsLoading(false);
    }
  };
  const handleEditAccount = async ({
    account,
    newAccountName,
    password,
  }: {
    account: AccountEntry;
    newAccountName: string;
    password: string;
  }) => {
    setIsLoading(true);
    try {
      await renameAccount({ accountName: account.id, newAccountName, password });
      setAccounts((accs) =>
        accs?.map((acc) => (acc.address === account.address ? { ...acc, id: newAccountName } : acc)),
      );
      if (selectedAccount?.id === account.id) {
        setSelectedAccount({ ...selectedAccount, id: newAccountName });
      }
      setDialogToDisplay('Accounts');
    } catch (e) {
      throw new Error(`Error editing account: ${e}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleImportAccount = (account: AccountEntry) => setAccounts((accs) => [...(accs ? [...accs] : []), account]);

  const handleAccountToEdit = (accountName: string | undefined) =>
    setAccountToEdit(accounts?.find((acc) => acc.id === accountName));

  const handleSelectAccount = async ({ accountName, password }: { accountName: string; password: string }) => {
    try {
      await onAccountChange({ accountId: accountName, password });
      const match = accounts?.find((acc) => acc.id === accountName);
      setSelectedAccount(match);
      return true;
    } catch (e) {
      setError('Error switching account. Please check your password');
      return false;
    }
  };

  const handleGetAccountMnemonic = async ({ password, accountName }: { password: string; accountName: string }) => {
    try {
      setIsLoading(true);
      const mnemonic = await showMnemonicForAccount({ password, accountName });
      setAccountMnemonic({ value: mnemonic, accountName });
    } catch (e) {
      setError(e as string);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    if (storedAccounts) {
      setAccounts(storedAccounts);
    }

    if (storedAccounts && !selectedAccount) {
      setSelectedAccount(storedAccounts[0]);
    }
  }, [storedAccounts]);

  return (
    <AccountsContext.Provider
      value={useMemo(
        () => ({
          error,
          setError,
          accounts,
          selectedAccount,
          accountToEdit,
          dialogToDisplay,
          accountMnemonic,
          setDialogToDisplay,
          setAccountMnemonic,
          isLoading,
          handleAddAccount,
          handleEditAccount,
          handleAccountToEdit,
          handleSelectAccount,
          handleImportAccount,
          handleGetAccountMnemonic,
        }),
        [accounts, selectedAccount, accountToEdit, dialogToDisplay, isLoading, error, accountMnemonic],
      )}
    >
      {children}
    </AccountsContext.Provider>
  );
};
