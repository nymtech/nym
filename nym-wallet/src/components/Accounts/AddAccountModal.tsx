import React, { useContext, useEffect, useState } from 'react';
import {
  Alert,
  Box,
  Button,
  CircularProgress,
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
import { createMnemonic } from 'src/requests';
import { AccountsContext } from 'src/context';

const createAccountSteps = ['Save and copy mnemonic for your new account', 'Name your new account', 'Confirm password'];
const importAccountSteps = [
  'Provide mnemonic of account you want to import',
  'Name your new account',
  'Confirm password',
];

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

const MnemonicStep = ({ mnemonic, onNext }: { mnemonic: string; onNext: () => void }) => {
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
        <Button disabled={!copied} fullWidth disableElevation variant="contained" size="large" onClick={onNext}>
          I saved my mnemonic
        </Button>
      </DialogActions>
    </Box>
  );
};

const ImportMnemonic = ({
  value,
  onChange,
  onNext,
}: {
  value: string;
  onChange: (value: string) => void;
  onNext: () => void;
}) => (
  <Box sx={{ mt: 1 }}>
    <DialogContent>
      <Stack spacing={2} alignItems="center">
        <TextField multiline rows={3} value={value} onChange={(e) => onChange(e.target.value)} fullWidth />
      </Stack>
    </DialogContent>
    <DialogActions sx={{ p: 3, pt: 0 }}>
      <Button
        disabled={value.length === 0}
        fullWidth
        disableElevation
        variant="contained"
        size="large"
        onClick={onNext}
      >
        Next
      </Button>
    </DialogActions>
  </Box>
);

const NameAccount = ({ onPrev, onNext }: { onPrev: () => void; onNext: (value: string) => void }) => {
  const [value, setValue] = useState('');
  return (
    <Box sx={{ mt: 1 }}>
      <DialogContent>
        <TextField value={value} onChange={(e) => setValue(e.target.value)} fullWidth />
      </DialogContent>
      <DialogActions sx={{ p: 3, pt: 0 }}>
        <Button fullWidth size="large" onClick={onPrev}>
          Back
        </Button>
        <Button
          disabled={!value.length}
          fullWidth
          disableElevation
          variant="contained"
          size="large"
          onClick={() => onNext(value)}
        >
          Next
        </Button>
      </DialogActions>
    </Box>
  );
};

const ConfirmPassword = ({ onPrev, onConfirm }: { onPrev: () => void; onConfirm: (password: string) => void }) => {
  const [value, setValue] = useState('');
  const { isLoading } = useContext(AccountsContext);

  return (
    <Box sx={{ mt: 1 }}>
      <DialogContent>
        <TextField value={value} onChange={(e) => setValue(e.target.value)} fullWidth />
      </DialogContent>
      <DialogActions sx={{ p: 3, pt: 0 }}>
        <Button fullWidth size="large" onClick={onPrev}>
          Back
        </Button>
        <Button
          disabled={!value.length || isLoading}
          fullWidth
          disableElevation
          variant="contained"
          size="large"
          onClick={() => onConfirm(value)}
          endIcon={isLoading && <CircularProgress size={20} />}
        >
          Add account
        </Button>
      </DialogActions>
    </Box>
  );
};

export const AddAccountModal = ({ withoutPassword }: { withoutPassword?: boolean }) => {
  const [step, setStep] = useState(0);
  const [data, setData] = useState({
    mnemonic: '',
    accountName: '',
  });

  const { dialogToDisplay, setDialogToDisplay, handleAddAccount } = useContext(AccountsContext);

  const generateMnemonic = async () => {
    const mnemon = await createMnemonic();
    setData((d) => ({ ...d, mnemonic: mnemon }));
  };

  const handleClose = () => {
    setDialogToDisplay('Accounts');
    setData({ mnemonic: '', accountName: '' });
    setStep(0);
  };

  useEffect(() => {
    if (dialogToDisplay === 'Add') generateMnemonic();
    else setData({ mnemonic: '', accountName: '' });
  }, [dialogToDisplay]);

  return (
    <Dialog
      open={dialogToDisplay === 'Add' || dialogToDisplay === 'Import'}
      onClose={handleClose}
      fullWidth
      hideBackdrop
    >
      <DialogTitle>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">{`${dialogToDisplay} new account`}</Typography>
          <IconButton onClick={handleClose}>
            <Close />
          </IconButton>
        </Box>
        {!withoutPassword && (
          <Typography variant="body1" sx={{ color: 'grey.600' }}>
            {`Step ${step + 1}/${createAccountSteps.length}`}
          </Typography>
        )}
        <Typography sx={{ mt: 2 }}>
          {dialogToDisplay === 'Add' ? createAccountSteps[step] : importAccountSteps[step]}
        </Typography>
      </DialogTitle>
      {withoutPassword && <NoPassword onClose={handleClose} />}
      {!withoutPassword &&
        (() => {
          switch (step) {
            case 0:
              return dialogToDisplay === 'Add' ? (
                <MnemonicStep mnemonic={data.mnemonic} onNext={() => setStep((s) => s + 1)} />
              ) : (
                <ImportMnemonic
                  value={data.mnemonic}
                  onChange={(value) => setData((d) => ({ ...d, mnemonic: value }))}
                  onNext={() => setStep((s) => s + 1)}
                />
              );
            case 1:
              return (
                <NameAccount
                  onPrev={() => setStep((s) => s - 1)}
                  onNext={(accountName) => {
                    setData((d) => ({ ...d, accountName }));
                    setStep((s) => s + 1);
                  }}
                />
              );
            case 2:
              return (
                <ConfirmPassword
                  onPrev={() => setStep((s) => s - 1)}
                  onConfirm={(password) => {
                    if (data.accountName && data.mnemonic) {
                      handleAddAccount({ accountName: data.accountName, mnemonic: data.mnemonic, password });
                    }
                  }}
                />
              );
            default:
              return null;
          }
        })()}
    </Dialog>
  );
};
