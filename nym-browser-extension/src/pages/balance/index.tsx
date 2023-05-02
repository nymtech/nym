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
    {actionsSchema.map(({ title, Icon }, index) => (
      <Stack justifyContent="center" alignItems="center" key={index}>
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
      <Address label="Address" address="n1fhu7p0zx5pvfffjudw5gpce3ncgdde94tan5d6" />
      <Balance />
      <Actions />
    </Stack>
  </PageLayout>
);
