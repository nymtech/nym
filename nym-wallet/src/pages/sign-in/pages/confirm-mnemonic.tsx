import React, { useContext, useEffect, useState } from 'react';
import { useHistory } from 'react-router-dom';
import { Button, Stack } from '@mui/material';
import { validateMnemonic } from 'src/requests';
import { MnemonicInput, Subtitle } from '../components';
import { SignInContext } from '../context';

export const ConfirmMnemonic = () => {
  const { error, setError, setMnemonic } = useContext(SignInContext);
  const [localMnemonic, setLocalMnemonic] = useState('');
  const history = useHistory();

  useEffect(() => {
    setError(undefined);
  }, [localMnemonic]);

  return (
    <Stack spacing={2}>
      <Subtitle subtitle="Enter the mnemonic you wish to create a password for" />
      <MnemonicInput mnemonic={localMnemonic} onUpdateMnemonic={(mnc) => setLocalMnemonic(mnc)} error={error} />
      <Button
        size="large"
        variant="contained"
        fullWidth
        onClick={async () => {
          const isValid = await validateMnemonic(localMnemonic);
          if (isValid) {
            setMnemonic(localMnemonic);
            history.push('/connect-password');
          } else {
            setError('The mnemonic provided is not valid. Please check the mnemonic');
          }
        }}
        disabled={localMnemonic.length === 0}
      >
        Next
      </Button>
      <Button size="large" color="inherit" fullWidth onClick={() => history.goBack()}>
        Back
      </Button>
    </Stack>
  );
};
