import React from 'react';
import { Alert, Box, Dialog, DialogContent, DialogTitle, IconButton, Stack, Typography } from '@mui/material';
import { Close } from '@mui/icons-material';

const passwordCreationSteps = [
  'Log out',
  'When signing in, select “Sign in with mnemonic”',
  'On the next screen click “Create a password for your account”',
  'Sign in to wallet with your new password',
  'Now you can create multiple accounts',
];

export const MultiAccountHowTo = ({ show, handleClose }: { show: boolean; handleClose: () => void }) => (
  <Dialog open={show} onClose={handleClose} fullWidth hideBackdrop>
    <DialogTitle>
      <Box display="flex" justifyContent="space-between" alignItems="center">
        <Typography variant="h6">Multi accounts</Typography>
        <IconButton onClick={handleClose}>
          <Close />
        </IconButton>
      </Box>
      <Typography variant="body1" sx={{ color: 'grey.600' }}>
        How to set up multiple accounts
      </Typography>
    </DialogTitle>
    <DialogContent>
      <Stack spacing={2}>
        <Alert severity="warning" icon={false}>
          <Typography>In order to create multiple accounts your wallet need password.</Typography>
          <Typography>Follow steps below to create password.</Typography>
        </Alert>
        <Typography>How to create a password for your account</Typography>
        {passwordCreationSteps.map((step, index) => (
          <Typography key={step}>{`${index + 1}. ${step}`}</Typography>
        ))}
      </Stack>
    </DialogContent>
  </Dialog>
);
