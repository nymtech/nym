import React from 'react';
import { Typography } from '@mui/material';
import { Stack } from '@mui/system';
import { ClientAddress } from '@nymproject/react/client-address/ClientAddress';

export const Address = ({ label, address }: { label: string; address: string }) => (
  <Stack direction="row" justifyContent="space-between" alignItems="center">
    <Typography fontWeight={700}>{label}</Typography>
    <ClientAddress withCopy address={address} />
  </Stack>
);
