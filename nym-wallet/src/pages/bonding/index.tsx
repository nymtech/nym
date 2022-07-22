import React, { useContext, useState } from 'react';
import { BondGatewayModal } from 'src/components/Bonding/modals/BondGatewayModal';
import { BondMixnodeModal } from 'src/components/Bonding/modals/BondMixnodeModal';
import { AppContext } from 'src/context/main';
import { BondingContextProvider, useBondingContext } from '../../context';
import { PageLayout } from '../../layouts';
import BondingCard from './bonding';
import GatewayCard from './gateway';
import MixnodeCard from './mixnode';

const Bonding = () => {
  const [showModal, setShowModal] = useState<'bond-mixnode' | 'bond-gateway'>();
  const {
    clientDetails,
    userBalance: { originalVesting },
  } = useContext(AppContext);
  const { bondedMixnode, bondedGateway } = useBondingContext();

  // TODO display a special UI on loading state
  return (
    <PageLayout>
      {showModal === 'bond-mixnode' && (
        <BondMixnodeModal
          denom={clientDetails?.display_mix_denom || 'nym'}
          hasVestingTokens={Boolean(originalVesting)}
          onBondMixnode={() => {}}
          onClose={() => setShowModal(undefined)}
          onError={() => {}}
        />
      )}
      {showModal === 'bond-gateway' && (
        <BondGatewayModal
          onBondGateway={() => {}}
          onClose={() => setShowModal(undefined)}
          denom={clientDetails?.display_mix_denom || 'nym'}
          hasVestingTokens={Boolean(originalVesting)}
        />
      )}
      {!bondedMixnode && !bondedGateway && (
        <BondingCard
          onBondMixnode={() => setShowModal('bond-mixnode')}
          onBondGateway={() => setShowModal('bond-gateway')}
        />
      )}
      {bondedMixnode && <MixnodeCard mixnode={bondedMixnode} />}
      {bondedGateway && <GatewayCard gateway={bondedGateway} />}
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
