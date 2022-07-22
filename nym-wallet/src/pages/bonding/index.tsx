import { useContext, useState } from 'react';
import { BondMixnodeModal } from 'src/components/Bonding/modals/BondModal';
import { AppContext } from 'src/context/main';
import { BondingContextProvider, useBondingContext } from '../../context';
import { PageLayout } from '../../layouts';
import BondingCard from './bonding';
import GatewayCard from './gateway';
import MixnodeCard from './mixnode';

const Bonding = () => {
  const [showModal, setShowModal] = useState<'bond-mixnode'>();
  const { clientDetails } = useContext(AppContext);
  const { bondedMixnode, bondedGateway } = useBondingContext();

  // TODO display a special UI on loading state
  return (
    <PageLayout>
      {showModal === 'bond-mixnode' && (
        <BondMixnodeModal onClose={() => setShowModal(undefined)} denom={clientDetails?.display_mix_denom || 'nym'} />
      )}
      {!bondedMixnode && !bondedGateway && <BondingCard onBond={() => setShowModal('bond-mixnode')} />}
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
