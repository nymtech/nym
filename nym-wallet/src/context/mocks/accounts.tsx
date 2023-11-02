import React, { useMemo, useState } from 'react';
import { AccountEntry } from '@nymproject/types';
import { AccountsContext, TAccountMnemonic, TAccountsDialog } from '../accounts';

export const MockAccountsProvider: FCWithChildren = ({ children }) => {
  const [accounts, setAccounts] = useState<AccountEntry[]>([{ id: 'Account_1', address: 'abc123' }]);
  const [selectedAccount, setSelectedAccount] = useState<AccountEntry | undefined>({
    id: 'Account_1',
    address: 'abc123',
  });
  const [accountToEdit, setAccountToEdit] = useState<AccountEntry>();
  const [dialogToDisplay, setDialogToDisplay] = useState<TAccountsDialog>();
  const [accountMnemonic, setAccountMnemonic] = useState<TAccountMnemonic>({
    value: undefined,
    accountName: undefined,
  });
  const [error, setError] = useState<string>();
  const [isLoading, setIsLoading] = useState(false);

  const handleAddAccount = async ({ accountName }: { accountName: string; mnemonic: string; password: string }) => {
    setIsLoading(true);
    try {
      setAccounts((accs) => [...accs, { address: 'abc123', id: accountName }]);
      setDialogToDisplay('Accounts');
    } catch (e) {
      setError(`Error adding account: ${e}`);
    } finally {
      setIsLoading(false);
    }
  };
  const handleEditAccount = async ({
    password,
    account,
    newAccountName,
  }: {
    password: string;
    account: AccountEntry;
    newAccountName: string;
  }) => {
    if (password) {
      setIsLoading(true);
      try {
        setAccounts((accs) => accs.map((acc) => (acc.id === account.id ? { ...acc, id: newAccountName } : acc)));
        setDialogToDisplay('Accounts');
      } catch (e) {
        setError(`Error adding account: ${e}`);
      } finally {
        setIsLoading(false);
      }
    }
  };

  const handleImportAccount = (account: AccountEntry) => setAccounts((accs) => [...(accs ? [...accs] : []), account]);

  const handleAccountToEdit = (accountName: string | undefined) =>
    setAccountToEdit(accounts?.find((acc) => acc.id === accountName));

  const handleSelectAccount = async ({ accountName }: { accountName: string; password: string }) => {
    const match = accounts?.find((acc) => acc.id === accountName);
    setSelectedAccount(match);
    return true;
  };

  const handleGetAccountMnemonic = async ({ accountName }: { password: string; accountName: string }) => {
    try {
      setIsLoading(true);
      const mnemonic = 'test mnemonic';
      setAccountMnemonic({ value: mnemonic, accountName });
    } catch (e) {
      setError(e as string);
    } finally {
      setIsLoading(false);
    }
  };
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
