import React, { useContext, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { FeeDetails } from '@nymproject/types';
import { Alert, AlertTitle, Box, Button, Typography } from '@mui/material';
import { Bond } from 'src/components/Bonding/Bond';
import { BondedMixnode } from 'src/components/Bonding/BondedMixnode';
import { TBondedMixnodeActions } from 'src/components/Bonding/BondedMixnodeActions';
import { UpdateBondAmountModal } from 'src/components/Bonding/modals/UpdateBondAmountModal';
import { BondOversaturatedModal } from 'src/components/Bonding/modals/BondOversaturatedModal';
import { ConfirmationDetailProps, ConfirmationDetailsModal } from 'src/components/Bonding/modals/ConfirmationModal';
import { ErrorModal } from 'src/components/Modals/ErrorModal';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { AppContext, urls } from 'src/context/main';
import { isGateway, isMixnode, isNymNode, TBondNymNodeArgs, TUpdateBondArgs } from 'src/types';
import { BondedGateway } from 'src/components/Bonding/BondedGateway';
import { RedeemRewardsModal } from 'src/components/Bonding/modals/RedeemRewardsModal';
import { VestingWarningModal } from 'src/components/VestingWarningModal';
import MigrateLegacyNode from 'src/components/Bonding/modals/MigrateLegacyNode';
import { BondedNymNode } from 'src/components/Bonding/BondedNymNode';
import { UpdateBondAmountNymNode } from 'src/components/Bonding/modals/UpdateBondAmountNymNode';
import { BondNymNode } from 'src/components/Bonding/modals/BondNymNodeModal';
import { BondingContextProvider, useBondingContext } from '../../context';

export const Bonding = () => {
  const [showModal, setShowModal] = useState<
    | 'bond-mixnode'
    | 'bond-nymnode'
    | 'bond-gateway'
    | 'update-bond'
    | 'update-bond-oversaturated'
    | 'unbond'
    | 'redeem'
    | 'update-bond-nymnode'
  >();

  const { network } = useContext(AppContext);

  const navigate = useNavigate();

  const {
    bondedNode,
    isLoading,
    error,
    redeemRewards,
    updateBondAmount,
    refresh,
    bond,
    migrateVestedMixnode,
    migrateLegacyNode,
  } = useBondingContext();

  const shouldShowMigrateLegacyNodeModal = () => {
    if (!bondedNode) {
      return false;
    }
    if (isMixnode(bondedNode) && !bondedNode.isUnbonding) {
      return true;
    }
    if (isGateway(bondedNode)) {
      return true;
    }
    return false;
  };

  const [confirmationDetails, setConfirmationDetails] = useState<ConfirmationDetailProps>();
  const [uncappedSaturation, setUncappedSaturation] = useState<number | undefined>();
  const [showMigrationModal, setShowMigrationModal] = useState(false);
  const [showMigrateLegacyNodeModal, setShowMigrateLegacyNodeModal] = useState(false);

  useEffect(() => {
    if (bondedNode && isMixnode(bondedNode) && bondedNode.uncappedStakeSaturation) {
      setUncappedSaturation(bondedNode.uncappedStakeSaturation);
    }

    setShowMigrateLegacyNodeModal(shouldShowMigrateLegacyNodeModal());
  }, [bondedNode]);

  const handleBondNymNode = async (data: TBondNymNodeArgs) => {
    setShowModal(undefined);
    const tx = await bond(data);
    if (tx) {
      setConfirmationDetails({
        status: 'success',
        title: 'Bonding successful',
        txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
      });
    }
  };

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

  const handleMigrateLegacyNode = async () => {
    setShowMigrateLegacyNodeModal(false);
    const tx = await migrateLegacyNode();
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

  const handleUpdateBond = async (data: TUpdateBondArgs) => {
    setShowModal(undefined);

    const tx = await updateBondAmount(data);
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

  const handleBondedNymNodeAction = async (action: TBondedMixnodeActions) => {
    switch (action) {
      case 'unbond': {
        navigate('/bonding/node-settings', { state: 'unbond' });
        break;
      }
      case 'updateBond': {
        setShowModal('update-bond-nymnode');
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
      {bondedNode && !isNymNode(bondedNode) && bondedNode?.proxy && (
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

      <MigrateLegacyNode
        open={showMigrateLegacyNodeModal}
        onClose={() => setShowMigrateLegacyNodeModal(false)}
        handleMigrate={handleMigrateLegacyNode}
      />

      {!bondedNode && <Bond disabled={isLoading} onBond={() => setShowModal('bond-nymnode')} />}

      {bondedNode && isNymNode(bondedNode) && (
        <BondedNymNode
          nymnode={bondedNode}
          network={network}
          onActionSelect={(action) => handleBondedNymNodeAction(action)}
        />
      )}

      {bondedNode && isMixnode(bondedNode) && (
        <BondedMixnode
          mixnode={bondedNode}
          network={network}
          onShowMigrateToNymNodeModal={() => setShowMigrateLegacyNodeModal(true)}
          onActionSelect={(action) => handleBondedMixnodeAction(action)}
        />
      )}

      {bondedNode && isGateway(bondedNode) && (
        <BondedGateway
          gateway={bondedNode}
          network={network}
          onShowMigrateToNymNodeModal={() => setShowMigrateLegacyNodeModal(true)}
          onActionSelect={handleBondedMixnodeAction}
        />
      )}

      <BondNymNode open={showModal === 'bond-nymnode'} onClose={handleCloseModal} onBond={handleBondNymNode} />

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

      {showModal === 'update-bond-nymnode' && bondedNode && isNymNode(bondedNode) && (
        <UpdateBondAmountNymNode
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
