/* eslint-disable react/no-unused-prop-types */
import React from 'react';
import { useNavigate } from 'react-router-dom';
import { Box, Button, Stack, Typography } from '@mui/material';
import { Subtitle, Title } from '../components';

export const ForgotPassword = () => {
  const navigate = useNavigate();
  return (
    <Stack spacing={1.5} sx={{ width: '100%', alignItems: 'stretch' }}>
      <Title title="Forgot password" align="center" />
      <Subtitle subtitle="Create a new password or sign in with mnemonic" align="center" />
      <Stack spacing={2} sx={{ width: '100%', pt: 0.5 }}>
        <Button
          variant="contained"
          size="large"
          onClick={() => navigate('/confirm-mnemonic')}
          fullWidth
          sx={{ py: 1.25 }}
        >
          Create a new password
        </Button>
        <Typography sx={{ textAlign: 'center', fontWeight: 600, color: 'text.secondary' }}>or</Typography>
        <Button
          size="large"
          variant="contained"
          fullWidth
          sx={{ py: 1.25 }}
          onClick={() => navigate('/sign-in-mnemonic')}
        >
          Sign in with mnemonic
        </Button>
        <Box display="flex" justifyContent="center" alignItems="center" sx={{ pt: 1 }}>
          <Button color="inherit" onClick={() => navigate(-1)}>
            Back
          </Button>
        </Box>
      </Stack>
    </Stack>
  );
};
