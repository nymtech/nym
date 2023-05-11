import React from 'react';
import { Typography } from '@mui/material';
import { Stack } from '@mui/system';
import { ClientAddress } from '@nymproject/react/client-address/ClientAddress';
import { useAppContext } from 'src/context';

export const Address = () => {
  const { client } = useAppContext();

  return (
    <Stack direction="row" justifyContent="space-between" alignItems="center">
      <Typography fontWeight={700}>Address</Typography>
      <ClientAddress withCopy address={client?.address || ''} />
    </Stack>
  );
};
