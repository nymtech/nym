import React, { useEffect, useState } from 'react';
import { Button } from '@mui/material';
import { v4 as uuidv4 } from 'uuid';
import { TAccount } from 'src/types';
import { EditAccountModal } from './EditAccountModal';
import { AddAccountModal } from './AddAccountModal';
import { AccountColor } from './AccountColor';
import { AccountsModal } from './AccountsModal';
import { ImportAccountModal } from './ImportAccountModal';

export type TDialog = 'Accounts' | 'Add' | 'Edit' | 'Import';

export const Accounts = ({ storedAccounts }: { storedAccounts: TAccount[] }) => {
  const [accounts, setAccounts] = useState(storedAccounts);
  const [selectedAccount, setSelectedAccount] = useState(accounts[0]);
  const [accountToEdit, setAccountToEdit] = useState<TAccount>();
  const [dialogToDisplasy, setDialogToDisplay] = useState<TDialog>();

  useEffect(() => {
    const selected = accounts.find((acc) => acc.address === selectedAccount.address);
    if (selected) setSelectedAccount(selected);
  }, [accounts]);

  return (
    <>
      <Button
        startIcon={<AccountColor address={selectedAccount.address} />}
        color="inherit"
        onClick={() => setDialogToDisplay('Accounts')}
        size="large"
        disableRipple
      >
        {selectedAccount.name}
      </Button>
      <AccountsModal
        show={dialogToDisplasy === 'Accounts'}
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
        show={dialogToDisplasy === 'Add'}
        onClose={() => {
          setDialogToDisplay(undefined);
        }}
        onAdd={(name) => {
          setAccounts((accs) => [...accs, { address: uuidv4(), name }]);
          setDialogToDisplay('Accounts');
        }}
      />
      <EditAccountModal
        show={dialogToDisplasy === 'Edit'}
        account={accountToEdit}
        onClose={() => {
          setDialogToDisplay('Accounts');
        }}
        onEdit={(account) => {
          setAccounts((accs) => accs.map((acc) => (acc.address === account.address ? account : acc)));
          setDialogToDisplay('Accounts');
        }}
      />
      <ImportAccountModal
        show={dialogToDisplasy === 'Import'}
        onClose={() => setDialogToDisplay('Accounts')}
        onImport={(mnemonic) => {
          setAccounts((accs) => [...accs, { name: 'New Account', address: uuidv4() }]);
          setDialogToDisplay('Accounts');
        }}
      />
    </>
  );
};
