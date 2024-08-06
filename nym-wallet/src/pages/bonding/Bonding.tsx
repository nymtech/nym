import React, { useContext, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { FeeDetails } from '@nymproject/types';
import { Alert, AlertTitle, Box, Button, Typography } from '@mui/material';
import { TPoolOption } from 'src/components';
import { Bond } from 'src/components/Bonding/Bond';
import { BondedMixnode } from 'src/components/Bonding/BondedMixnode';
import { TBondedMixnodeActions } from 'src/components/Bonding/BondedMixnodeActions';
import { BondGatewayModal } from 'src/components/Bonding/modals/BondGatewayModal';
import { BondMixnodeModal } from 'src/components/Bonding/modals/BondMixnodeModal';
import { UpdateBondAmountModal } from 'src/components/Bonding/modals/UpdateBondAmountModal';
import { BondOversaturatedModal } from 'src/components/Bonding/modals/BondOversaturatedModal';
import { ConfirmationDetailProps, ConfirmationDetailsModal } from 'src/components/Bonding/modals/ConfirmationModal';
import { ErrorModal } from 'src/components/Modals/ErrorModal';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { AppContext, urls } from 'src/context/main';
import { isGateway, isMixnode, TBondGatewayArgs, TBondMixNodeArgs, TUpdateBondArgs } from 'src/types';
import { BondedGateway } from 'src/components/Bonding/BondedGateway';
import { RedeemRewardsModal } from 'src/components/Bonding/modals/RedeemRewardsModal';
import { VestingWarningModal } from 'src/components/VestingWarningModal';
import { BondingContextProvider, useBondingContext } from '../../context';

export const Bonding = () => {
  const [showModal, setShowModal] = useState<
    'bond-mixnode' | 'bond-gateway' | 'update-bond' | 'update-bond-oversaturated' | 'unbond' | 'redeem'
  >();
  const [confirmationDetails, setConfirmationDetails] = useState<ConfirmationDetailProps>();
  const [uncappedSaturation, setUncappedSaturation] = useState<number | undefined>();
  const [showMigrationModal, setShowMigrationModal] = useState(false);
  const {
    network,
    clientDetails,
    userBalance: { originalVesting },
  } = useContext(AppContext);

  const navigate = useNavigate();

  const {
    bondedNode,
    bondMixnode,
    bondGateway,
    redeemRewards,
    isLoading,
    updateBondAmount,
    error,
    refresh,
    migrateVestedMixnode,
  } = useBondingContext();

  useEffect(() => {
    if (bondedNode && isMixnode(bondedNode) && bondedNode.uncappedStakeSaturation) {
      setUncappedSaturation(bondedNode.uncappedStakeSaturation);
    }
  }, [bondedNode]);

  const handleMigrateVestedMixnode = async () => {
    setShowMigrationModal(false);
    const tx = await migrateVestedMixnode();
    if (tx) {
      setConfirmationDetails({
        status: 'success',
        title: 'Migration successful',
        txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
      });
    }
  };

  const handleCloseModal = async () => {
    setShowModal(undefined);
    refresh();
  };

  const handleError = (err: string) => {
    setShowModal(undefined);
    setConfirmationDetails({
      status: 'error',
      title: 'An error occurred',
      subtitle: err,
    });
  };

  const handleBondMixnode = async (data: TBondMixNodeArgs, tokenPool: TPoolOption) => {
    setShowModal(undefined);
    const tx = await bondMixnode(data, tokenPool);
    if (tx) {
      setConfirmationDetails({
        status: 'success',
        title: 'Bond successful',
        txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
      });
    }
    return undefined;
  };

  const handleBondGateway = async (data: TBondGatewayArgs, tokenPool: TPoolOption) => {
    setShowModal(undefined);
    const tx = await bondGateway(data, tokenPool);
    if (tx) {
      setConfirmationDetails({
        status: 'success',
        title: 'Bond successful',
        txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
      });
    }
  };

  const handleUpdateBond = async (data: TUpdateBondArgs, tokenPool: TPoolOption) => {
    setShowModal(undefined);

    const tx = await updateBondAmount(data, tokenPool);
    if (tx) {
      setConfirmationDetails({
        status: 'success',
        title: 'Bond amount changed successfully',
        txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
      });
    }
  };

  const handleRedeemReward = async (fee?: FeeDetails) => {
    setShowModal(undefined);
    const tx = await redeemRewards(fee);
    if (tx) {
      setConfirmationDetails({
        status: 'success',
        title: 'Rewards redeemed successfully',
        txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
      });
    }
  };

  const handleBondedMixnodeAction = async (action: TBondedMixnodeActions) => {
    switch (action) {
      case 'updateBond': {
        if (uncappedSaturation) {
          setShowModal('update-bond-oversaturated');
        } else {
          setShowModal('update-bond');
        }
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

  if (error) {
    return <ErrorModal open message="An error occured, please check logs for details" onClose={() => refresh()} />;
  }

  return (
    <Box sx={{ mt: 4 }}>
      {bondedNode?.proxy && (
        <Alert severity="warning" sx={{ mb: 3 }}>
          <AlertTitle sx={{ fontWeight: 600 }}>Your bonded node is using tokens from the vesting contract!</AlertTitle>
          <Typography>
            In order to claim your rewards, you will need to migrate it out of the vesting contract.{' '}
          </Typography>
          <Typography mt={1}>
            <strong>Never fear</strong>, if you do not migrate them, <strong>you will continue to get rewards</strong>.
            However, please migrate your bonded node as soon as possible.
          </Typography>
          <Button variant="contained" size="small" sx={{ mt: 1 }} onClick={() => setShowMigrationModal(true)}>
            Migrate now
          </Button>
        </Alert>
      )}

      <VestingWarningModal
        kind="bond"
        isVisible={showMigrationModal}
        handleClose={() => {
          setShowMigrationModal(false);
        }}
        handleMigrate={async () => {
          await handleMigrateVestedMixnode();
        }}
      />

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

      {showModal === 'update-bond-oversaturated' && uncappedSaturation && (
        <BondOversaturatedModal
          open
          onClose={() => setShowModal(undefined)}
          onContinue={() => setShowModal('update-bond')}
          saturationPercentage={uncappedSaturation.toString()}
        />
      )}

      {showModal === 'update-bond' && bondedNode && isMixnode(bondedNode) && (
        <UpdateBondAmountModal
          node={bondedNode}
          onUpdateBond={handleUpdateBond}
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
          subtitle={confirmationDetails.subtitle}
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
