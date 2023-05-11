import React, { useEffect } from 'react';
import { Stack, Typography } from '@mui/material';
import { useAppContext } from 'src/context';
import Big from 'big.js';

export const Balance = () => {
  const { balance, getBalance } = useAppContext();

  useEffect(() => {
    getBalance();
  }, []);

  const calculateUSD = () => {
    if (balance) {
      const val = Big(balance).mul(0.15).round(0);
      return Intl.NumberFormat().format(Number(val));
    }

    return '-';
  };

  return (
    <Stack alignItems="center" gap={1}>
      <Typography sx={{ color: 'grey.600' }}>Available</Typography>
      <Typography variant="h4" textAlign="center">
        {balance} NYM
      </Typography>
      <Typography sx={{ color: 'grey.600' }}>~ £{calculateUSD()}</Typography>
    </Stack>
  );
};
