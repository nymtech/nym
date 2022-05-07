import React from 'react';
import { AccountsProvider } from 'src/context';
import { Accounts } from './Accounts';

export const MultiAccounts = () => (
  <AccountsProvider>
    <Accounts />
  </AccountsProvider>
);
