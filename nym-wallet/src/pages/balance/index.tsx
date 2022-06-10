import React from 'react';
import { Box } from '@mui/material';
import { BalanceCard } from './balance';
import { VestingCard } from './vesting';

import { PageLayout } from '../../layouts';

export const Balance = () => (
  <PageLayout>
    <Box display="flex" flexDirection="column" gap={2}>
      <BalanceCard />
      <VestingCard />
    </Box>
  </PageLayout>
);
