import React from 'react';
import { Alert, Box, Paper, Dialog, DialogContent, DialogTitle, IconButton, Stack, Typography } from '@mui/material';
import { Close } from '@mui/icons-material';

const passwordCreationSteps = [
  'Log out',
  'When signing in, select “Sign in with mnemonic”',
  'On the next screen click “Create a password for your account”',
  'Sign in to wallet with your new password',
  'Now you can create multiple accounts',
];

export const MultiAccountHowTo = ({ show, handleClose }: { show: boolean; handleClose: () => void }) => (
  <Dialog open={show} onClose={handleClose} fullWidth>
    <Paper>
      <DialogTitle>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">Multi accounts</Typography>
          <IconButton onClick={handleClose}>
            <Close />
          </IconButton>
        </Box>
        <Typography variant="body1" sx={{ color: (t) => t.palette.nym.text.muted }}>
          How to set up multiple accounts
        </Typography>
      </DialogTitle>
      <DialogContent>
        <Stack spacing={2}>
          <Alert
            severity="warning"
            icon={false}
            sx={(t) => (t.palette.mode === 'dark' ? { bgcolor: (t) => t.palette.background.paper } : {})}
          >
            <Typography>In order to create multiple accounts your wallet needs a password.</Typography>
            <Typography>Follow steps below to create password.</Typography>
          </Alert>
          <Typography>How to create a password for your account</Typography>
          {passwordCreationSteps.map((step, index) => (
            <Typography key={step}>{`${index + 1}. ${step}`}</Typography>
          ))}
        </Stack>
      </DialogContent>
    </Paper>
  </Dialog>
);
