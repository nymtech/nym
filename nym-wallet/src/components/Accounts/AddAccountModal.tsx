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
import { Check, Close, ContentCopySharp } from '@mui/icons-material';
import { useClipboard } from 'use-clipboard-copy';

const createAccountSteps = ['Save and copy mnemonic for your new account', 'Name your new account'];

const passwordCreationSteps = [
  'Log out',
  'During sign in screen click “Sign in with mnemonic” button',
  'On next screen click “Create a password for your account”',
  'Sign in to wallet with your new password',
  'Now you can create multiple accounts',
];

const NoPassword = ({ onClose }: { onClose: () => void }) => (
  <Box sx={{ mt: 1 }}>
    <DialogContent>
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
    </DialogContent>
    <DialogActions sx={{ p: 3 }}>
      <Button fullWidth disableElevation variant="contained" size="large" onClick={onClose}>
        OK
      </Button>
    </DialogActions>
  </Box>
);

const MnemonicStep = ({ mnemonic, onSave }: { mnemonic: string; onSave: () => void }) => {
  const { copy, copied } = useClipboard({ copiedTimeout: 5000 });
  return (
    <Box sx={{ mt: 1 }}>
      <DialogContent>
        <Stack spacing={2} alignItems="center">
          <Alert severity="warning" icon={false} sx={{ display: 'block' }}>
            <Typography sx={{ textAlign: 'center' }}>
              Below is your 24 word mnemonic, make sure to store it in a safe place for accessing your wallet in the
              future
            </Typography>
          </Alert>
          <TextField multiline rows={3} value={mnemonic} fullWidth />

          <Button
            color="inherit"
            disableElevation
            size="large"
            onClick={() => {
              copy(mnemonic);
            }}
            sx={{
              width: 250,
            }}
            endIcon={!copied ? <ContentCopySharp /> : <Check color="success" />}
          >
            Copy mnemonic
          </Button>
        </Stack>
      </DialogContent>
      <DialogActions sx={{ p: 3, pt: 0 }}>
        <Button disabled={!copied} fullWidth disableElevation variant="contained" size="large" onClick={onSave}>
          I saved my mnemonic
        </Button>
      </DialogActions>
    </Box>
  );
};

const NameAccount = ({ onAdd }: { onAdd: (value: string) => void }) => {
  const [value, setValue] = useState('');
  return (
    <Box sx={{ mt: 1 }}>
      <DialogContent>
        <TextField value={value} onChange={(e) => setValue(e.target.value)} fullWidth />
      </DialogContent>
      <DialogActions sx={{ p: 3, pt: 0 }}>
        <Button
          disabled={!value.length}
          fullWidth
          disableElevation
          variant="contained"
          size="large"
          onClick={() => onAdd(value)}
        >
          Add
        </Button>
      </DialogActions>
    </Box>
  );
};

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
  const [step, setStep] = useState(0);

  return (
    <Dialog open={show} onClose={onClose} fullWidth hideBackdrop>
      <DialogTitle>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">Add new account</Typography>
          <IconButton onClick={onClose}>
            <Close />
          </IconButton>
        </Box>
        {!withoutPassword && (
          <Typography variant="body1" sx={{ color: 'grey.600' }}>
            {`Step ${step + 1}/${createAccountSteps.length}`}
          </Typography>
        )}
        <Typography sx={{ mt: 2 }}>{createAccountSteps[step]}</Typography>
      </DialogTitle>
      {withoutPassword && <NoPassword onClose={onClose} />}
      {!withoutPassword &&
        (() => {
          switch (step) {
            case 0:
              return (
                <MnemonicStep
                  mnemonic="lonely employ curtain skull gas swim pizza injury tail birth inmate apart giraffe behave caution hammer echo action best symptom skull toast beyond casino"
                  onSave={() => setStep((s) => s + 1)}
                />
              );
            case 1:
              return <NameAccount onAdd={onAdd} />;
            default:
              return null;
          }
        })()}
    </Dialog>
  );
};
