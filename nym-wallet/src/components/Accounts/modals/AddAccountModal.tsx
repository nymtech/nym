import React, { useContext, useEffect, useState } from 'react';
import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  TextField,
  Typography,
} from '@mui/material';
import { ArrowBackSharp } from '@mui/icons-material';
import { useClipboard } from 'use-clipboard-copy';
import { createMnemonic, validateMnemonic } from 'src/requests';
import { Console } from 'src/utils/console';
import { AccountsContext } from 'src/context';
import { ConfirmPassword, Mnemonic } from 'src/components';
import { MnemonicInput } from 'src/components/textfields';

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
}) => {
  const [error, setError] = useState<string>();

  const handleOnNext = async () => {
    const isValid = await validateMnemonic(value);
    if (!isValid) setError('Please enter a valid mnemonic. Mnemonic must have a word count that is a multiple of 6.');
    else onNext();
  };

  return (
    <>
      <DialogContent>
        <Typography variant="body1" sx={{ color: 'error.main', my: 2 }}>
          {error}
        </Typography>
        <MnemonicInput
          mnemonic={value}
          onUpdateMnemonic={(mnemon) => {
            onChange(mnemon);
            setError(undefined);
          }}
        />
      </DialogContent>
      <DialogActions sx={{ p: 3, pt: 0 }}>
        <Button
          disabled={value.length === 0}
          fullWidth
          disableElevation
          variant="contained"
          size="large"
          onClick={handleOnNext}
        >
          Next
        </Button>
      </DialogActions>
    </>
  );
};

const NameAccount = ({ onNext }: { onNext: (value: string) => void }) => {
  const [value, setValue] = useState('');
  const [error, setError] = useState<string>();

  const nameValidation = /^([a-zA-Z0-9\s]){1,20}$/;

  const handleNext = (accountName: string) => {
    if (!nameValidation.test(accountName)) {
      setError('Account name must  contain only letters and numbers and be between 1 and 20 characters');
    } else onNext(value);
  };

  return (
    <>
      <DialogContent>
        <Typography variant="body1" sx={{ color: 'error.main', my: 2 }}>
          {error}
        </Typography>
        <TextField
          placeholder="Account name"
          value={value}
          onChange={(e) => {
            setValue(e.target.value);
            setError(undefined);
          }}
          fullWidth
        />
      </DialogContent>
      <DialogActions sx={{ p: 3, pt: 0 }}>
        <Button
          disabled={!value.length}
          fullWidth
          disableElevation
          variant="contained"
          size="large"
          onClick={() => handleNext(value)}
        >
          Next
        </Button>
      </DialogActions>
    </>
  );
};

export const AddAccountModal = () => {
  const [step, setStep] = useState(0);
  const [data, setData] = useState({
    mnemonic: '',
    accountName: '',
  });

  const { dialogToDisplay, setDialogToDisplay, handleAddAccount, setError, isLoading, error } =
    useContext(AccountsContext);

  const generateMnemonic = async () => {
    const mnemon = await createMnemonic();
    setData((d) => ({ ...d, mnemonic: mnemon }));
  };

  const resetState = () => {
    setData({ mnemonic: '', accountName: '' });
    setStep(0);
    setError(undefined);
  };

  const handleClose = () => {
    setDialogToDisplay('Accounts');
    resetState();
  };

  useEffect(() => {
    if (dialogToDisplay === 'Add') generateMnemonic();
    if (dialogToDisplay === 'Accounts') resetState();
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
                buttonTitle="Add account"
                onConfirm={async (password) => {
                  if (data.accountName && data.mnemonic) {
                    try {
                      await handleAddAccount({ accountName: data.accountName, mnemonic: data.mnemonic, password });
                      setStep(0);
                      setDialogToDisplay('Accounts');
                    } catch (e) {
                      Console.error(e as string);
                    }
                  }
                }}
                isLoading={isLoading}
                error={error}
              />
            );
          default:
            return null;
        }
      })()}
    </Dialog>
  );
};
