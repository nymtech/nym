import React, { useContext, useState } from 'react';
import { Button, FormControl, Stack } from '@mui/material';
import { useSnackbar } from 'notistack';
import { TPages } from '../types';
import { Subtitle, Title, PasswordStrength } from '../components';
import { PasswordInput } from '../components/textfields';
import { SignInContext } from '../context';
import { createPassword } from '../../../requests';

export const CreatePassword = ({ onSkip, onNext }: { page: TPages; onNext: () => void; onSkip: () => void }) => {
  const { password, setPassword } = useContext(SignInContext);
  const [confirmedPassword, setConfirmedPassword] = useState<string>('');
  const [isStrongPassword, setIsStrongPassword] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  const { mnemonic } = useContext(SignInContext);

  const handleSkip = () => {
    setPassword('');
    onSkip();
  };

  const { enqueueSnackbar } = useSnackbar();

  const storePassword = async () => {
    try {
      setIsLoading(true);
      await createPassword({ mnemonic, password });
      enqueueSnackbar('Password successfully created', { variant: 'success' });
      setPassword('');
      onNext();
    } catch (e) {
      enqueueSnackbar(e as string, { variant: 'error' });
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Stack spacing={3} alignItems="center" minWidth="50%">
      <Title title="Create password" />
      <Subtitle subtitle="Create a strong password. Min 8 characters, at least one capital letter, number and special symbol" />
      <FormControl fullWidth>
        <Stack spacing={2}>
          <>
            <PasswordInput password={password} onUpdatePassword={(pswd) => setPassword(pswd)} label="Password" />
            <PasswordStrength password={password} onChange={(isStrong) => setIsStrongPassword(isStrong)} />
          </>
          <PasswordInput
            password={confirmedPassword}
            onUpdatePassword={(pswd) => setConfirmedPassword(pswd)}
            label="Confirm password"
          />
          <Button
            size="large"
            variant="contained"
            disabled={password !== confirmedPassword || password.length === 0 || !isStrongPassword || isLoading}
            onClick={storePassword}
          >
            Next
          </Button>
          <Button size="large" color="info" onClick={handleSkip}>
            Skip and sign in with mnemonic
          </Button>
        </Stack>
      </FormControl>
    </Stack>
  );
};
