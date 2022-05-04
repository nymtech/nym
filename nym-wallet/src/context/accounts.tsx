import React, { createContext, useContext, useEffect, useMemo, useState } from 'react';
import { Account, AccountEntry, TAccountsDialog } from 'src/types';
import { addAccount as addAccountRequest } from 'src/requests';
import { ClientContext } from './main';

type TAccounts = {
  accounts?: AccountEntry[];
  selectedAccount?: AccountEntry;
  accountToEdit?: AccountEntry;
  dialogToDisplay?: TAccountsDialog;
  handleAddAccount: (data: { accountName: string; mnemonic: string; password: string }) => void;
  setDialogToDisplay: (dialog?: TAccountsDialog) => void;
  handleSelectAccount: (accountId: string) => void;
  handleAccountToEdit: (accountId: string) => void;
  handleEditAccount: (account: AccountEntry) => void;
  handleImportAccount: (account: AccountEntry) => void;
};

export const AccountsContext = createContext({} as TAccounts);

export const AccountsProvider: React.FC = ({ children }) => {
  const [accounts, setAccounts] = useState<AccountEntry[]>();
  const [selectedAccount, setSelectedAccount] = useState<AccountEntry>();
  const [accountToEdit, setAccountToEdit] = useState<AccountEntry>();
  const [dialogToDisplay, setDialogToDisplay] = useState<TAccountsDialog>();

  const { onAccountChange, storedAccounts } = useContext(ClientContext);

  const handleAddAccount = async ({
    accountName,
    mnemonic,
    password,
  }: {
    accountName: string;
    mnemonic: string;
    password: string;
  }) => {
    await addAccountRequest({
      accountName,
      mnemonic,
      password,
    });
  };
  const handleEditAccount = (account: AccountEntry) =>
    setAccounts((accs) => accs?.map((acc) => (acc.address === account.address ? account : acc)));

  const handleImportAccount = (account: AccountEntry) => setAccounts((accs) => [...(accs ? [...accs] : []), account]);

  const handleAccountToEdit = (accountName: string) =>
    setAccountToEdit(accounts?.find((acc) => acc.id === accountName));

  const handleSelectAccount = async (accountName: string) => {
    const match = accounts?.find((acc) => acc.id === accountName);
    if (match) {
      try {
        await onAccountChange(match.id);
        setSelectedAccount(match);
      } catch (e) {
        console.log('Error swtiching account');
      }
    }
  };

  useEffect(() => {
    if (storedAccounts) {
      setAccounts(storedAccounts);
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
          handleAddAccount,
          handleEditAccount,
          handleAccountToEdit,
          handleSelectAccount,
          handleImportAccount,
        }),
        [accounts, selectedAccount, accountToEdit, dialogToDisplay],
      )}
    >
      {children}
    </AccountsContext.Provider>
  );
};
