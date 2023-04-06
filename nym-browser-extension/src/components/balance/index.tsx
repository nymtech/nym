import React from 'react';
import { Stack, Typography } from '@mui/material';

export const Balance = () => {
  return (
    <Stack alignItems="center" gap={1}>
      <Typography sx={{ color: 'grey.600' }}>Available</Typography>
      <Typography variant="h4">1000.35 NYM</Typography>
      <Typography sx={{ color: 'grey.600' }}>~250.00 USD</Typography>
    </Stack>
  );
};
