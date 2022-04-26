import React from 'react';
import { Button } from '@mui/material';
import { v4 as uuidv4 } from 'uuid';
import { TAccount } from 'src/types';
import { EditAccountModal } from './EditAccountModal';
import { AddAccountModal } from './AddAccountModal';
import { AccountsModal } from './AccountsModal';
import { ImportAccountModal } from './ImportAccountModal';
import { AccountAvatar } from './AccountAvatar';
import { TDialog } from './types';

export const Accounts = ({
  accounts,
  selectedAccount,
  accountToEdit,
  dialogToDisplay,
  addAccount,
  editAccount,
  setAccountToEdit,
  importAccount,
  setSelectedAccount,
  setDialogToDisplay,
}: {
  accounts: TAccount[];
  selectedAccount: TAccount;
  accountToEdit?: TAccount;
  dialogToDisplay?: TDialog;
  addAccount: (acc: TAccount) => void;
  editAccount: (acc: TAccount) => void;
  importAccount: (acc: TAccount) => void;
  setAccountToEdit: (acc: TAccount) => void;
  setSelectedAccount: (accs: TAccount) => void;
  setDialogToDisplay: (dialog: TDialog | undefined) => void;
}) => (
  <>
    <Button
      startIcon={<AccountAvatar address={selectedAccount.address} name={selectedAccount.name} />}
      color="inherit"
      onClick={() => setDialogToDisplay('Accounts')}
      disableRipple
    >
      {selectedAccount.name}
    </Button>
    <AccountsModal
      show={dialogToDisplay === 'Accounts'}
      onClose={() => setDialogToDisplay(undefined)}
      accounts={accounts}
      onAccountSelect={(acc) => setSelectedAccount(acc)}
      selectedAccount={selectedAccount.address}
      onAdd={() => {
        setDialogToDisplay('Add');
      }}
      onEdit={(acc) => {
        setAccountToEdit(acc);
        setDialogToDisplay('Edit');
      }}
      onImport={() => setDialogToDisplay('Import')}
    />
    <AddAccountModal
      show={dialogToDisplay === 'Add'}
      onClose={() => {
        setDialogToDisplay('Accounts');
      }}
      onAdd={(name) => {
        addAccount({ name, address: uuidv4() });
        setDialogToDisplay('Accounts');
      }}
    />
    <EditAccountModal
      show={dialogToDisplay === 'Edit'}
      account={accountToEdit}
      onClose={() => {
        setDialogToDisplay('Accounts');
      }}
      onEdit={(account) => {
        editAccount(account);
        setDialogToDisplay('Accounts');
      }}
    />
    <ImportAccountModal
      show={dialogToDisplay === 'Import'}
      onClose={() => setDialogToDisplay('Accounts')}
      onImport={() => {
        importAccount({ name: 'New Account', address: uuidv4() });
        setDialogToDisplay('Accounts');
      }}
    />
  </>
);
