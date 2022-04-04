/* eslint-disable react/no-unused-prop-types */
import React, { useEffect, useState } from 'react';
import { useHistory } from 'react-router-dom';
import { Box, Button, Stack, Typography } from '@mui/material';
import { isPasswordCreated } from 'src/requests';
import { SubtitleSlick, Title } from '../components';

export const ExistingAccount = () => {
  const [passwordExists, setPasswordExists] = useState(true);
  const history = useHistory();

  const checkForPassword = async () => {
    const hasPassword = await isPasswordCreated();
    setPasswordExists(hasPassword);
  };

  useEffect(() => {
    checkForPassword();
  }, []);

  return (
    <>
      <Title title="Welcome to Nym" />
      <SubtitleSlick subtitle="NEXT GENERATION OF PRIVACY" />
      <Stack spacing={2} sx={{ width: 300 }}>
        <Button variant="contained" size="large" onClick={() => history.push('/sign-in-mnemonic')} fullWidth>
          Sign in with mnemonic
        </Button>
        <Typography sx={{ textAlign: 'center', fontWeight: 600 }}>or</Typography>
        <Button variant="contained" size="large" fullWidth onClick={() => history.push('/sign-in-password')}>
          Sign in with password
        </Button>
        <Box display="flex" justifyContent={passwordExists ? 'center' : 'space-between'}>
          <Button color="inherit" onClick={() => history.push('/welcome')}>
            Back
          </Button>
          {!passwordExists && (
            <Button color="info" onClick={() => history.push('/confirm-mnemonic')}>
              Create a password
            </Button>
          )}
        </Box>
      </Stack>
    </>
  );
};
