import React from 'react';
import { Button } from '@mui/material';
import { v4 as uuidv4 } from 'uuid';
import { AccountEntry } from 'src/types';
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
  accounts?: AccountEntry[];
  selectedAccount?: AccountEntry;
  accountToEdit?: AccountEntry;
  dialogToDisplay?: TDialog;
  addAccount: (acc: { accountName: string; mnemonic: string; password: string }) => Promise<void>;
  editAccount: (acc: AccountEntry) => void;
  importAccount: (acc: AccountEntry & { mnemonic: string }) => void;
  setAccountToEdit: (accountName: string) => void;
  setSelectedAccount: (accountName: string) => void;
  setDialogToDisplay: (dialog: TDialog | undefined) => void;
}) =>
  accounts && selectedAccount ? (
    <>
      <Button
        startIcon={<AccountAvatar address={selectedAccount.address} name={selectedAccount.id} />}
        sx={{ color: 'nym.text.dark' }}
        onClick={() => setDialogToDisplay('Accounts')}
        disableRipple
      >
        {selectedAccount.id}
      </Button>
      <AccountsModal
        show={dialogToDisplay === 'Accounts'}
        onClose={() => setDialogToDisplay(undefined)}
        accounts={accounts}
        onAccountSelect={(accountName) => setSelectedAccount(accountName)}
        selectedAccount={selectedAccount.id}
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
        onAdd={async (data) => {
          addAccount(data);
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
          importAccount({ id: 'New Account', address: uuidv4(), mnemonic });
          setDialogToDisplay('Accounts');
        }}
      />
    </>
  ) : null;
