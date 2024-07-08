import React, { useContext, useState } from 'react';
import {
  Box,
  Button,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Paper,
  Typography,
} from '@mui/material';
import { AccountsContext } from '@src/context';
import { PasswordInput } from '@nymproject/react';
import { StyledBackButton } from '@src/components/StyledBackButton';
import { Mnemonic } from '@src/components/Mnemonic';

export const MnemonicModal = () => {
  const [password, setPassword] = useState('');

  const {
    dialogToDisplay,
    setDialogToDisplay,
    accountMnemonic,
    setAccountMnemonic,
    handleGetAccountMnemonic,
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
    <Dialog
      open={dialogToDisplay === 'Mnemonic'}
      onClose={handleClose}
      fullWidth
      PaperComponent={Paper}
      PaperProps={{ elevation: 0 }}
    >
      <DialogTitle>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">Display mnemonic</Typography>
        </Box>
        <Typography fontSize="small" sx={{ color: 'grey.600' }}>
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
            <Mnemonic mnemonic={accountMnemonic.value} />
          )}
        </Box>
      </DialogContent>
      <DialogActions sx={{ p: 3, gap: 2 }}>
        <StyledBackButton onBack={handleClose} />
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
                await handleGetAccountMnemonic({ password, accountName: accountMnemonic?.accountName });
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
