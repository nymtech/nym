import React, { useEffect, useState } from 'react';
import { TAccount } from 'src/types';
import { Accounts } from './Accounts';
import { TDialog } from './types';

export const AccountsContainer = ({ storedAccounts }: { storedAccounts: TAccount[] }) => {
  const [accounts, setAccounts] = useState(storedAccounts);
  const [selectedAccount, setSelectedAccount] = useState(accounts[0]);
  const [accountToEdit, setAccountToEdit] = useState<TAccount>();
  const [dialogToDisplay, setDialogToDisplay] = useState<TDialog>();

  useEffect(() => {
    const selected = accounts.find((acc) => acc.address === selectedAccount.address);
    if (selected) setSelectedAccount(selected);
  }, [accounts]);

  const addAccount = (account: TAccount) => setAccounts((accs) => [...accs, account]);
  const editAccount = (account: TAccount) =>
    setAccounts((accs) => accs.map((acc) => (acc.address === account.address ? account : acc)));
  const importAccount = (account: TAccount) => setAccounts((accs) => [...accs, account]);

  return (
    <Accounts
      accounts={accounts}
      selectedAccount={selectedAccount}
      accountToEdit={accountToEdit}
      dialogToDisplay={dialogToDisplay}
      addAccount={addAccount}
      editAccount={editAccount}
      importAccount={importAccount}
      setAccountToEdit={setAccountToEdit}
      setSelectedAccount={setSelectedAccount}
      setDialogToDisplay={setDialogToDisplay}
    />
  );
};
