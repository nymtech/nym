import React, { useContext, useState } from 'react';
import {
  Box,
  Button,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  Typography,
} from '@mui/material';
import { ArrowBackSharp } from '@mui/icons-material';
import { AccountsContext } from 'src/context';
import { useClipboard } from 'use-clipboard-copy';
import { Mnemonic } from '../Mnemonic';
import { PasswordInput } from '../textfields';

export const MnemonicModal = () => {
  const [password, setPassword] = useState('');

  const { copy, copied } = useClipboard({ copiedTimeout: 5000 });

  const {
    dialogToDisplay,
    setDialogToDisplay,
    accountMnemonic,
    setAccountMnemonic,
    handleGetAcccountMnemonic,
    error,
    setError,
    isLoading,
  } = useContext(AccountsContext);

  const handleClose = () => {
    setAccountMnemonic({ value: undefined, accountName: undefined });
    setError(undefined);
    setDialogToDisplay('Accounts');
    setPassword('');
  };

  return (
    <Dialog open={dialogToDisplay === 'Mnemonic'} onClose={handleClose} fullWidth hideBackdrop>
      <DialogTitle>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">Display mnemonic</Typography>
          <IconButton onClick={handleClose}>
            <ArrowBackSharp />
          </IconButton>
        </Box>
        <Typography variant="body1" sx={{ color: 'grey.600' }}>
          {`Display mnemonic for: ${accountMnemonic?.accountName}`}
        </Typography>
      </DialogTitle>
      <DialogContent sx={{ p: 0 }}>
        <Box sx={{ px: 3, mt: 1 }}>
          {error && (
            <Typography variant="body1" sx={{ color: 'error.main', mb: 2 }}>
              {error}
            </Typography>
          )}
          {!accountMnemonic.value ? (
            <>
              <Typography sx={{ mb: 2 }}>Enter the password used to login to your wallet</Typography>
              <PasswordInput
                label="Password"
                password={password}
                onUpdatePassword={(pswrd) => setPassword(pswrd)}
                autoFocus
              />
            </>
          ) : (
            <Mnemonic mnemonic={accountMnemonic.value} handleCopy={copy} copied={copied} />
          )}
        </Box>
      </DialogContent>
      <DialogActions sx={{ p: 3 }}>
        {!accountMnemonic.value && (
          <Button
            disableRipple
            disabled={!password.length || isLoading}
            fullWidth
            disableElevation
            variant="contained"
            size="large"
            onClick={async () => {
              if (accountMnemonic?.accountName) {
                setError(undefined);
                await handleGetAcccountMnemonic({ password, accountName: accountMnemonic?.accountName });
              }
            }}
            endIcon={isLoading && <CircularProgress size={20} />}
          >
            Display mnemonic
          </Button>
        )}
      </DialogActions>
    </Dialog>
  );
};
