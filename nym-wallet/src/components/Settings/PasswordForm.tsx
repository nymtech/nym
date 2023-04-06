import React, { useState } from 'react';
import { Button, FormControl, Stack } from '@mui/material';
import { useSnackbar } from 'notistack';
import { createPassword, updatePassword } from '../../requests';
import { PasswordStrength } from '../../pages/auth/components';
import { MnemonicInput, PasswordInput } from '../textfields';
import { Error } from '../Error';

const PasswordForm = ({ mode, onPwdSaved }: { mode: 'create' | 'update'; onPwdSaved: () => void }) => {
  const [mnemonic, setMnemonic] = useState('');
  const [password, setPassword] = useState('');
  const [confirmedPassword, setConfirmedPassword] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);
  const [isSafePassword, setIsSafePassword] = useState(false);

  const { enqueueSnackbar } = useSnackbar();

  const reset = () => {
    setMnemonic('');
    setPassword('');
    setConfirmedPassword('');
  };

  const savePassword = async () => {
    try {
      setIsLoading(true);
      if (mode === 'create') {
        await createPassword({ mnemonic, password });
      } else {
        await updatePassword({ mnemonic, password });
      }
      enqueueSnackbar(`Password successfully ${mode === 'create' ? 'created' : 'updated'}`, { variant: 'success' });
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
          {mode === 'update' && (
            <Error
              message="Creating a new password will overwrite any old one stored on your machine.
            Make sure you have saved any mnemonics associated with the password before creating a new one."
            />
          )}
          <MnemonicInput mnemonic={mnemonic} onUpdateMnemonic={(m) => setMnemonic(m)} />
          <PasswordInput
            password={password}
            onUpdatePassword={(pwd) => setPassword(pwd)}
            label="New password"
            autoFocus
          />
          <PasswordStrength password={password} handleIsSafePassword={setIsSafePassword} withWarnings />
          <PasswordInput
            password={confirmedPassword}
            onUpdatePassword={(pwd) => setConfirmedPassword(pwd)}
            label="Confirm password"
          />
          <Button
            size="large"
            variant="contained"
            disabled={password !== confirmedPassword || password.length === 0 || isLoading || !isSafePassword}
            onClick={savePassword}
          >
            Save New Password
          </Button>
        </Stack>
      </FormControl>
    </Stack>
  );
};

export default PasswordForm;
