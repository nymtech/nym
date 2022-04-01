import React, { useContext, useState } from 'react';
import { Button, Stack } from '@mui/material';
import { MnemonicInput, Subtitle } from '../components';
import { SignInContext } from '../context';
import { useHistory } from 'react-router';

export const ConfirmMnemonic = () => {
  const { validateMnemonic, setMnemonic, error } = useContext(SignInContext);
  const [localMnemonic, setLocalMnemonic] = useState('');
  const history = useHistory();

  return (
    <Stack spacing={2}>
      <Subtitle subtitle="Enter the mnemonic you wish to create a password for" />
      <MnemonicInput mnemonic={localMnemonic} onUpdateMnemonic={(mnc) => setLocalMnemonic(mnc)} />
      <Button
        size="large"
        variant="contained"
        fullWidth
        onClick={async () => {
          setMnemonic(localMnemonic);
          history.push('/connect-password');
        }}
        disabled={localMnemonic.length === 0 || !!error}
      >
        Next
      </Button>
    </Stack>
  );
};
