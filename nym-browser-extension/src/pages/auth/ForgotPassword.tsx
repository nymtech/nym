import React from 'react';
import { Typography } from '@mui/material';
import { Box } from '@mui/system';
import { TopLogoLayout } from 'src/layouts';

const steps = [
  'Make sure you have your mnemonic saved',
  'Uninstal Nym extension wallet',
  'Reinstal Nym extension wallet',
  'Import your account using seed phrase',
  'Create new password',
];

export const ForgotPassword = () => (
  <TopLogoLayout title="Forgot password" description="Follow instructions below" Actions={<div />}>
    <Box sx={{ px: 2 }}>
      {steps.map((step, index) => (
        <Typography sx={{ color: 'grey.700', my: 3 }} key={step} variant="body2">
          {`${index + 1}. ${step}`}
        </Typography>
      ))}
    </Box>
  </TopLogoLayout>
);
