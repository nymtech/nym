import React, { useContext, useState } from 'react';
import { useHistory } from 'react-router';
import { Button, FormControl, Stack } from '@mui/material';
import { useSnackbar } from 'notistack';
import { Subtitle, Title, PasswordStrength } from '../components';
import { PasswordInput } from '../components/textfields';
import { SignInContext } from '../context';
import { createPassword } from '../../../requests';

export const ConnectPassword = () => {
  const { password, setPassword } = useContext(SignInContext);
  const [confirmedPassword, setConfirmedPassword] = useState<string>('');
  const [isStrongPassword, setIsStrongPassword] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  const { mnemonic } = useContext(SignInContext);
  const history = useHistory();

  const { enqueueSnackbar } = useSnackbar();

  const storePassword = async () => {
    try {
      setIsLoading(true);
      await createPassword({ mnemonic, password });
      enqueueSnackbar('Password successfully created', { variant: 'success' });
      setPassword('');
      history.push('/sign-in-password');
    } catch (e) {
      enqueueSnackbar(e as string, { variant: 'error' });
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Stack spacing={3} alignItems="center" minWidth="50%">
      <Title title="Create optional password" />
      <Subtitle subtitle="Password should be min 8 characters, at least one number and one symbol" />
      <FormControl fullWidth>
        <Stack spacing={2}>
          <>
            <PasswordInput
              password={password}
              onUpdatePassword={(pswd) => setPassword(pswd)}
              label="Password"
              autoFocus
            />
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
            Create password
          </Button>
        </Stack>
      </FormControl>
    </Stack>
  );
};
