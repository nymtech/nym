import React, { useContext, useState } from 'react';
import { useHistory } from 'react-router-dom';
import { Button, LinearProgress, FormControl, Stack, Box } from '@mui/material';
import { PasswordInput, Subtitle } from '../components';
import { ClientContext } from '../../../context/main';

export const SignInPassword = () => {
  const [password, setPassword] = useState('');
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
      <Subtitle subtitle="Enter a password to sign in" />
      <FormControl fullWidth>
        <Stack spacing={2}>
          <PasswordInput
            label="Enter password"
            password={password}
            onUpdatePassword={(pswd) => setPassword(pswd)}
            error={error}
            autoFocus
          />
          <Button
            variant="contained"
            size="large"
            fullWidth
            onClick={() => logIn({ type: 'password', value: password })}
          >
            Sign in with password
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
