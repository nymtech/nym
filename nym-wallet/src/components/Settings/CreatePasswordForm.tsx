import React, { useContext, useState } from 'react';
import { Button, FormControl, Stack } from '@mui/material';
import { useSnackbar } from 'notistack';
import { AuthContext } from '../../context';
import { createPassword } from '../../requests';
import { PasswordStrength } from '../../pages/auth/components';
import { PasswordInput } from '../textfields';

const CreatePasswordForm = () => {
  const [confirmedPassword, setConfirmedPassword] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);
  const [isSafePassword, setIsSafePassword] = useState(false);

  const { password, setPassword, resetState, mnemonic } = useContext(AuthContext);

  const { enqueueSnackbar } = useSnackbar();

  const storePassword = async () => {
    try {
      setIsLoading(true);
      await createPassword({ mnemonic, password });
      enqueueSnackbar('Password successfully created', { variant: 'success' });
      resetState();
    } catch (e) {
      setIsLoading(false);
      enqueueSnackbar(e as string, { variant: 'error' });
    }
  };

  return (
    <Stack spacing={3} alignItems="center" minWidth="50%">
      <FormControl fullWidth>
        <Stack spacing={2}>
          <>
            <PasswordInput
              password={password}
              onUpdatePassword={(pwd) => setPassword(pwd)}
              label="Password"
              autoFocus
            />
            <PasswordStrength password={password} handleIsSafePassword={setIsSafePassword} withWarnings />
          </>
          <PasswordInput
            password={confirmedPassword}
            onUpdatePassword={(pwd) => setConfirmedPassword(pwd)}
            label="Confirm password"
          />
          <Button
            size="large"
            variant="contained"
            disabled={password !== confirmedPassword || password.length === 0 || isLoading || !isSafePassword}
            onClick={storePassword}
          >
            Create Password
          </Button>
        </Stack>
      </FormControl>
    </Stack>
  );
};

export default CreatePasswordForm;
