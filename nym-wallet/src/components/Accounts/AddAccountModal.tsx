import React, { useContext, useEffect, useState } from 'react';
import {
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
import { ArrowBackSharp } from '@mui/icons-material';
import { useClipboard } from 'use-clipboard-copy';
import { createMnemonic } from 'src/requests';
import { AccountsContext } from 'src/context';
import { Mnemonic } from '../Mnemonic';

const createAccountSteps = [
  'Copy and save mnemonic for your new account',
  'Name your new account',
  'Confirm the password used to login to your wallet',
];
const importAccountSteps = [
  'Provide mnemonic of account you want to import',
  'Name your new account',
  'Confirm the password used to login to your wallet',
];

const MnemonicStep = ({ mnemonic, onNext }: { mnemonic: string; onNext: () => void }) => {
  const { copy, copied } = useClipboard({ copiedTimeout: 5000 });
  return (
    <Box sx={{ mt: 1 }}>
      <DialogContent>
        <Mnemonic mnemonic={mnemonic} handleCopy={copy} copied={copied} />
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
        <TextField
          multiline
          rows={3}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          fullWidth
          type="password"
        />
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

const NameAccount = ({ onNext }: { onNext: (value: string) => void }) => {
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
          onClick={() => onNext(value)}
        >
          Next
        </Button>
      </DialogActions>
    </Box>
  );
};

const ConfirmPassword = ({ onConfirm }: { onConfirm: (password: string) => void }) => {
  const [value, setValue] = useState('');
  const { isLoading, error } = useContext(AccountsContext);

  return (
    <Box sx={{ mt: 1 }}>
      <DialogContent>
        {error && (
          <Typography variant="body1" sx={{ color: 'error.main', mb: 2 }}>
            {error}
          </Typography>
        )}
        <TextField value={value} onChange={(e) => setValue(e.target.value)} fullWidth type="password" />
      </DialogContent>
      <DialogActions sx={{ p: 3, pt: 0 }}>
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

export const AddAccountModal = () => {
  const [step, setStep] = useState(0);
  const [data, setData] = useState({
    mnemonic: '',
    accountName: '',
  });

  const { dialogToDisplay, setDialogToDisplay, handleAddAccount, setError } = useContext(AccountsContext);

  const generateMnemonic = async () => {
    const mnemon = await createMnemonic();
    setData((d) => ({ ...d, mnemonic: mnemon }));
  };

  const handleClose = () => {
    setDialogToDisplay('Accounts');
    setData({ mnemonic: '', accountName: '' });
    setStep(0);
    setError(undefined);
  };

  useEffect(() => {
    if (dialogToDisplay === 'Add') generateMnemonic();
  }, [dialogToDisplay]);

  useEffect(() => {
    setError(undefined);
  }, [step]);

  return (
    <Dialog
      open={dialogToDisplay === 'Add' || dialogToDisplay === 'Import'}
      onClose={handleClose}
      fullWidth
      hideBackdrop
    >
      <DialogTitle sx={{ pb: 0 }}>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">{`${dialogToDisplay} new account`}</Typography>
          <IconButton onClick={() => (step === 0 ? handleClose() : setStep((s) => s - 1))}>
            <ArrowBackSharp />
          </IconButton>
        </Box>
        <Typography sx={{ mt: 2 }}>
          {dialogToDisplay === 'Add' ? createAccountSteps[step] : importAccountSteps[step]}
        </Typography>
      </DialogTitle>
      {(() => {
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
                onNext={(accountName) => {
                  setData((d) => ({ ...d, accountName }));
                  setStep((s) => s + 1);
                }}
              />
            );
          case 2:
            return (
              <ConfirmPassword
                onConfirm={async (password) => {
                  if (data.accountName && data.mnemonic) {
                    await handleAddAccount({ accountName: data.accountName, mnemonic: data.mnemonic, password });
                    setStep(0);
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
