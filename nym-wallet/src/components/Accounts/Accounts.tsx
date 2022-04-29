import React from 'react';
import { Button } from '@mui/material';
import { v4 as uuidv4 } from 'uuid';
import { TAccount } from 'src/types';
import { createMnemonic } from 'src/requests';
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
  accounts?: TAccount[];
  selectedAccount: TAccount;
  accountToEdit?: TAccount;
  dialogToDisplay?: TDialog;
  addAccount: (acc: TAccount) => void;
  editAccount: (acc: TAccount) => void;
  importAccount: (acc: TAccount) => void;
  setAccountToEdit: (accountName: string) => void;
  setSelectedAccount: (accountName: string) => void;
  setDialogToDisplay: (dialog: TDialog | undefined) => void;
}) =>
  accounts ? (
    <>
      <Button
        startIcon={<AccountAvatar address={selectedAccount.address} name={selectedAccount.name} />}
        sx={{ color: 'nym.text.dark' }}
        onClick={() => setDialogToDisplay('Accounts')}
        disableRipple
      >
        {selectedAccount.name}
      </Button>
      <AccountsModal
        show={dialogToDisplay === 'Accounts'}
        onClose={() => setDialogToDisplay(undefined)}
        accounts={accounts}
        onAccountSelect={(accountName) => setSelectedAccount(accountName)}
        selectedAccount={selectedAccount.address}
        onAdd={() => {
          setDialogToDisplay('Add');
        }}
        onEdit={(accountName) => {
          setAccountToEdit(accountName);
          setDialogToDisplay('Edit');
        }}
        onImport={() => setDialogToDisplay('Import')}
      />
      <AddAccountModal
        show={dialogToDisplay === 'Add'}
        onClose={() => {
          setDialogToDisplay('Accounts');
        }}
        onAdd={async (name) => {
          const mnemonic = await createMnemonic();
          addAccount({ name, address: uuidv4(), mnemonic });
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
        onImport={(mnemonic) => {
          importAccount({ name: 'New Account', address: uuidv4(), mnemonic });
          setDialogToDisplay('Accounts');
        }}
      />
    </>
  ) : null;
