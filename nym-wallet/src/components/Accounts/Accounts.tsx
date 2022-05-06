import React, { useContext } from 'react';
import { Button } from '@mui/material';
import { AccountsContext } from 'src/context';
import { EditAccountModal } from './EditAccountModal';
import { AddAccountModal } from './AddAccountModal';
import { AccountsModal } from './AccountsModal';
import { AccountAvatar } from './AccountAvatar';
import { MnemonicModal } from './MnemonicModal';

export const Accounts = () => {
  const { accounts, selectedAccount, setDialogToDisplay } = useContext(AccountsContext);

  return accounts && selectedAccount ? (
    <>
      <Button
        startIcon={<AccountAvatar name={selectedAccount.id} />}
        sx={{ color: 'nym.text.dark' }}
        onClick={() => setDialogToDisplay('Accounts')}
        disableRipple
      >
        {selectedAccount.id}
      </Button>
      <AccountsModal />
      <AddAccountModal />
      <EditAccountModal />
      <MnemonicModal />
    </>
  ) : null;
};
