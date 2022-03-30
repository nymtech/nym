import React, { useContext, useEffect } from 'react';
import { Box } from '@mui/material';
import { BalanceCard } from './balance';
import { VestingCard } from './vesting';
import { ClientContext } from '../../context/main';
import { PageLayout } from '../../layouts';
import { ValidatorSelector } from '../../components/ValidatorSelector';

export const Balance = () => {
  const { userBalance } = useContext(ClientContext);

  useEffect(() => {
    userBalance.fetchBalance();
  }, []);

  return (
    <PageLayout>
      <Box display="flex" flexDirection="column" gap={2}>
        <BalanceCard />
        {userBalance.originalVesting && <VestingCard />}
      </Box>
      <ValidatorSelector
        onChangeValidatorSelection={(selectedValidator) => console.log('selectedValidator:', selectedValidator)}
      />
    </PageLayout>
  );
};
