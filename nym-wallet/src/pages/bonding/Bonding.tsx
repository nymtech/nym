import React, { useContext, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { FeeDetails, decimalToFloatApproximation } from '@nymproject/types';
import { Box } from '@mui/material';
import { TPoolOption } from 'src/components';
import { Bond } from 'src/components/Bonding/Bond';
import { BondedMixnode } from 'src/components/Bonding/BondedMixnode';
import { TBondedMixnodeActions } from 'src/components/Bonding/BondedMixnodeActions';
import { BondGatewayModal } from 'src/components/Bonding/modals/BondGatewayModal';
import { BondMixnodeModal } from 'src/components/Bonding/modals/BondMixnodeModal';
import { BondMoreModal } from 'src/components/Bonding/modals/BondMoreModal';
import { BondOversaturatedModal } from 'src/components/Bonding/modals/BondOversaturatedModal';
import { ConfirmationDetailProps, ConfirmationDetailsModal } from 'src/components/Bonding/modals/ConfirmationModal';
import { ErrorModal } from 'src/components/Modals/ErrorModal';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { AppContext, urls } from 'src/context/main';
import { isGateway, isMixnode, TBondGatewayArgs, TBondMixNodeArgs, TBondMoreArgs } from 'src/types';
import { BondedGateway } from 'src/components/Bonding/BondedGateway';
import { RedeemRewardsModal } from 'src/components/Bonding/modals/RedeemRewardsModal';
import { Console } from 'src/utils/console';
import { BondingContextProvider, useBondingContext } from '../../context';
import { getMixnodeStakeSaturation } from '../../requests';

const Bonding = () => {
  const [showModal, setShowModal] = useState<
    'bond-mixnode' | 'bond-gateway' | 'bond-more' | 'bond-more-oversaturated' | 'unbond' | 'redeem'
  >();
  const [confirmationDetails, setConfirmationDetails] = useState<ConfirmationDetailProps>();
  const [saturationPercentage, setSaturationPercentage] = useState<string | undefined>();

  const {
    network,
    clientDetails,
    userBalance: { originalVesting, balance },
  } = useContext(AppContext);

  const navigate = useNavigate();

  const { bondedNode, bondMixnode, bondGateway, redeemRewards, isLoading, checkOwnership, bondMore } =
    useBondingContext();

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
    return undefined;
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

  const handleBondMore = async (data: TBondMoreArgs, tokenPool: TPoolOption) => {
    setShowModal(undefined);
    const tx = await bondMore(data, tokenPool);
    setConfirmationDetails({
      status: 'success',
      title: 'Bond More successful',
      txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
    });
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

  const handleCheckStakeSaturation = async (newMixId: number) => {
    try {
      const newSaturation = decimalToFloatApproximation(
        (await getMixnodeStakeSaturation(newMixId)).uncapped_saturation,
      );
      if (newSaturation && newSaturation > 1) {
        const newSaturationPercentage = Math.round(newSaturation * 100);
        return { isOverSaturated: true, saturationPercentage: newSaturationPercentage };
      }
      return { isOverSaturated: false, saturationPercentage: undefined };
    } catch (e) {
      Console.error('Error fetching the saturation, error:', e);
      return { isOverSaturated: false, saturationPercentage: undefined };
    }
  };

  const handleBondedMixnodeAction = async (action: TBondedMixnodeActions) => {
    switch (action) {
      case 'bondMore': {
        if (bondedNode && isMixnode(bondedNode)) {
          const { isOverSaturated, saturationPercentage: newSaturationPercentage } = await handleCheckStakeSaturation(
            bondedNode.mixId,
          );
          if (isOverSaturated && newSaturationPercentage) {
            setShowModal('bond-more-oversaturated');
            setSaturationPercentage(newSaturationPercentage.toString());
            break;
          }
        }
        setShowModal('bond-more');
        break;
      }
      case 'unbond': {
        navigate('/bonding/node-settings', { state: 'unbond' });
        break;
      }
      case 'redeem': {
        setShowModal('redeem');
        break;
      }
      default: {
        return undefined;
      }
    }
    return undefined;
  };

  return (
    <Box sx={{ mt: 4 }}>
      {!bondedNode && <Bond disabled={isLoading} onBond={() => setShowModal('bond-mixnode')} />}

      {bondedNode && isMixnode(bondedNode) && (
        <BondedMixnode
          mixnode={bondedNode}
          network={network}
          onActionSelect={(action) => handleBondedMixnodeAction(action)}
        />
      )}

      {bondedNode && isGateway(bondedNode) && (
        <BondedGateway gateway={bondedNode} onActionSelect={handleBondedMixnodeAction} network={network} />
      )}

      {showModal === 'bond-mixnode' && (
        <BondMixnodeModal
          denom={clientDetails?.display_mix_denom || 'nym'}
          hasVestingTokens={Boolean(originalVesting)}
          onBondMixnode={handleBondMixnode}
          onSelectNodeType={() => setShowModal('bond-gateway')}
          onClose={() => setShowModal(undefined)}
          onError={handleError}
        />
      )}

      {showModal === 'bond-gateway' && (
        <BondGatewayModal
          denom={clientDetails?.display_mix_denom || 'nym'}
          hasVestingTokens={Boolean(originalVesting)}
          onBondGateway={handleBondGateway}
          onSelectNodeType={() => setShowModal('bond-mixnode')}
          onClose={() => setShowModal(undefined)}
          onError={handleError}
        />
      )}

      {showModal === 'bond-more-oversaturated' && saturationPercentage && (
        <BondOversaturatedModal
          open
          onClose={() => setShowModal(undefined)}
          onContinue={() => setShowModal('bond-more')}
          saturationPercentage={saturationPercentage}
        />
      )}

      {showModal === 'bond-more' && bondedNode && isMixnode(bondedNode) && (
        <BondMoreModal
          node={bondedNode}
          userBalance={balance?.printable_balance}
          onBondMore={handleBondMore}
          onClose={() => setShowModal(undefined)}
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
    </Box>
  );
};

export const BondingPage = () => (
  <BondingContextProvider>
    <Bonding />
  </BondingContextProvider>
);
