import React, { useEffect } from 'react';
import { Stack, Typography } from '@mui/material';
import { useAppContext } from 'src/context';

export const Balance = () => {
  const { balance, getBalance } = useAppContext();

  useEffect(() => {
    getBalance();
  }, []);

  return (
    <Stack alignItems="center" gap={1}>
      <Typography sx={{ color: 'grey.600' }}>Available</Typography>
      <Typography variant="h4" textAlign="center">
        {balance} NYM
      </Typography>
      <Typography sx={{ color: 'grey.600' }}>~250.00 USD</Typography>
    </Stack>
  );
};
