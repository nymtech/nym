import { useState } from 'react';
import { Button, FormControl, Stack } from '@mui/material';
import { useSnackbar } from 'notistack';
import { PasswordInput } from '@nymproject/react';
import { updatePassword } from '../../requests';
import { PasswordStrength } from '../../pages/auth/components';

const PasswordUpdateForm = ({ onPwdSaved }: { onPwdSaved: () => void }) => {
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmedPassword, setConfirmedPassword] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);
  const [isSafePassword, setIsSafePassword] = useState(false);

  const { enqueueSnackbar } = useSnackbar();

  const reset = () => {
    setCurrentPassword('');
    setNewPassword('');
    setConfirmedPassword('');
  };

  const savePassword = async () => {
    try {
      setIsLoading(true);
      await updatePassword({ currentPassword, newPassword });
      reset();
      onPwdSaved();
    } catch (e) {
      enqueueSnackbar(e as string, { variant: 'error' });
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Stack spacing={3} alignItems="center">
      <FormControl fullWidth>
        <Stack spacing={3} mt={2}>
          <PasswordInput
            password={currentPassword}
            onUpdatePassword={(pwd) => setCurrentPassword(pwd)}
            label="Current password"
            autoFocus
          />
          <PasswordInput password={newPassword} onUpdatePassword={(pwd) => setNewPassword(pwd)} label="New password" />
          <PasswordStrength password={newPassword} handleIsSafePassword={setIsSafePassword} withWarnings />
          <PasswordInput
            password={confirmedPassword}
            onUpdatePassword={(pwd) => setConfirmedPassword(pwd)}
            label="Confirm password"
          />
          <Button
            size="large"
            variant="contained"
            disabled={
              currentPassword.length === 0 ||
              currentPassword === newPassword ||
              newPassword !== confirmedPassword ||
              newPassword.length === 0 ||
              isLoading ||
              !isSafePassword
            }
            onClick={savePassword}
          >
            Save New Password
          </Button>
        </Stack>
      </FormControl>
    </Stack>
  );
};

export default PasswordUpdateForm;
