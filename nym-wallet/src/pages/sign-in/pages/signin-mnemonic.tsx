import React, { useContext, useState } from 'react';
import { Box, Button, FormControl, LinearProgress, Stack } from '@mui/material';
import { useHistory } from 'react-router-dom';
import { MnemonicInput, Subtitle } from '../components';
import { ClientContext } from '../../../context/main';

export const SignInMnemonic = () => {
  const [mnemonic, setMnemonic] = useState('');
  const { setError, logIn, error, isLoading } = useContext(ClientContext);
  const history = useHistory();

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
          <Button
            variant="outlined"
            disableElevation
            size="large"
            onClick={() => {
              setError(undefined);
              history.push('/existing-account');
            }}
            fullWidth
            sx={{ color: 'common.white', border: '1px solid white', '&:hover': { border: '1px solid white' } }}
          >
            Back
          </Button>
        </Stack>
      </FormControl>
    </Stack>
  );
};
