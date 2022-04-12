import React, { useEffect, useState } from 'react';
import {
  Alert,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  Stack,
  TextField,
  Typography,
} from '@mui/material';
import { Add, Close } from '@mui/icons-material';

const passwordCreationSteps = [
  'Log out',
  'During sign in screen click “Sign in with mnemonic” button',
  'On next screen click “Create a password for your account”',
  'Sign in to wallet with your new password',
  'Now you can create multiple accounts',
];

const NoPassword = () => (
  <Stack spacing={2}>
    <Alert severity="warning" icon={false} sx={{ display: 'block' }}>
      <Typography sx={{ textAlign: 'center' }}>
        You can’t add new accounts if your wallet doesn’t have a password.
      </Typography>
      <Typography sx={{ textAlign: 'center' }}>Follow steps below to create password.</Typography>
    </Alert>
    <Typography variant="h6">How to create password to your account</Typography>
    {passwordCreationSteps.map((step, i) => (
      <Typography>{`${i + 1}. ${step}`}</Typography>
    ))}
  </Stack>
);

export const AddAccountModal = ({
  show,
  withoutPassword,
  onClose,
  onAdd,
}: {
  show: boolean;
  withoutPassword?: boolean;
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
          {withoutPassword ? (
            <NoPassword />
          ) : (
            <TextField
              label="Account name"
              fullWidth
              value={accountName}
              onChange={(e) => setAccountName(e.target.value)}
              autoFocus
            />
          )}
        </Box>
      </DialogContent>
      <DialogActions sx={{ p: 3 }}>
        <Button
          fullWidth
          disableElevation
          variant="contained"
          size="large"
          startIcon={withoutPassword ? undefined : <Add fontSize="small" />}
          onClick={() => (withoutPassword ? onClose() : onAdd(accountName))}
          disabled={!withoutPassword && !accountName.length}
        >
          {withoutPassword ? 'OK' : 'Add'}
        </Button>
      </DialogActions>
    </Dialog>
  );
};
