import React, { useContext, useState, useEffect } from 'react';
import { useHistory } from 'react-router-dom';
import { Box, Button, FormControl, Stack } from '@mui/material';
import { ClientContext } from 'src/context';
import { isPasswordCreated } from 'src/requests';
import { MnemonicInput, Subtitle } from '../components';

export const SignInMnemonic = () => {
  const [mnemonic, setMnemonic] = useState('');
  const [passwordExists, setPasswordExists] = useState(true);

  const { setError, logIn, error } = useContext(ClientContext);
  const history = useHistory();

  const checkForPassword = async () => {
    const hasPassword = await isPasswordCreated();
    setPasswordExists(hasPassword);
  };

  const handlePageChange = (page: string) => {
    setError(undefined);
    history.push(page);
  };

  useEffect(() => {
    checkForPassword();
  }, []);

  return (
    <Stack spacing={2} alignItems="center" minWidth="50%">
      <Subtitle subtitle="Enter a mnemonic to sign in" />
      <FormControl fullWidth>
        <Stack spacing={2}>
          <MnemonicInput mnemonic={mnemonic} onUpdateMnemonic={(mnc) => setMnemonic(mnc)} error={error} />
          <Button
            variant="contained"
            size="large"
            fullWidth
            onClick={() => logIn({ type: 'mnemonic', value: mnemonic })}
          >
            Sign in with mnemonic
          </Button>
          <Box display="flex" justifyContent={passwordExists ? 'center' : 'space-between'}>
            <Button color="inherit" onClick={() => handlePageChange('/existing-account')}>
              Back
            </Button>
            {!passwordExists && (
              <Button color="info" onClick={() => handlePageChange('/confirm-mnemonic')}>
                Create a password
              </Button>
            )}
          </Box>
        </Stack>
      </FormControl>
    </Stack>
  );
};
