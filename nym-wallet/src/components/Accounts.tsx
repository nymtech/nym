import React, { useEffect, useState } from 'react';
import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  ListItem,
  ListItemAvatar,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  TextField,
  Typography,
} from '@mui/material';
import stc from 'string-to-color';
import { Add, CircleTwoTone, Close, Edit } from '@mui/icons-material';
import { v4 as uuidv4 } from 'uuid';

export type TAccount = {
  name: string;
  address: string;
};

type TDialog = 'Accounts' | 'Add' | 'Edit';

const AccountColor = ({ address }: { address: string }) => <CircleTwoTone sx={{ color: stc(address) }} />;

const AccountItem = ({
  name,
  address,
  selected,
  onSelect,
  onEdit,
}: {
  name: string;
  address: string;
  selected: boolean;
  onSelect: () => void;
  onEdit: () => void;
}) => (
  <ListItem disablePadding disableGutters sx={selected ? { bgcolor: 'rgba(33, 208, 115, 0.1)' } : {}}>
    <ListItemButton disableRipple onClick={onSelect}>
      <ListItemAvatar sx={{ minWidth: 0, mr: 2 }}>
        <AccountColor address={address} />
      </ListItemAvatar>
      <ListItemText primary={name} secondary={address} />
      <ListItemIcon>
        <IconButton
          onClick={(e) => {
            e.stopPropagation();
            onEdit();
          }}
        >
          <Edit />
        </IconButton>
      </ListItemIcon>
    </ListItemButton>
  </ListItem>
);

const AccountsModal = ({
  show,
  accounts,
  selectedAccount,
  onClose,
  onAccountSelect,
  onAddAccount,
  onEditAccount,
}: {
  show: boolean;
  accounts: TAccount[];
  selectedAccount: TAccount['address'];
  onClose: () => void;
  onAccountSelect: (account: TAccount) => void;
  onAddAccount: () => void;
  onEditAccount: (acc: TAccount) => void;
}) => (
  <Dialog open={show} onClose={onClose} fullWidth hideBackdrop>
    <DialogTitle>
      <Box display="flex" justifyContent="space-between" alignItems="center">
        <Typography variant="h6">Accounts</Typography>
        <IconButton onClick={onClose}>
          <Close />
        </IconButton>
      </Box>
      <Typography variant="body1" sx={{ color: 'grey.600' }}>
        Switch between accounts
      </Typography>
    </DialogTitle>
    <DialogContent sx={{ padding: 0 }}>
      {accounts.map(({ name, address }) => (
        <AccountItem
          name={name}
          address={address}
          onSelect={() => {
            onAccountSelect({ name, address });
            onClose();
          }}
          onEdit={() => onEditAccount({ name, address })}
          selected={selectedAccount === address}
          key={address}
        />
      ))}
    </DialogContent>
    <DialogActions sx={{ p: 3 }}>
      <Button
        fullWidth
        disableElevation
        variant="contained"
        size="large"
        startIcon={<Add fontSize="small" />}
        onClick={onAddAccount}
      >
        Add new account
      </Button>
    </DialogActions>
  </Dialog>
);

const AddAccountModal = ({
  show,
  onClose,
  onAdd,
}: {
  show: boolean;
  onClose: () => void;
  onAdd: (accountName: string) => void;
}) => {
  const [accountName, setAccountName] = useState('');

  useEffect(() => {
    if (!show) setAccountName('');
  }, [show]);

  return (
    <Dialog open={show} onClose={onClose} fullWidth hideBackdrop>
      <DialogTitle>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">Add new account</Typography>
          <IconButton onClick={onClose}>
            <Close />
          </IconButton>
        </Box>
        <Typography variant="body1" sx={{ color: 'grey.600' }}>
          New wallet address
        </Typography>
      </DialogTitle>
      <DialogContent sx={{ p: 0 }}>
        <Box sx={{ px: 3, mt: 1 }}>
          <TextField
            label="Account name"
            fullWidth
            value={accountName}
            onChange={(e) => setAccountName(e.target.value)}
            autoFocus
          />
        </Box>
      </DialogContent>
      <DialogActions sx={{ p: 3 }}>
        <Button
          fullWidth
          disableElevation
          variant="contained"
          size="large"
          startIcon={<Add fontSize="small" />}
          onClick={() => onAdd(accountName)}
          disabled={!accountName.length}
        >
          Add
        </Button>
      </DialogActions>
    </Dialog>
  );
};

const EditAccountModal = ({
  account,
  show,
  onClose,
  onEdit,
}: {
  account?: TAccount;
  show: boolean;
  onClose: () => void;
  onEdit: (account: TAccount) => void;
}) => {
  const [accountName, setAccountName] = useState('');

  useEffect(() => {
    setAccountName(account ? account?.name : '');
  }, [account]);

  return (
    <Dialog open={show} onClose={onClose} fullWidth hideBackdrop>
      <DialogTitle>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">Edit account name</Typography>
          <IconButton onClick={onClose}>
            <Close />
          </IconButton>
        </Box>
        <Typography variant="body1" sx={{ color: 'grey.600' }}>
          New wallet address
        </Typography>
      </DialogTitle>
      <DialogContent sx={{ p: 0 }}>
        <Box sx={{ px: 3, mt: 1 }}>
          <TextField
            label="Account name"
            fullWidth
            value={accountName}
            onChange={(e) => setAccountName(e.target.value)}
            autoFocus
          />
        </Box>
      </DialogContent>
      <DialogActions sx={{ p: 3 }}>
        <Button
          fullWidth
          disableElevation
          variant="contained"
          size="large"
          onClick={() => account && onEdit({ ...account, name: accountName })}
          disabled={!accountName?.length}
        >
          Edit
        </Button>
      </DialogActions>
    </Dialog>
  );
};

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
        onAddAccount={() => {
          setDialogToDisplay('Add');
        }}
        onEditAccount={(acc) => {
          setAccountToEdit(acc);
          setDialogToDisplay('Edit');
        }}
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
    </>
  );
};
