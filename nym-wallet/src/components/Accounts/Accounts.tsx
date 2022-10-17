import React, { useContext, useEffect, useState } from 'react';
import { AccountsContext, AppContext } from 'src/context';
import { isPasswordCreated } from 'src/requests';
import { EditAccountModal } from './modals/EditAccountModal';
import { AddAccountModal } from './modals/AddAccountModal';
import { AccountsModal } from './modals/AccountsModal';
import { MnemonicModal } from './modals/MnemonicModal';
import { AccountOverview } from './AccountOverview';
import { MultiAccountHowTo } from './MultiAccountHowTo';
import { MultiAccountWithPwdHowTo } from './MultiAccountWithPwdHowTo';

export const Accounts = () => {
  const { accounts, selectedAccount, setDialogToDisplay } = useContext(AccountsContext);

  return accounts && selectedAccount ? (
    <>
      <AccountOverview account={selectedAccount} onClick={() => setDialogToDisplay('Accounts')} />
      <AccountsModal />
      <AddAccountModal />
      <EditAccountModal />
      <MnemonicModal />
    </>
  ) : null;
};

export const SingleAccount = () => {
  const [showHowToDialog, setShowHowToDialog] = useState(false);
  const [passwordExist, setPasswordExist] = useState(false);
  const { clientDetails } = useContext(AppContext);

  useEffect(() => {
    const checkPassword = async () => {
      if (await isPasswordCreated()) {
        setPasswordExist(true);
      }
    };
    checkPassword();
  }, [clientDetails]);

  return (
    <>
      <AccountOverview
        account={{ id: 'Account 1', address: clientDetails?.client_address || '' }}
        onClick={() => setShowHowToDialog(true)}
      />
      {passwordExist ? (
        <MultiAccountWithPwdHowTo show={showHowToDialog} handleClose={() => setShowHowToDialog(false)} />
      ) : (
        <MultiAccountHowTo show={showHowToDialog} handleClose={() => setShowHowToDialog(false)} />
      )}
    </>
  );
};
