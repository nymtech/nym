import React, { createContext, useContext, useEffect, useMemo, useState } from 'react';
import { AccountEntry } from 'src/types';
import { addAccount as addAccountRequest } from 'src/requests';
import { useSnackbar } from 'notistack';
import { ClientContext } from './main';

type TAccounts = {
  accounts?: AccountEntry[];
  selectedAccount?: AccountEntry;
  accountToEdit?: AccountEntry;
  dialogToDisplay?: TAccountsDialog;
  isLoading: boolean;
  handleAddAccount: (data: { accountName: string; mnemonic: string; password: string }) => void;
  setDialogToDisplay: (dialog?: TAccountsDialog) => void;
  handleSelectAccount: (accountId: string) => void;
  handleAccountToEdit: (accountId: string) => void;
  handleEditAccount: (account: AccountEntry) => void;
  handleImportAccount: (account: AccountEntry) => void;
};

export type TAccountsDialog = 'Accounts' | 'Add' | 'Edit' | 'Import';

export const AccountsContext = createContext({} as TAccounts);

export const AccountsProvider: React.FC = ({ children }) => {
  const [accounts, setAccounts] = useState<AccountEntry[]>([]);
  const [selectedAccount, setSelectedAccount] = useState<AccountEntry>();
  const [accountToEdit, setAccountToEdit] = useState<AccountEntry>();
  const [dialogToDisplay, setDialogToDisplay] = useState<TAccountsDialog>();
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
      enqueueSnackbar(`Error adding account: ${e}`, { variant: 'error' });
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
          accounts,
          selectedAccount,
          accountToEdit,
          dialogToDisplay,
          setDialogToDisplay,
          isLoading,
          handleAddAccount,
          handleEditAccount,
          handleAccountToEdit,
          handleSelectAccount,
          handleImportAccount,
        }),
        [accounts, selectedAccount, accountToEdit, dialogToDisplay, isLoading],
      )}
    >
      {children}
    </AccountsContext.Provider>
  );
};
