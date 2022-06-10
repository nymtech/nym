import React, { useContext, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Box, Button, FormControl, Stack } from '@mui/material';
import { PasswordInput } from 'src/components';
import { Subtitle } from '../components';
import { AppContext } from '../../../context/main';

export const SignInPassword = () => {
  const [password, setPassword] = useState('');
  const { setError, logIn, error } = useContext(AppContext);
  const navigate = useNavigate();

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
          <Box display="flex" justifyContent="space-between">
            <Button
              color="inherit"
              disableElevation
              onClick={() => {
                setError(undefined);
                navigate('/existing-account');
              }}
            >
              Back
            </Button>

            <Button
              color="info"
              onClick={() => {
                setError(undefined);
                navigate('/sign-in-mnemonic');
              }}
              size="small"
            >
              Forgotten password?
            </Button>
          </Box>
        </Stack>
      </FormControl>
    </Stack>
  );
};
