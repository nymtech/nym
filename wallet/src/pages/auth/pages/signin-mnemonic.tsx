import React, { useContext, useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { Box, Button, FormControl, Stack } from '@mui/material';
import { AppContext } from '@src/context';
import { isPasswordCreated } from '@src/requests';
import { MnemonicInput } from '@nymproject/react';
import { Subtitle } from '../components';

export const SignInMnemonic = () => {
  const [mnemonic, setMnemonic] = useState('');
  const [passwordExists, setPasswordExists] = useState(true);

  const { setError, logIn, error } = useContext(AppContext);
  const navigate = useNavigate();

  const checkForPassword = async () => {
    const hasPassword = await isPasswordCreated();
    setPasswordExists(hasPassword);
  };

  const handlePageChange = (page: any) => {
    setError(undefined);
    navigate(page);
  };

  useEffect(() => {
    checkForPassword();
  }, []);

  return (
    <Stack spacing={2} alignItems="center" minWidth="50%">
      <Subtitle subtitle="Enter a mnemonic to sign in" />
      <FormControl fullWidth>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            logIn({ type: 'mnemonic', value: mnemonic });
          }}
        >
          <Stack spacing={2}>
            <MnemonicInput mnemonic={mnemonic} onUpdateMnemonic={(mnc) => setMnemonic(mnc)} error={error} />
            <Button variant="contained" size="large" fullWidth type="submit">
              Sign in with mnemonic
            </Button>
            <Box display="flex" justifyContent={passwordExists ? 'center' : 'space-between'}>
              <Button color="inherit" onClick={() => handlePageChange(-1)}>
                Back
              </Button>
              {!passwordExists && (
                <Button color="info" onClick={() => handlePageChange('/confirm-mnemonic')}>
                  Create a password
                </Button>
              )}
            </Box>
          </Stack>
        </form>
      </FormControl>
    </Stack>
  );
};
