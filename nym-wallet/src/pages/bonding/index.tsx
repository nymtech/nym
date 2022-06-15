import React, { useContext, useEffect, useState } from 'react';
import { AppContext } from 'src/context/main';
import { Box } from '@mui/material';
import { useBondingContext, BondingContextProvider } from '../../context';
import { PageLayout } from '../../layouts';
import { BondingCard } from './BondingCard';
import { BondedMixnodeCard } from './BoundedMixnodeCard';
import { BondedGatewayCard } from './BoundedGatewayCard';
import { EnumRequestStatus } from '../../components';
import { useCheckOwnership } from '../../hooks/useCheckOwnership';

const Bonding = () => {
  const [status] = useState(EnumRequestStatus.initial);
  const { bondedMixnode, bondedGateway } = useBondingContext();
  const { checkOwnership, ownership, isLoading } = useCheckOwnership();

  useEffect(() => {
    if (status === EnumRequestStatus.initial) {
      const initialiseForm = async () => {
        await checkOwnership();
      };
      initialiseForm();
    }
  }, [status, checkOwnership]);

  return (
    <PageLayout>
      <Box display="flex" flexDirection="column" gap={2}>
        {!bondedMixnode &&
          !bondedGateway &&
          status === EnumRequestStatus.initial &&
          !ownership.hasOwnership &&
          !isLoading && <BondingCard />}
        {bondedMixnode && <BondedMixnodeCard />}
        {bondedGateway && <BondedGatewayCard />}
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
