import React, { useMemo, useState } from 'react';
import { AccountEntry } from 'src/types';
import { AccountsContext, TAccountMnemonic, TAccountsDialog } from '../accounts';

export const MockAccountsProvider: React.FC = ({ children }) => {
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
  const handleEditAccount = (account: AccountEntry) =>
    setAccounts((accs) => accs?.map((acc) => (acc.address === account.address ? account : acc)));

  const handleImportAccount = (account: AccountEntry) => setAccounts((accs) => [...(accs ? [...accs] : []), account]);

  const handleAccountToEdit = (accountName: string) =>
    setAccountToEdit(accounts?.find((acc) => acc.id === accountName));

  const handleSelectAccount = async (accountName: string) => {
    if (accountName !== selectedAccount?.id) {
      const match = accounts?.find((acc) => acc.id === accountName);
      setSelectedAccount(match);
    }
  };

  const handleGetAcccountMnemonic = async ({ accountName }: { password: string; accountName: string }) => {
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
          handleGetAcccountMnemonic,
        }),
        [accounts, selectedAccount, accountToEdit, dialogToDisplay, isLoading, error, accountMnemonic],
      )}
    >
      {children}
    </AccountsContext.Provider>
  );
};
