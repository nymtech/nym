import React, { useEffect } from 'react';
import { Stack, Typography } from '@mui/material';
import { useAppContext } from 'src/context';

export const Balance = () => {
  const { balance, fiatBalance, currency, getBalance } = useAppContext();

  useEffect(() => {
    getBalance();
  }, []);

  const fiat = fiatBalance ? `~ ${Intl.NumberFormat().format(fiatBalance)} ${currency.toUpperCase()}` : '-';

  return (
    <Stack alignItems="center" gap={1}>
      <Typography sx={{ color: 'grey.600' }}>Available</Typography>
      <Typography variant="h4" textAlign="center">
        {balance} NYM
      </Typography>
      <Typography sx={{ color: 'grey.600' }}>{fiat}</Typography>
    </Stack>
  );
};
