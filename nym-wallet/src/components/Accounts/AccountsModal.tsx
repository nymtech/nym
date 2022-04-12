import React from 'react';
import { Box, Button, Dialog, DialogActions, DialogContent, DialogTitle, IconButton, Typography } from '@mui/material';
import { Add, Close } from '@mui/icons-material';
import { AccountItem } from './AccountItem';
import { TAccount } from './types';

export const AccountsModal = ({
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
