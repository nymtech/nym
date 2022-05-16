import React, { useContext, useEffect } from 'react';
import { Box } from '@mui/material';
import { BalanceCard } from './balance';
import { VestingCard } from './vesting';
import { AppContext } from '../../context/main';
import { PageLayout } from '../../layouts';

export const Balance = () => {
  const { userBalance } = useContext(AppContext);

  useEffect(() => {
    userBalance.fetchBalance();
  }, []);

  return (
    <PageLayout>
      <Box display="flex" flexDirection="column" gap={2}>
        <BalanceCard />
        {userBalance.originalVesting && <VestingCard />}
      </Box>
    </PageLayout>
  );
};
