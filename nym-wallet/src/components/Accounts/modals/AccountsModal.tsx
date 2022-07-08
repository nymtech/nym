import React, { useContext, useState } from 'react';
import { Box, Button, Dialog, DialogActions, DialogContent, DialogTitle, IconButton, Typography } from '@mui/material';
import { Add, ArrowDownwardSharp, Close } from '@mui/icons-material';
import { AccountsContext } from 'src/context';
import { AccountItem } from '../AccountItem';
import { ConfirmPasswordModal } from './ConfirmPasswordModal';

export const AccountsModal = () => {
  const { accounts, dialogToDisplay, setDialogToDisplay, setError, handleSelectAccount, selectedAccount } =
    useContext(AccountsContext);
  const [accountToSwitchTo, setAccountToSwitchTo] = useState<string>();

  const handleClose = () => {
    setDialogToDisplay(undefined);
    setError(undefined);
    setAccountToSwitchTo(undefined);
  };

  if (accountToSwitchTo)
    return (
      <ConfirmPasswordModal
        accountName={accountToSwitchTo}
        onClose={() => {
          handleClose();
          setDialogToDisplay('Accounts');
        }}
        onConfirm={async (password) => {
          const isSuccessful = await handleSelectAccount({ password, accountName: accountToSwitchTo });
          if (isSuccessful) handleClose();
        }}
      />
    );

  return (
    <Dialog open={dialogToDisplay === 'Accounts'} onClose={handleClose} fullWidth hideBackdrop>
      <DialogTitle>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">Accounts</Typography>
          <IconButton onClick={handleClose}>
            <Close />
          </IconButton>
        </Box>
        <Typography variant="body1" sx={{ color: (theme) => theme.palette.text.disabled }}>
          Switch between accounts
        </Typography>
      </DialogTitle>
      <DialogContent sx={{ padding: 0 }}>
        {accounts?.map(({ id, address }) => (
          <AccountItem
            name={id}
            address={address}
            key={address}
            onSelectAccount={() => {
              if (selectedAccount?.id !== id) {
                setAccountToSwitchTo(id);
              }
            }}
          />
        ))}
      </DialogContent>
      <DialogActions sx={{ p: 3 }}>
        <Button startIcon={<ArrowDownwardSharp />} onClick={() => setDialogToDisplay('Import')}>
          Import account
        </Button>
        <Button
          disableElevation
          variant="contained"
          startIcon={<Add fontSize="small" />}
          onClick={() => setDialogToDisplay('Add')}
        >
          Add new account
        </Button>
      </DialogActions>
    </Dialog>
  );
};
