import React, { useContext, useState } from 'react';
import { Button, FormControl, Stack } from '@mui/material';
import { PasswordInput, Subtitle } from '../components';
import { ClientContext } from '../../../context/main';
import { TPages } from '../types';

export const SignInPassword = ({ onPrev }: { page: TPages; onPrev: () => void }) => {
  const [password, setPassword] = useState('');
  const { setError, logIn, error } = useContext(ClientContext);

  return (
    <>
      <Stack spacing={2} alignItems="center" minWidth="50%">
        <Subtitle subtitle="Enter a password to sign in" />
        <FormControl fullWidth>
          <Stack spacing={2}>
            <PasswordInput
              label="Enter password"
              password={password}
              onUpdatePassword={(pswd) => setPassword(pswd)}
              error={error}
            />
            <Button
              variant="contained"
              size="large"
              fullWidth
              onClick={() => logIn({ type: 'password', value: password })}
            >
              {`Sign in with password`}
            </Button>
            <Button
              variant="outlined"
              disableElevation
              size="large"
              onClick={() => {
                setError(undefined);
                onPrev();
              }}
              fullWidth
              sx={{ color: 'common.white', border: '1px solid white', '&:hover': { border: '1px solid white' } }}
            >
              Back
            </Button>
          </Stack>
        </FormControl>
      </Stack>
    </>
  );
};
