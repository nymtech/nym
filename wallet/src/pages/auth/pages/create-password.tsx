import { useContext, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, FormControl, Stack } from '@mui/material';
import { useSnackbar } from 'notistack';
import { AuthContext } from '@src/context/auth';
import { createPassword } from '@src/requests';
import { PasswordInput } from '@nymproject/react';
import { Subtitle, Title, PasswordStrength } from '../components';

export const CreatePassword = () => {
  const { password, setPassword, resetState, mnemonic } = useContext(AuthContext);
  const [confirmedPassword, setConfirmedPassword] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);
  const [isSafePassword, setIsSafePassword] = useState(false);

  const navigate = useNavigate();

  const handleSkip = () => {
    setPassword('');
    navigate('/sign-in-mnemonic');
  };

  const { enqueueSnackbar } = useSnackbar();

  const storePassword = async () => {
    try {
      setIsLoading(true);
      await createPassword({ mnemonic, password });
      enqueueSnackbar('Password successfully created', { variant: 'success' });
      resetState();
      navigate('/sign-in-password');
    } catch (e) {
      setIsLoading(false);
      enqueueSnackbar(e as string, { variant: 'error' });
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
