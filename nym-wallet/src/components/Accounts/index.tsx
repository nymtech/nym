import React, { useEffect, useState } from 'react';
import { Button } from '@mui/material';
import { v4 as uuidv4 } from 'uuid';
import { TAccount } from 'src/types';
import { EditAccountModal } from './EditAccountModal';
import { AddAccountModal } from './AddAccountModal';
import { AccountsModal } from './AccountsModal';
import { ImportAccountModal } from './ImportAccountModal';
import { AccountAvatar } from './AccountAvatar';

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
        startIcon={<AccountAvatar address={selectedAccount.address} name={selectedAccount.name} />}
        color="inherit"
        onClick={() => setDialogToDisplay('Accounts')}
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
          setDialogToDisplay('Accounts');
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
        onImport={() => {
          setAccounts((accs) => [...accs, { name: 'New Account', address: uuidv4() }]);
          setDialogToDisplay('Accounts');
        }}
      />
    </>
  );
};
