import React from 'react';
import {
  Alert,
  Box,
  Paper,
  Dialog,
  DialogContent,
  DialogTitle,
  IconButton,
  Stack,
  Typography,
  Button,
} from '@mui/material';
import { Close } from '@mui/icons-material';

const passwordCreationSteps = [
  'Log out',
  'When signing in, select “Sign in with mnemonic”',
  'On the next screen click “Create a password for your account”',
  'Sign in to wallet with your new password',
  'Now you can create multiple accounts',
];

const MultiAccountPasswordExists = ({ show, handleClose }: { show: boolean; handleClose: () => void }) => (
  <Dialog open={show} onClose={handleClose} fullWidth PaperComponent={Paper} PaperProps={{ elevation: 0 }}>
    <DialogTitle>
      <Box display="flex" justifyContent="space-between" alignItems="center">
        <Typography variant="h6" fontWeight={600}>
          Add new account
        </Typography>
        <IconButton onClick={handleClose}>
          <Close />
        </IconButton>
      </Box>
    </DialogTitle>
    <DialogContent>
      <Stack spacing={2}>
        <Alert
          severity="warning"
          icon={false}
          sx={(t) => (t.palette.mode === 'dark' ? { bgcolor: (theme) => theme.palette.background.paper } : {})}
        >
          <Typography fontWeight={600} align="center" marginBottom={1}>
            In order to create multiple accounts your wallet need password. Follow steps below to create password.{' '}
          </Typography>
          <Typography align="center">
            if you had created a password on this machine before, creating a new password for this account will
            overwrite old one.
          </Typography>
        </Alert>
        <Typography fontWeight={600}>How to create a password for your account</Typography>
        {passwordCreationSteps.map((step, index) => (
          <Typography key={step} sx={{ display: 'flex' }}>
            <Box fontWeight={600}>{index + 1}</Box>. {step}
          </Typography>
        ))}
        <Button fullWidth disableElevation variant="contained" size="large" onClick={handleClose}>
          ok
        </Button>
      </Stack>
    </DialogContent>
  </Dialog>
);

const MultiAccountPasswordNonExistent = ({ show, handleClose }: { show: boolean; handleClose: () => void }) => (
  <Dialog open={show} onClose={handleClose} fullWidth PaperComponent={Paper} PaperProps={{ elevation: 0 }}>
    <DialogTitle>
      <Box display="flex" justifyContent="space-between" alignItems="center">
        <Typography variant="h6" fontWeight={600}>
          Multi accounts
        </Typography>
        <IconButton onClick={handleClose}>
          <Close />
        </IconButton>
      </Box>
      <Typography variant="body1" sx={{ color: (theme) => theme.palette.nym.text.muted }}>
        How to set up multiple accounts
      </Typography>
    </DialogTitle>
    <DialogContent>
      <Stack spacing={2}>
        <Alert
          severity="warning"
          icon={false}
          sx={(t) => (t.palette.mode === 'dark' ? { bgcolor: (theme) => theme.palette.background.paper } : {})}
        >
          <Typography fontWeight={600} align="center" marginBottom={1}>
            In order to create multiple accounts your wallet needs a password.
          </Typography>
          <Typography align="center">Follow steps below to create password.</Typography>
        </Alert>
        <Typography fontWeight={600}>How to create a password for your account</Typography>
        {passwordCreationSteps.map((step, index) => (
          <Typography key={step} sx={{ display: 'flex' }}>
            <Box fontWeight={600}>{index + 1}</Box>. {step}
          </Typography>
        ))}
      </Stack>
    </DialogContent>
  </Dialog>
);

export const MultiAccountHowTo = ({
  show,
  handleClose,
  passwordExists,
}: {
  show: boolean;
  handleClose: () => void;
  passwordExists: boolean;
}) =>
  passwordExists ? (
    <MultiAccountPasswordExists show={show} handleClose={handleClose} />
  ) : (
    <MultiAccountPasswordNonExistent show={show} handleClose={handleClose} />
  );
