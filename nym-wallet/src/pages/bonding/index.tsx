import React, { useContext } from 'react';
import { AppContext } from 'src/context/main';
import { Box } from '@mui/material';
import { useBondingContext, BondingContextProvider } from '../../context';
import { PageLayout } from '../../layouts';
import BondingCard from './bonding';
import MixnodeCard from './mixnode';
import GatewayCard from './gateway';

const Bonding = () => {
  const { bondedMixnode, bondedGateway, loading } = useBondingContext();

  // TODO display a special UI on loading state
  return (
    <PageLayout>
      <Box display="flex" flexDirection="column" gap={2}>
        {!bondedMixnode && !bondedGateway && <BondingCard />}
        {bondedMixnode && <MixnodeCard mixnode={bondedMixnode} />}
        {bondedGateway && <GatewayCard gateway={bondedGateway} />}
      </Box>
    </PageLayout>
  );
};

export const BondingPage = () => {
  const { network } = useContext(AppContext);
  return (
    <BondingContextProvider network={network}>
      <Bonding />
    </BondingContextProvider>
  );
};
