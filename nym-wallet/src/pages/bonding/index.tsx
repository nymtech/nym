import React, { useContext, useState } from 'react';
import { DecCoin, FeeDetails } from '@nymproject/types';
import { TPoolOption } from 'src/components';
import { Bond } from 'src/components/Bonding/Bond';
import { BondedMixnode } from 'src/components/Bonding/BondedMixnode';
import { TBondedMixnodeActions } from 'src/components/Bonding/BondedMixnodeActions';
import { BondGatewayModal } from 'src/components/Bonding/modals/BondGatewayModal';
import { BondMixnodeModal } from 'src/components/Bonding/modals/BondMixnodeModal';
import { BondMoreModal } from 'src/components/Bonding/modals/BondMoreModal';
import { ConfirmationDetailProps, ConfirmationDetailsModal } from 'src/components/Bonding/modals/ConfirmationModal';
import { NodeSettings } from 'src/components/Bonding/modals/NodeSettingsModal';
import { UnbondModal } from 'src/components/Bonding/modals/UnbondModal';
import { ErrorModal } from 'src/components/Modals/ErrorModal';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { AppContext, urls } from 'src/context/main';
import { isGateway, isMixnode, TBondGatewayArgs, TBondMixNodeArgs } from 'src/types';
import { BondingContextProvider, useBondingContext } from '../../context';
import { PageLayout } from '../../layouts';
import { BondedGateway } from 'src/components/Bonding/BondedGateway';
import { RedeemRewardsModal } from 'src/components/Bonding/modals/RedeemRewardsModal';
import { CompoundRewardsModal } from 'src/components/Bonding/modals/CompoundRewardsModal';

const Bonding = () => {
  const [showModal, setShowModal] = useState<
    'bond-mixnode' | 'bond-gateway' | 'bond-more' | 'unbond' | 'redeem' | 'compound' | 'node-settings'
  >();
  const [confirmationDetails, setConfirmationDetails] = useState<ConfirmationDetailProps>();

  const {
    network,
    clientDetails,
    userBalance: { originalVesting, balance },
  } = useContext(AppContext);

  const {
    bondedNode,
    bondMixnode,
    bondGateway,
    unbond,
    updateMixnode,
    redeemRewards,
    compoundRewards,
    isLoading,
    checkOwnership,
  } = useBondingContext();

  const handleCloseModal = async () => {
    setShowModal(undefined);
    await checkOwnership();
  };

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

  const handleBondGateway = async (data: TBondGatewayArgs, tokenPool: TPoolOption) => {
    setShowModal(undefined);
    const tx = await bondGateway(data, tokenPool);
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

  const handleUpdateProfitMargin = async (profitMargin: number, fee?: FeeDetails) => {
    setShowModal(undefined);
    const tx = await updateMixnode(profitMargin, fee);
    setConfirmationDetails({
      status: 'success',
      title: 'Profit margin update successful',
      txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
    });
  };

  const handleBondMore = async (args: { additionalBond: DecCoin; signature: string; tokenPool: TPoolOption }) => {
    setShowModal(undefined);
    // const tx = await bondMixnode(data, tokenPool);
    // setConfirmationDetails({
    //   status: 'success',
    //   title: 'Bond successful',
    //   txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
    // });
  };

  const handleRedeemReward = async (fee?: FeeDetails) => {
    setShowModal(undefined);
    const tx = await redeemRewards(fee);
    setConfirmationDetails({
      status: 'success',
      title: 'Rewards redeemed successfully',
      txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
    });
  };

  const handleCompoundReward = async (fee?: FeeDetails) => {
    setShowModal(undefined);
    const tx = await compoundRewards(fee);
    setConfirmationDetails({
      status: 'success',
      title: 'Rewards compounded successfully',
      txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
    });
  };

  const handleBondedMixnodeAction = (action: TBondedMixnodeActions) => {
    switch (action) {
      case 'bondMore': {
        setShowModal('bond-more');
        break;
      }
      case 'unbond': {
        setShowModal('unbond');
        break;
      }
      case 'redeem': {
        setShowModal('redeem');
        break;
      }
      case 'compound': {
        setShowModal('compound');
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

  return (
    <PageLayout>
      {!bondedNode && (
        <Bond
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

      {bondedNode && isGateway(bondedNode) && (
        <BondedGateway gateway={bondedNode} onActionSelect={handleBondedMixnodeAction} />
      )}
      {showModal === 'bond-mixnode' && (
        <BondMixnodeModal
          denom={clientDetails?.display_mix_denom || 'nym'}
          hasVestingTokens={Boolean(originalVesting)}
          onBondMixnode={handleBondMixnode}
          onClose={() => setShowModal(undefined)}
          onError={handleError}
        />
      )}

      {showModal === 'bond-gateway' && (
        <BondGatewayModal
          onBondGateway={handleBondGateway}
          onClose={() => setShowModal(undefined)}
          denom={clientDetails?.display_mix_denom || 'nym'}
          hasVestingTokens={Boolean(originalVesting)}
          onError={handleError}
        />
      )}

      {bondedNode && showModal === 'bond-more' && (
        <BondMoreModal
          currentBond={bondedNode?.bond}
          userBalance={balance?.printable_balance}
          hasVestingTokens={Boolean(originalVesting)}
          onClose={() => setShowModal(undefined)}
          onConfirm={handleBondMore}
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

      {showModal === 'redeem' && bondedNode && isMixnode(bondedNode) && (
        <RedeemRewardsModal
          node={bondedNode}
          onClose={() => setShowModal(undefined)}
          onConfirm={handleRedeemReward}
          onError={handleError}
        />
      )}

      {showModal === 'compound' && bondedNode && isMixnode(bondedNode) && (
        <CompoundRewardsModal
          node={bondedNode}
          onClose={() => setShowModal(undefined)}
          onConfirm={handleCompoundReward}
          onError={handleError}
        />
      )}

      {showModal === 'node-settings' && bondedNode && isMixnode(bondedNode) && (
        <NodeSettings
          currentPm={bondedNode.profitMargin}
          isVesting={Boolean(bondedNode.proxy)}
          onConfirm={handleUpdateProfitMargin}
          onClose={handleCloseModal}
          onError={handleError}
        />
      )}

      {confirmationDetails && confirmationDetails.status === 'success' && (
        <ConfirmationDetailsModal
          title={confirmationDetails.title}
          subtitle={confirmationDetails.subtitle || 'This operation can take up to one hour to process'}
          status={confirmationDetails.status}
          txUrl={confirmationDetails.txUrl}
          onClose={() => {
            setConfirmationDetails(undefined);
            handleCloseModal();
          }}
        />
      )}

      {confirmationDetails && confirmationDetails.status === 'error' && (
        <ErrorModal open message={confirmationDetails.subtitle} onClose={() => setConfirmationDetails(undefined)} />
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
