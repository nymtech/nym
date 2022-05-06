import React, { createContext, Dispatch, SetStateAction, useContext, useEffect, useMemo, useState } from 'react';
import { AccountEntry } from 'src/types';
import { addAccount as addAccountRequest, showMnemonicForAccount } from 'src/requests';
import { useSnackbar } from 'notistack';
import { ClientContext } from './main';

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
  handleSelectAccount: (accountId: string) => void;
  handleAccountToEdit: (accountId: string) => void;
  handleEditAccount: (account: AccountEntry) => void;
  handleImportAccount: (account: AccountEntry) => void;
  handleGetAcccountMnemonic: (data: { password: string; accountName: string }) => void;
};

export type TAccountsDialog = 'Accounts' | 'Add' | 'Edit' | 'Import' | 'Mnemonic';
export type TAccountMnemonic = { value?: string; accountName?: string };

export const AccountsContext = createContext({} as TAccounts);

export const AccountsProvider: React.FC = ({ children }) => {
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
  const { onAccountChange, storedAccounts } = useContext(ClientContext);
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
      setDialogToDisplay('Accounts');
    } catch (e) {
      setError(`Error adding account: ${e}`);
    } finally {
      setIsLoading(false);
    }
  };
  const handleEditAccount = (account: AccountEntry) =>
    setAccounts((accs) => accs?.map((acc) => (acc.address === account.address ? account : acc)));

  const handleImportAccount = (account: AccountEntry) => setAccounts((accs) => [...(accs ? [...accs] : []), account]);

  const handleAccountToEdit = (accountName: string) =>
    setAccountToEdit(accounts?.find((acc) => acc.id === accountName));

  const handleSelectAccount = async (accountName: string) => {
    await onAccountChange(accountName);
    const match = accounts?.find((acc) => acc.id === accountName);
    setSelectedAccount(match);
  };

  const handleGetAcccountMnemonic = async ({ password, accountName }: { password: string; accountName: string }) => {
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
          handleGetAcccountMnemonic,
        }),
        [accounts, selectedAccount, accountToEdit, dialogToDisplay, isLoading, error, accountMnemonic],
      )}
    >
      {children}
    </AccountsContext.Provider>
  );
};
