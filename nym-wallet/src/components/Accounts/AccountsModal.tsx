import React from 'react';
import { Box, Button, Dialog, DialogActions, DialogContent, DialogTitle, IconButton, Typography } from '@mui/material';
import { Add, ArrowDownwardSharp, Close } from '@mui/icons-material';
import { TAccount } from 'src/types';
import { AccountItem } from './AccountItem';

export const AccountsModal = ({
  show,
  accounts,
  selectedAccount,
  onClose,
  onAccountSelect,
  onAdd,
  onEdit,
  onImport,
}: {
  show: boolean;
  accounts: TAccount[];
  selectedAccount: TAccount['address'];
  onClose: () => void;
  onAccountSelect: (account: TAccount) => void;
  onAdd: () => void;
  onEdit: (acc: TAccount) => void;
  onImport: () => void;
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
          onEdit={() => onEdit({ name, address })}
          selected={selectedAccount === address}
          key={address}
        />
      ))}
    </DialogContent>
    <DialogActions sx={{ p: 3 }}>
      <Button startIcon={<ArrowDownwardSharp />} onClick={onImport}>
        Import account
      </Button>
      <Button disableElevation variant="contained" startIcon={<Add fontSize="small" />} onClick={onAdd}>
        Add new account
      </Button>
    </DialogActions>
  </Dialog>
);
