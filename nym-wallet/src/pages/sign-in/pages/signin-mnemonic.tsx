import React, { useContext, useState, useEffect } from 'react';
import { Box, Button, FormControl, LinearProgress, Stack } from '@mui/material';
import { useHistory } from 'react-router-dom';
import { isPasswordCreated } from 'src/requests';
import { MnemonicInput, Subtitle } from '../components';
import { ClientContext } from '../../../context/main';

export const SignInMnemonic = () => {
  const [mnemonic, setMnemonic] = useState('');
  const { setError, logIn, error, isLoading } = useContext(ClientContext);
  const [passwordExists, setPasswordExists] = useState(true);
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

  if (isLoading)
    return (
      <Box width="25%">
        <LinearProgress variant="indeterminate" />
      </Box>
    );

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
