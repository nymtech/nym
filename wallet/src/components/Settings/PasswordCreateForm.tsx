import { useState } from 'react';
import { Button, FormControl, Stack } from '@mui/material';
import { useSnackbar } from 'notistack';
import { MnemonicInput } from '@nymproject/react';
import { PasswordInput } from '@nymproject/react';
import { createPassword } from '../../requests';
import { PasswordStrength } from '../../pages/auth/components';

const PasswordCreateForm = ({ onPwdSaved }: { onPwdSaved: () => void }) => {
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
      await createPassword({ mnemonic, password });
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
          <MnemonicInput mnemonic={mnemonic} onUpdateMnemonic={(m) => setMnemonic(m)} />
          <PasswordInput password={password} onUpdatePassword={(pwd) => setPassword(pwd)} label="Password" />
          <PasswordStrength password={password} handleIsSafePassword={setIsSafePassword} withWarnings />
          <PasswordInput
            password={confirmedPassword}
            onUpdatePassword={(pwd) => setConfirmedPassword(pwd)}
            label="Confirm password"
          />
          <Button
            size="large"
            variant="contained"
            disabled={
              mnemonic.length === 0 ||
              password !== confirmedPassword ||
              password.length === 0 ||
              isLoading ||
              !isSafePassword
            }
            onClick={savePassword}
          >
            Save Password
          </Button>
        </Stack>
      </FormControl>
    </Stack>
  );
};

export default PasswordCreateForm;
