/* eslint-disable react/no-unused-prop-types */
import React from 'react';
import { useNavigate } from 'react-router-dom';
import { Box, Button, Stack, Typography } from '@mui/material';
import { SubtitleSlick, Title } from '../components';

export const ExistingAccount = () => {
  const navigate = useNavigate();
  return (
    <>
      <Title title="Welcome to Nym" />
      <SubtitleSlick subtitle="NEXT GENERATION OF PRIVACY" />
      <Stack spacing={2} sx={{ width: 300 }}>
        <Button variant="contained" size="large" onClick={() => navigate('/sign-in-mnemonic')} fullWidth>
          Sign in with mnemonic
        </Button>
        <Typography sx={{ textAlign: 'center', fontWeight: 600 }}>or</Typography>
        <Button variant="contained" size="large" fullWidth onClick={() => navigate('/sign-in-password')}>
          Sign in with password
        </Button>
        <Box display="flex" justifyContent="space-between">
          <Button color="inherit" onClick={() => navigate('/')}>
            Back
          </Button>
          <Button color="info" onClick={() => navigate('/forgot-password')}>
            Forgot password?
          </Button>
        </Box>
      </Stack>
    </>
  );
};
