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
import { Add, Circle, Close, Edit } from '@mui/icons-material';
import { v4 as uuidv4 } from 'uuid';

export type TAccount = {
  name: string;
  address: string;
};

const AccountColor = ({ address }: { address: string }) => <Circle sx={{ color: stc(address) }} />;

const AccountItem = ({ name, address, onSelect }: { name: string; address: string; onSelect: () => void }) => (
  <ListItem disablePadding disableGutters>
    <ListItemButton disableRipple onClick={onSelect}>
      <ListItemAvatar>
        <AccountColor address={address} />
      </ListItemAvatar>
      <ListItemText primary={name} secondary={address} />
      <ListItemIcon>
        <IconButton onClick={(e) => e.stopPropagation()}>
          <Edit />
        </IconButton>
      </ListItemIcon>
    </ListItemButton>
  </ListItem>
);

const AccountModal = ({
  show,
  accounts,
  onClose,
  onAccountSelect,
  onAddAccount,
}: {
  show: boolean;
  accounts: TAccount[];
  onClose: () => void;
  onAccountSelect: (account: TAccount) => void;
  onAddAccount: () => void;
}) => (
  <Dialog open={show} onClose={onClose} fullWidth>
    <DialogTitle>
      <Box display="flex" justifyContent="space-between" alignItems="center">
        <Typography variant="h6">Account</Typography>
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
    <Dialog open={show} onClose={onClose} fullWidth>
      <DialogTitle>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">Account</Typography>
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

export const Accounts = ({ storedAccounts }: { storedAccounts: TAccount[] }) => {
  const [accounts, setAccounts] = useState(storedAccounts);
  const [selectedAccount, setSelectedAccount] = useState(accounts[0]);
  const [showAccountsDialog, setShowAccountsDialog] = useState(false);
  const [showNewAccountsDialog, setShowNewAccountsDialog] = useState(false);

  return (
    <>
      <Button
        startIcon={<AccountColor address={selectedAccount.address} />}
        color="inherit"
        onClick={() => setShowAccountsDialog(true)}
      >
        {selectedAccount.name}
      </Button>
      <AccountModal
        show={showAccountsDialog}
        onClose={() => setShowAccountsDialog(false)}
        accounts={accounts}
        onAccountSelect={(acc) => setSelectedAccount(acc)}
        onAddAccount={() => {
          setShowAccountsDialog(false);
          setShowNewAccountsDialog(true);
        }}
      />
      <AddAccountModal
        show={showNewAccountsDialog}
        onClose={() => setShowNewAccountsDialog(false)}
        onAdd={(name) => {
          setAccounts((accs) => [...accs, { address: uuidv4(), name }]);
          setShowNewAccountsDialog(false);
          setShowAccountsDialog(true);
        }}
      />
    </>
  );
};
