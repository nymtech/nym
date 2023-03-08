import React, { useContext, useEffect, useState } from 'react';
import { Box } from '@mui/material';
import { AppContext } from '../../context/main';

import { BalanceCard } from './Balance';
import { VestingCard } from './Vesting';
import { PageLayout } from '../../layouts';
import { TransferModal } from '../../components/Balance/modals/TransferModal';

export const Balance = () => {
  const [showTransferModal, setShowTransferModal] = useState(false);

  const { userBalance, clientDetails, network } = useContext(AppContext);

  useEffect(() => {
    userBalance.fetchBalance();
  }, []);

  const handleShowTransferModal = async () => {
    await userBalance.refreshBalances();
    setShowTransferModal(true);
  };

  const appendDenom = (value: string = '') => `${value} ${clientDetails?.display_mix_denom.toUpperCase()}`;

  return (
    <PageLayout>
      <Box display="flex" flexDirection="column" gap={4}>
        <BalanceCard
          userBalance={userBalance.balance}
          userBalanceError={userBalance.error}
          clientAddress={clientDetails?.client_address}
          network={network}
        />
        <VestingCard
          userBalance={appendDenom(userBalance.balance?.printable_balance)}
          unlockedTokens={appendDenom(userBalance.tokenAllocation?.spendable)}
          unlockedRewards={appendDenom(userBalance.tokenAllocation?.spendable)}
          originalVesting={userBalance.originalVesting}
          onTransfer={handleShowTransferModal}
          fetchBalance={userBalance.fetchBalance}
          fetchTokenAllocation={userBalance.fetchTokenAllocation}
        />
        {showTransferModal && <TransferModal onClose={() => setShowTransferModal(false)} />}
      </Box>
    </PageLayout>
  );
};
