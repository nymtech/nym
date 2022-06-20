import React, { useContext, useState } from 'react';
import { Box } from '@mui/material';
import { AppContext } from '../../context/main';

import { BalanceCard } from './balance';
import { VestingCard } from './vesting';
import { PageLayout } from '../../layouts';
import { TransferModal } from './components/TransferModal';

export const Balance = () => {
  const [showTransferModal, setShowTransferModal] = useState(false);

  const { userBalance } = useContext(AppContext);

  const handleShowTransferModal = async () => {
    await userBalance.refreshBalances();
    setShowTransferModal(false);
  };

  return (
    <PageLayout>
      <Box display="flex" flexDirection="column" gap={2}>
        <BalanceCard />
        <VestingCard onTransfer={handleShowTransferModal} />
        {showTransferModal && <TransferModal onClose={() => setShowTransferModal(false)} />}
      </Box>
    </PageLayout>
  );
};
