import React from 'react';
import { Box, Stack, IconButton, Typography } from '@mui/material';
import { PageLayout } from 'src/layouts/PageLayout';
import { Address, Balance } from 'src/components';
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
      <Stack justifyContent="center" alignItems="center" key={title}>
        <IconButton color="primary" size="large">
          {Icon}
        </IconButton>
        <Typography>{title}</Typography>
      </Stack>
    ))}
  </Box>
);

export const BalancePage = () => (
  <PageLayout>
    <Stack gap={6}>
      <Address />
      <Balance />
      <Actions />
    </Stack>
  </PageLayout>
);
