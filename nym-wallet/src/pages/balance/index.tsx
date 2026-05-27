import React, { useContext, useEffect, useState } from 'react';
import { Stack } from '@mui/material';
import { AppContext } from '../../context/main';

import { BalanceCard } from './Balance';
import { VestingCard } from './Vesting';
import { PageLayout } from '../../layouts';
import { TransferModal } from '../../components/Balance/modals/TransferModal';
import { OverviewQuickActions } from './OverviewQuickActions';
import { NetworkOverviewSection } from './NetworkOverviewSection';

export const Balance = () => {
  const [showTransferModal, setShowTransferModal] = useState(false);

  const { userBalance, clientDetails, network } = useContext(AppContext);

  useEffect(() => {
    const interval = setInterval(() => {
      userBalance.fetchBalance();
      userBalance.fetchTokenAllocation(true);
    }, 10000);

    return () => clearInterval(interval);
  }, []);

  const handleShowTransferModal = async () => {
    await userBalance.refreshBalances();
    setShowTransferModal(true);
  };

  const appendDenom = (value: string = '') => `${value} ${clientDetails?.display_mix_denom.toUpperCase()}`;

  return (
    <PageLayout>
      <Stack spacing={3.5}>
        <Stack spacing={3} sx={{ width: '100%', minWidth: 0 }}>
          <BalanceCard
            userBalance={userBalance.balance}
            userBalanceError={userBalance.error}
            clientAddress={clientDetails?.client_address}
            network={network}
          />
          <OverviewQuickActions />
          {network === 'MAINNET' ? <NetworkOverviewSection /> : null}
        </Stack>
        <VestingCard
          unlockedTokens={appendDenom(userBalance.tokenAllocation?.spendableVestedCoins)}
          unlockedRewards={appendDenom(userBalance.tokenAllocation?.spendableRewardCoins)}
          unlockedTransferable={appendDenom(userBalance.tokenAllocation?.spendable)}
          originalVesting={userBalance.originalVesting}
          onTransfer={handleShowTransferModal}
          fetchBalance={userBalance.fetchBalance}
          fetchTokenAllocation={userBalance.fetchTokenAllocation}
        />
        {showTransferModal && <TransferModal onClose={() => setShowTransferModal(false)} />}
      </Stack>
    </PageLayout>
  );
};
