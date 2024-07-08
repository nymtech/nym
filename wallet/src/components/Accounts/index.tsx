import React, { useContext } from 'react';
import { AccountsProvider, AppContext } from '@src/context';
import { Accounts, SingleAccount } from './Accounts';

export const MultiAccounts = () => {
  const { loginType } = useContext(AppContext);

  if (loginType === 'password') {
    return (
      <AccountsProvider>
        <Accounts />
      </AccountsProvider>
    );
  }
  return <SingleAccount />;
};
