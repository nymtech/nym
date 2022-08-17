import React, { useContext, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Box, Button } from '@mui/material';
import { AppContext } from '../../context/main';

import { PageLayout } from '../../layouts';

export const NodeSettingsPage = () => {
  //   const [showTransferModal, setShowTransferModal] = useState(false);
  //   const [showVestingCard, setShowVestingCard] = useState(false);

  const { userBalance } = useContext(AppContext);

  const navigate = useNavigate();

  useEffect(() => {
    const { originalVesting, currentVestingPeriod, tokenAllocation } = userBalance;
    // if (
    //   originalVesting &&
    //   currentVestingPeriod === 'After' &&
    //   tokenAllocation?.locked === '0' &&
    //   tokenAllocation?.vesting === '0' &&
    //   tokenAllocation?.spendable === '0'
    // ) {
    //   setShowVestingCard(false);
    // } else if (originalVesting) {
    //   setShowVestingCard(true);
    // }
  }, [userBalance]);

  const handleShowTransferModal = async () => {
    await userBalance.refreshBalances();
    // setShowTransferModal(true);
  };

  return (
    <PageLayout>
      <Box display="flex" flexDirection="column" gap={2}>
        <h1>hello</h1>
        <Button onClick={() => navigate('/bonding')}>exit</Button>
      </Box>
    </PageLayout>
  );
};
