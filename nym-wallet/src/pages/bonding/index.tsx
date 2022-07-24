import { FeeDetails } from '@nymproject/types';
import React, { useContext, useState } from 'react';
import { TPoolOption } from 'src/components';
import { BondedMixnode } from 'src/components/Bonding/BondedMixnode';
import { TBondedMixnodeActions } from 'src/components/Bonding/BondedMixnodeActions';
import { BondGatewayModal } from 'src/components/Bonding/modals/BondGatewayModal';
import { BondMixnodeModal } from 'src/components/Bonding/modals/BondMixnodeModal';
import { ConfirmationDetailProps, ConfirmationDetailsModal } from 'src/components/Bonding/modals/ConfirmationModal';
import { NodeSettings } from 'src/components/Bonding/modals/NodeSettingsModal';
import { UnbondModal } from 'src/components/Bonding/modals/UnbondModal';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { AppContext, urls } from 'src/context/main';
import { isGateway, isMixnode, TBondMixNodeArgs } from 'src/types';
import { BondingContextProvider, useBondingContext } from '../../context';
import { PageLayout } from '../../layouts';
import BondingCard from './bonding';
import GatewayCard from './gateway';

const Bonding = () => {
  const [showModal, setShowModal] = useState<'bond-mixnode' | 'bond-gateway' | 'unbond' | 'node-settings'>();
  const [confirmationDetails, setConfirmationDetails] = useState<ConfirmationDetailProps>();
  const {
    network,
    clientDetails,
    userBalance: { originalVesting },
  } = useContext(AppContext);

  const { bondedNode, bondMixnode, unbond, isLoading } = useBondingContext();

  const handleCloseModal = () => setShowModal(undefined);

  const handleError = (error: string) => {
    setShowModal(undefined);
    setConfirmationDetails({
      status: 'error',
      title: 'An error occurred',
      subtitle: error,
    });
  };

  const handleBondMixnode = async (data: TBondMixNodeArgs, tokenPool: TPoolOption) => {
    setShowModal(undefined);
    const tx = await bondMixnode(data, tokenPool);
    setConfirmationDetails({
      status: 'success',
      title: 'Bond successful',
      txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
    });
  };

  const handleUnbond = async (fee?: FeeDetails) => {
    setShowModal(undefined);
    const tx = await unbond(fee);
    setConfirmationDetails({
      status: 'success',
      title: 'Unbond successful',
      txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
    });
  };

  const handleBondedMixnodeAction = (action: TBondedMixnodeActions) => {
    switch (action) {
      case 'unbond': {
        setShowModal('unbond');
        break;
      }
      case 'nodeSettings': {
        setShowModal('node-settings');
        break;
      }
      default: {
        return undefined;
      }
    }
  };

  // TODO display a special UI on loading state
  return (
    <PageLayout>
      {showModal === 'bond-mixnode' && (
        <BondMixnodeModal
          denom={clientDetails?.display_mix_denom || 'nym'}
          hasVestingTokens={Boolean(originalVesting)}
          onBondMixnode={handleBondMixnode}
          onClose={handleCloseModal}
          onError={handleError}
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

      {showModal === 'unbond' && bondedNode && (
        <UnbondModal
          node={bondedNode}
          onClose={() => setShowModal(undefined)}
          onConfirm={handleUnbond}
          onError={handleError}
        />
      )}

      {showModal === 'node-settings' && bondedNode && isMixnode(bondedNode) && (
        <NodeSettings currentPm={bondedNode.profitMargin} onClose={handleCloseModal} />
      )}

      {!bondedNode && (
        <BondingCard
          disabled={isLoading}
          onBondMixnode={() => setShowModal('bond-mixnode')}
          onBondGateway={() => setShowModal('bond-gateway')}
        />
      )}

      {bondedNode && isMixnode(bondedNode) && (
        <BondedMixnode
          mixnode={bondedNode}
          network={network}
          onActionSelect={(action) => handleBondedMixnodeAction(action)}
        />
      )}

      {bondedNode && isGateway(bondedNode) && <GatewayCard gateway={bondedNode} />}

      {confirmationDetails && (
        <ConfirmationDetailsModal
          title={confirmationDetails.title}
          subtitle={confirmationDetails.subtitle || 'This operation can take up to one hour to process'}
          status={confirmationDetails.status}
          txUrl={confirmationDetails.txUrl}
          onClose={() => setConfirmationDetails(undefined)}
        />
      )}

      {isLoading && <LoadingModal />}
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
