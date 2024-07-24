import { useContext, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, CircularProgress, FormControl, Stack } from '@mui/material';
import { useSnackbar } from 'notistack';
import { AuthContext } from '@src/context/auth';
import { PasswordInput } from '@nymproject/react';
import { archiveWalletFile, createPassword, isPasswordCreated } from '@src/requests';
import { Subtitle, Title, PasswordStrength } from '../components';

export const ConnectPassword = () => {
  const [confirmedPassword, setConfirmedPassword] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);
  const [isSafePassword, setIsSafePassword] = useState(false);
  const { mnemonic, password, setPassword, resetState } = useContext(AuthContext);
  const navigate = useNavigate();

  const { enqueueSnackbar } = useSnackbar();

  const storePassword = async () => {
    try {
      setIsLoading(true);

      const exists = await isPasswordCreated();
      if (exists) {
        await archiveWalletFile();
      }

      await createPassword({ mnemonic, password });
      resetState();
      enqueueSnackbar('Password successfully created', { variant: 'success' });
      navigate('/sign-in-password');
    } catch (e) {
      enqueueSnackbar(e as string, { variant: 'error' });
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
            <PasswordStrength password={password} handleIsSafePassword={setIsSafePassword} withWarnings />
          </>
          <PasswordInput
            password={confirmedPassword}
            onUpdatePassword={(pswd) => setConfirmedPassword(pswd)}
            label="Confirm password"
          />
          <Button
            size="large"
            variant="contained"
            disabled={password !== confirmedPassword || password.length === 0 || isLoading || !isSafePassword}
            onClick={storePassword}
          >
            {isLoading ? <CircularProgress size={25} /> : 'Create password'}
          </Button>
          <Button
            size="large"
            color="inherit"
            onClick={() => {
              setPassword('');
              navigate(-1);
            }}
          >
            Back
          </Button>
        </Stack>
      </FormControl>
    </Stack>
  );
};
