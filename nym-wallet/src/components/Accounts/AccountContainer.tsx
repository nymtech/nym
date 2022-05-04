import React, { useEffect, useState } from 'react';
import { AccountEntry } from 'src/types';
import { addAccount as addAccountRequest } from 'src/requests';
import { Accounts } from './Accounts';
import { TDialog } from './types';

export const AccountsContainer = ({ storedAccounts }: { storedAccounts: AccountEntry[] }) => {
  const [accounts, setAccounts] = useState<AccountEntry[]>(storedAccounts);
  const [selectedAccount, setSelectedAccount] = useState<AccountEntry>(storedAccounts[0]);
  const [accountToEdit, setAccountToEdit] = useState<AccountEntry>();
  const [dialogToDisplay, setDialogToDisplay] = useState<TDialog>();

  useEffect(() => {
    const selected = accounts?.find((acc) => acc.address === selectedAccount?.address);
    if (selected) setSelectedAccount(selected);
  }, [accounts, storedAccounts]);

  useEffect(() => {
    setAccounts(storedAccounts);
  }, [storedAccounts]);

  const addAccount = async ({
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
  const editAccount = (account: AccountEntry) =>
    setAccounts((accs) => accs?.map((acc) => (acc.address === account.address ? account : acc)));
  const importAccount = (account: AccountEntry) => setAccounts((accs) => [...accs, account]);
  const handleAccountToEdit = (accountName: string) => setAccountToEdit(accounts.find((acc) => acc.id === accountName));
  const handleSelectedAccount = (accountName: string) => {
    const match = accounts.find((acc) => acc.id === accountName);
    if (match) setSelectedAccount(match);
  };

  return (
    <Accounts
      accounts={accounts}
      selectedAccount={selectedAccount}
      accountToEdit={accountToEdit}
      dialogToDisplay={dialogToDisplay}
      addAccount={addAccount}
      editAccount={editAccount}
      importAccount={importAccount}
      setAccountToEdit={handleAccountToEdit}
      setSelectedAccount={handleSelectedAccount}
      setDialogToDisplay={setDialogToDisplay}
    />
  );
};
