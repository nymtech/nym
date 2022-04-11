/* eslint-disable react/no-unused-prop-types */
import React from 'react';
import { useHistory } from 'react-router-dom';
import { Box, Button, Stack, Typography } from '@mui/material';
import { SubtitleSlick, Title } from '../components';

export const ExistingAccount = () => {
  const history = useHistory();
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
        <Box display="flex" justifyContent="space-between">
          <Button color="inherit" onClick={() => history.push('/welcome')}>
            Back
          </Button>
          <Button color="info" onClick={() => history.push('/sign-in-mnemonic')}>
            Forgot password?
          </Button>
        </Box>
      </Stack>
    </>
  );
};
