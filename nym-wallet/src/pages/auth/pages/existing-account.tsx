/* eslint-disable react/no-unused-prop-types */
import React from 'react';
import { useNavigate } from 'react-router-dom';
import { Box, Button, Stack, Typography } from '@mui/material';
import { SubtitleSlick } from '../components';

export const ExistingAccount = () => {
  const navigate = useNavigate();
  return (
    <Stack spacing={1.5} sx={{ width: '100%', alignItems: 'stretch' }}>
      <SubtitleSlick subtitle="Next generation of privacy" />
      <Stack spacing={2} sx={{ width: '100%', pt: 0.5 }}>
        <Button
          variant="contained"
          size="large"
          onClick={() => navigate('/sign-in-mnemonic')}
          fullWidth
          sx={{ py: 1.25 }}
        >
          Sign in with mnemonic
        </Button>
        <Typography sx={{ textAlign: 'center', fontWeight: 600, color: 'text.secondary' }}>or</Typography>
        <Button
          variant="contained"
          size="large"
          fullWidth
          sx={{ py: 1.25 }}
          onClick={() => navigate('/sign-in-password')}
        >
          Sign in with password
        </Button>
        <Box display="flex" justifyContent="center" alignItems="center" flexWrap="wrap" gap={2} sx={{ pt: 1 }}>
          <Button color="inherit" onClick={() => navigate('/')}>
            Back
          </Button>
          <Button color="info" onClick={() => navigate('/forgot-password')}>
            Forgot password?
          </Button>
        </Box>
      </Stack>
    </Stack>
  );
};
