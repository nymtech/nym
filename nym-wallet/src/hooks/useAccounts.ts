import React, { useState } from 'react';
import { TDialog } from 'src/components/Accounts';

export const useAccounts = () => {
  const [accounts, setAccounts] = useState(storedAccounts);
  const [selectedAccount, setSelectedAccount] = useState(accounts[0]);
  const [accountToEdit, setAccountToEdit] = useState<TAccount>();
  const [dialogToDisplay, setDialogToDisplay] = useState<TDialog>();

  return { dialogToDisplay, accounts, accountToEdit, selectedAccount };
};
