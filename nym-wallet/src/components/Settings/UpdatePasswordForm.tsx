import React, { useContext, useState } from 'react';
import { Button, FormControl, Stack } from '@mui/material';
import { useSnackbar } from 'notistack';
import { AuthContext } from '../../context';
// import { updatePassword } from '../../requests';
import { PasswordStrength } from '../../pages/auth/components';
import { PasswordInput } from '../textfields';

const UpdatePasswordForm = () => {
  const [currentPassword, setCurrentPassword] = useState<string>('');
  const [confirmedPassword, setConfirmedPassword] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);
  const [isSafePassword, setIsSafePassword] = useState(false);

  const { password, setPassword, resetState, mnemonic } = useContext(AuthContext);

  const { enqueueSnackbar } = useSnackbar();

  const storePassword = async () => {
    try {
      setIsLoading(true);
      // await updatePassword({ mnemonic, currentPassword, newPassword });
      enqueueSnackbar('Password successfully updated', { variant: 'success' });
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
          <PasswordInput
            password={password}
            onUpdatePassword={(pwd) => setCurrentPassword(pwd)}
            label="Current Password"
            autoFocus
          />
          <>
            <PasswordInput password={password} onUpdatePassword={(pwd) => setPassword(pwd)} label="Password" />
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
            Change Password
          </Button>
        </Stack>
      </FormControl>
    </Stack>
  );
};

export default UpdatePasswordForm;
