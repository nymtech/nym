import React, { useContext, useState } from 'react';
import {
  Box,
  Button,
  Paper,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  Typography,
  Divider,
} from '@mui/material';
import { Add, ArrowDownwardSharp, Close } from '@mui/icons-material';
import { useTheme } from '@mui/material/styles';
import { AccountsContext } from '@src/context';
import { AccountItem } from '../AccountItem';
import { ConfirmPasswordModal } from './ConfirmPasswordModal';

export const AccountsModal = () => {
  const { accounts, dialogToDisplay, setDialogToDisplay, setError, handleSelectAccount, selectedAccount } =
    useContext(AccountsContext);
  const [accountToSwitchTo, setAccountToSwitchTo] = useState<string>();

  const theme = useTheme();

  const handleClose = () => {
    setDialogToDisplay(undefined);
    setError(undefined);
    setAccountToSwitchTo(undefined);
  };

  if (accountToSwitchTo)
    return (
      <ConfirmPasswordModal
        modalTitle="Switch account"
        accountName={accountToSwitchTo}
        buttonTitle="Switch account"
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
    <Dialog
      open={dialogToDisplay === 'Accounts'}
      onClose={handleClose}
      fullWidth
      PaperProps={{
        style: { border: `1px solid ${theme.palette.nym.nymWallet.modal.border}` },
      }}
    >
      <Paper>
        <DialogTitle>
          <Box display="flex" justifyContent="space-between" alignItems="center">
            <Typography variant="h6">Accounts</Typography>
            <IconButton onClick={handleClose}>
              <Close />
            </IconButton>
          </Box>
          <Typography fontSize="small" sx={{ color: 'grey.600' }}>
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
        <Divider variant="middle" sx={{ mt: 3 }} />
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
            Create account
          </Button>
        </DialogActions>
      </Paper>
    </Dialog>
  );
};
