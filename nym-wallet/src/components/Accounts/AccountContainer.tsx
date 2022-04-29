import React, { useEffect, useState } from 'react';
import { TAccount } from 'src/types';
import { Accounts } from './Accounts';
import { TDialog } from './types';

export const AccountsContainer = ({ storedAccounts }: { storedAccounts: TAccount[] }) => {
  const [accounts, setAccounts] = useState<TAccount[]>(storedAccounts);
  const [selectedAccount, setSelectedAccount] = useState<TAccount>(storedAccounts[0]);
  const [accountToEdit, setAccountToEdit] = useState<TAccount>();
  const [dialogToDisplay, setDialogToDisplay] = useState<TDialog>();

  useEffect(() => {
    const selected = accounts?.find((acc) => acc.address === selectedAccount?.address);
    if (selected) setSelectedAccount(selected);
  }, [accounts]);

  const addAccount = (account: TAccount) => setAccounts((accs) => [...accs, account]);
  const editAccount = (account: TAccount) =>
    setAccounts((accs) => accs?.map((acc) => (acc.address === account.address ? account : acc)));
  const importAccount = (account: TAccount) => setAccounts((accs) => [...accs, account]);
  const handleAccountToEdit = (accountName: string) =>
    setAccountToEdit(accounts.find((acc) => acc.name === accountName));
  const handleSelectedAccount = (accountName: string) => {
    const match = accounts.find((acc) => acc.name === accountName);
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
