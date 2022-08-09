import React, { useContext, useEffect, useState } from 'react';
import { Box } from '@mui/material';
import { AppContext } from '../../context/main';

import { BalanceCard } from './balance';
import { VestingCard } from './vesting';
import { PageLayout } from '../../layouts';
import { TransferModal } from './components/TransferModal';

export const Balance = () => {
  const [showTransferModal, setShowTransferModal] = useState(false);
  const [showVestingCard, setShowVestingCard] = useState(false);

  const { userBalance } = useContext(AppContext);

  useEffect(() => {
    const { originalVesting, currentVestingPeriod, tokenAllocation } = userBalance;
    if (
      originalVesting &&
      currentVestingPeriod === 'After' &&
      tokenAllocation?.locked === '0' &&
      tokenAllocation?.vesting === '0' &&
      tokenAllocation?.spendable === '0'
    ) {
      setShowVestingCard(false);
    } else if (originalVesting) {
      setShowVestingCard(true);
    }
  }, [userBalance]);

  const handleShowTransferModal = async () => {
    await userBalance.refreshBalances();
    setShowTransferModal(true);
  };

  return (
    <PageLayout>
      <Box display="flex" flexDirection="column" gap={2}>
        <BalanceCard />
        {showVestingCard && <VestingCard onTransfer={handleShowTransferModal} />}
        {showTransferModal && <TransferModal onClose={() => setShowTransferModal(false)} />}
      </Box>
    </PageLayout>
  );
};
