import React from 'react';
import { Box, Stack } from '@mui/material';
import { IconButton, Typography } from '@mui/material';
import { PageLayout } from 'src/layouts/PageLayout';
import { Balance } from 'src/components/balance';
import { ClientAddress } from '@nymproject/react/client-address/ClientAddress';
import { ArrowDownwardRounded, ArrowUpwardRounded, TollRounded } from '@mui/icons-material';

const actionsSchema = [
  {
    title: 'Send',
    Icon: <ArrowDownwardRounded fontSize="large" />,
  },
  {
    title: 'Receive',
    Icon: <ArrowUpwardRounded fontSize="large" />,
  },
  {
    title: 'Buy',
    Icon: <TollRounded fontSize="large" />,
  },
];

const Actions = () => (
  <Box display="flex" justifyContent="space-evenly">
    {actionsSchema.map(({ title, Icon }) => (
      <Stack justifyContent="center" alignItems="center">
        <IconButton color="primary" size="large">
          {Icon}
        </IconButton>
        <Typography>{title}</Typography>
      </Stack>
    ))}
  </Box>
);

const Address = () => (
  <Stack direction="row" justifyContent="space-between" alignItems="center">
    <Typography fontWeight={700}>Address</Typography>
    <ClientAddress withCopy address="n1fhu7p0zx5pvfffjudw5gpce3ncgdde94tan5d6" smallIcons />
  </Stack>
);

export const BalancePage = () => {
  return (
    <PageLayout>
      <Stack gap={6}>
        <Address />
        <Balance />
        <Actions />
      </Stack>
    </PageLayout>
  );
};
