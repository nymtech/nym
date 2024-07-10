import { useContext, useState } from 'react';
import { AccountsContext, AppContext } from '@src/context';
import { EditAccountModal } from './modals/EditAccountModal';
import { AddAccountModal } from './modals/AddAccountModal';
import { AccountsModal } from './modals/AccountsModal';
import { MnemonicModal } from './modals/MnemonicModal';
import { AccountOverview } from './AccountOverview';
import { MultiAccountHowTo } from './modals/MultiAccountHowTo';

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
  const { clientDetails } = useContext(AppContext);

  return (
    <>
      <AccountOverview
        account={{ id: 'Account 1', address: clientDetails?.client_address || '' }}
        onClick={() => setShowHowToDialog(true)}
      />
      <MultiAccountHowTo show={showHowToDialog} handleClose={() => setShowHowToDialog(false)} />
    </>
  );
};
