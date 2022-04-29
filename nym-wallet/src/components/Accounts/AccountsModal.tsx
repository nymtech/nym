import React from 'react';
import { Box, Button, Dialog, DialogActions, DialogContent, DialogTitle, IconButton, Typography } from '@mui/material';
import { Add, ArrowDownwardSharp, Close } from '@mui/icons-material';
import { AccountEntry } from 'src/types';
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
  accounts: AccountEntry[];
  selectedAccount: AccountEntry['id'];
  onClose: () => void;
  onAccountSelect: (accountName: string) => void;
  onAdd: () => void;
  onEdit: (accoutnName: string) => void;
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
      {accounts.map(({ id, address }) => (
        <AccountItem
          name={id}
          address={address}
          onSelect={() => {
            onAccountSelect(id);
            onClose();
          }}
          onEdit={() => onEdit(id)}
          isSelected={selectedAccount === id}
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
