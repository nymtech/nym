import React, { FC, useContext, useEffect, useState } from 'react';
import { Box, Button, Link, Paper, Stack, Typography } from '@mui/material';
import { DelegationWithEverything, MajorCurrencyAmount } from '@nymproject/types';
import { AppContext, urls } from 'src/context/main';
import { DelegationList } from 'src/components/Delegation/DelegationList';
import { PendingEvents } from 'src/components/Delegation/PendingEvents';
import { TPoolOption } from 'src/components';
import { CompoundModal } from 'src/components/Rewards/CompoundModal';
import { getSpendableCoins, userBalance } from 'src/requests';
import { RewardsSummary } from '../../components/Rewards/RewardsSummary';
import { useDelegationContext, DelegationContextProvider } from '../../context/delegations';
import { RewardsContextProvider } from '../../context/rewards';
import { DelegateModal } from '../../components/Delegation/DelegateModal';
import { UndelegateModal } from '../../components/Delegation/UndelegateModal';
import { DelegationListItemActions } from '../../components/Delegation/DelegationActions';
import { RedeemModal } from '../../components/Rewards/RedeemModal';
import { DelegationModal, DelegationModalProps } from '../../components/Delegation/DelegationModal';

export const Delegation: FC = () => {
  const [showNewDelegationModal, setShowNewDelegationModal] = useState<boolean>(false);
  const [showDelegateMoreModal, setShowDelegateMoreModal] = useState<boolean>(false);
  const [showUndelegateModal, setShowUndelegateModal] = useState<boolean>(false);
  const [showRedeemRewardsModal, setShowRedeemRewardsModal] = useState<boolean>(false);
  const [showCompoundRewardsModal, setShowCompoundRewardsModal] = useState<boolean>(false);
  const [confirmationModalProps, setConfirmationModalProps] = useState<DelegationModalProps | undefined>();
  const [currentDelegationListActionItem, setCurrentDelegationListActionItem] = useState<DelegationWithEverything>();

  const {
    clientDetails,
    network,
    userBalance: { balance, originalVesting, fetchBalance },
  } = useContext(AppContext);

  const {
    delegations,
    pendingDelegations,
    totalDelegations,
    totalRewards,
    isLoading,
    addDelegation,
    undelegate,
    redeemRewards,
    compoundRewards,
    refresh,
  } = useDelegationContext();

  // Refresh the rewards and delegations periodically when page is mounted
  useEffect(() => {
    const timer = setInterval(refresh, 1 * 60 * 1000); // every 1 minute
    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    refresh();
  }, [network, clientDetails, confirmationModalProps]);

  const handleDelegationItemActionClick = (item: DelegationWithEverything, action: DelegationListItemActions) => {
    setCurrentDelegationListActionItem(item);
    // eslint-disable-next-line default-case
    switch (action) {
      case 'delegate':
        setShowDelegateMoreModal(true);
        break;
      case 'undelegate':
        setShowUndelegateModal(true);
        break;
      case 'redeem':
        setShowRedeemRewardsModal(true);
        break;
      case 'compound':
        setShowCompoundRewardsModal(true);
        break;
    }
  };

  const handleNewDelegation = async (identityKey: string, amount: MajorCurrencyAmount, tokenPool: TPoolOption) => {
    setConfirmationModalProps({
      status: 'loading',
      action: 'delegate',
    });
    setShowNewDelegationModal(false);
    setCurrentDelegationListActionItem(undefined);
    try {
      const tx = await addDelegation(
        {
          identity: identityKey,
          amount,
        },
        tokenPool,
      );

      const bal = await userBalance();
      let spendableLocked;

      if (tokenPool === 'locked') spendableLocked = await getSpendableCoins();

      setConfirmationModalProps({
        status: 'success',
        action: 'delegate',
        message: 'Delegations can take up to one hour to process',
        balance:
          tokenPool === 'locked'
            ? `${spendableLocked?.amount} ${spendableLocked?.denom}`
            : bal?.printable_balance || '-',
        transactionUrl: `${urls(network).blockExplorer}/transaction/${tx.transaction_hash}`,
        tokenPool,
      });
    } catch (e) {
      setConfirmationModalProps({
        status: 'error',
        action: 'delegate',
        message: (e as Error).message,
      });
    }
  };

  const handleDelegateMore = async (identityKey: string, amount: MajorCurrencyAmount, tokenPool: TPoolOption) => {
    if (currentDelegationListActionItem?.node_identity !== identityKey || !clientDetails) {
      setConfirmationModalProps({
        status: 'error',
        action: 'delegate',
      });
      return;
    }

    setConfirmationModalProps({
      status: 'loading',
      action: 'delegate',
    });
    setShowDelegateMoreModal(false);
    setCurrentDelegationListActionItem(undefined);

    try {
      const tx = await addDelegation(
        {
          identity: identityKey,
          amount,
        },
        tokenPool,
      );
      const bal = await userBalance();
      let spendableLocked;

      if (originalVesting) spendableLocked = await getSpendableCoins();

      setConfirmationModalProps({
        status: 'success',
        action: 'delegate',
        balance:
          tokenPool === 'locked' ? `${spendableLocked?.amount} ${clientDetails?.denom}` : bal?.printable_balance || '-',
        transactionUrl: `${urls(network).blockExplorer}/transaction/${tx.transaction_hash}`,
        tokenPool,
      });
    } catch (e) {
      setConfirmationModalProps({
        status: 'error',
        action: 'delegate',
        message: (e as Error).message,
      });
    }
  };

  const handleUndelegate = async (identityKey: string, proxy: string | null) => {
    setConfirmationModalProps({
      status: 'loading',
      action: 'undelegate',
    });
    setShowUndelegateModal(false);
    setCurrentDelegationListActionItem(undefined);

    try {
      const tx = await undelegate(identityKey, proxy);
      const bal = await userBalance();

      setConfirmationModalProps({
        status: 'success',
        action: 'undelegate',
        balance: bal?.printable_balance || '-',
        transactionUrl: `${urls(network).blockExplorer}/transaction/${tx.transaction_hash}`,
      });
    } catch (e) {
      setConfirmationModalProps({
        status: 'error',
        action: 'undelegate',
        message: (e as Error).message,
      });
    }
  };

  const handleRedeem = async (identityKey: string, proxy: string | null) => {
    setConfirmationModalProps({
      status: 'loading',
      action: 'redeem',
    });
    setShowRedeemRewardsModal(false);
    setCurrentDelegationListActionItem(undefined);

    try {
      const tx = await redeemRewards(identityKey, proxy);
      const bal = await userBalance();
      setConfirmationModalProps({
        status: 'success',
        action: 'redeem',
        balance: bal?.printable_balance || '-',
        transactionUrl: `${urls(network).blockExplorer}/transaction/${tx.transaction_hash}`,
      });
    } catch (e) {
      setConfirmationModalProps({
        status: 'error',
        action: 'redeem',
        message: (e as Error).message,
      });
    }
  };

  const handleCompound = async (identityKey: string, proxy: string | null) => {
    setConfirmationModalProps({
      status: 'loading',
      action: 'compound',
    });
    setShowCompoundRewardsModal(false);
    setCurrentDelegationListActionItem(undefined);

    try {
      const tx = await compoundRewards(identityKey, proxy);
      const bal = await userBalance();
      setConfirmationModalProps({
        status: 'success',
        action: 'compound',
        balance: bal?.printable_balance || '-',
        transactionUrl: `${urls(network).blockExplorer}/transaction/${tx.transaction_hash}`,
      });
    } catch (e) {
      setConfirmationModalProps({
        status: 'error',
        action: 'redeem',
        message: (e as Error).message,
      });
    }
  };

  return (
    <>
      <Paper elevation={0} sx={{ p: 4, mt: 4 }}>
        <Stack spacing={5}>
          <Box display="flex" justifyContent="space-between" alignItems="center">
            <Typography variant="h6">Delegations</Typography>
            <Link
              href={`${urls(network).networkExplorer}/network-components/mixnodes/`}
              target="_blank"
              rel="noreferrer"
              underline="hover"
              sx={{ color: 'primary.main', textDecorationColor: 'primary.main' }}
            >
              <Typography color="primary.main" variant="body2">
                Network Explorer
              </Typography>
            </Link>
          </Box>
          <Box display="flex" justifyContent="space-between" alignItems="center">
            <RewardsSummary isLoading={isLoading} totalDelegation={totalDelegations} totalRewards={totalRewards} />
            <Button
              variant="contained"
              disableElevation
              onClick={() => setShowNewDelegationModal(true)}
              sx={{ py: 1.5, px: 5 }}
            >
              Delegate
            </Button>
          </Box>
          <DelegationList
            explorerUrl={urls(network).networkExplorer}
            isLoading={isLoading}
            items={delegations}
            onItemActionClick={handleDelegationItemActionClick}
          />
        </Stack>
      </Paper>

      {pendingDelegations && (
        <Paper elevation={0} sx={{ p: 4, mt: 2 }}>
          <Stack spacing={5}>
            <Typography variant="h6">Pending Delegation Events</Typography>
            <PendingEvents pendingEvents={pendingDelegations} explorerUrl={urls(network).networkExplorer} />
          </Stack>
        </Paper>
      )}

      {showNewDelegationModal && (
        <DelegateModal
          open={showNewDelegationModal}
          onClose={() => setShowNewDelegationModal(false)}
          onOk={handleNewDelegation}
          header="Delegate"
          buttonText="Delegate stake"
          currency={clientDetails!.denom}
          accountBalance={balance?.printable_balance}
          rewardInterval="weekly"
          hasVestingContract={Boolean(originalVesting)}
        />
      )}

      {currentDelegationListActionItem && showDelegateMoreModal && (
        <DelegateModal
          open={showDelegateMoreModal}
          onClose={() => setShowDelegateMoreModal(false)}
          onOk={handleDelegateMore}
          header="Delegate more"
          buttonText="Delegate more"
          identityKey={currentDelegationListActionItem.node_identity}
          currency={clientDetails!.denom}
          estimatedReward={0}
          accountBalance={balance?.printable_balance}
          nodeUptimePercentage={currentDelegationListActionItem.avg_uptime_percent}
          profitMarginPercentage={currentDelegationListActionItem.profit_margin_percent}
          rewardInterval="weekly"
          hasVestingContract={Boolean(originalVesting)}
        />
      )}

      {currentDelegationListActionItem && showUndelegateModal && (
        <UndelegateModal
          open={showUndelegateModal}
          onClose={() => setShowUndelegateModal(false)}
          onOk={handleUndelegate}
          proxy={currentDelegationListActionItem.proxy}
          currency={currentDelegationListActionItem.amount.denom}
          fee={0.1}
          amount={+currentDelegationListActionItem.amount.amount}
          identityKey={currentDelegationListActionItem.node_identity}
        />
      )}

      {currentDelegationListActionItem?.accumulated_rewards && showRedeemRewardsModal && (
        <RedeemModal
          open={showRedeemRewardsModal}
          onClose={() => setShowRedeemRewardsModal(false)}
          onOk={(identity) => handleRedeem(identity, currentDelegationListActionItem.proxy)}
          message="Redeem rewards"
          currency={clientDetails!.denom}
          identityKey={currentDelegationListActionItem?.node_identity}
          fee={0.004375}
          amount={+currentDelegationListActionItem.accumulated_rewards.amount}
        />
      )}

      {currentDelegationListActionItem?.accumulated_rewards && showCompoundRewardsModal && (
        <CompoundModal
          open={showCompoundRewardsModal}
          onClose={() => setShowCompoundRewardsModal(false)}
          onOk={(identity) => handleCompound(identity, currentDelegationListActionItem.proxy)}
          message="Compound rewards"
          currency={clientDetails!.denom}
          identityKey={currentDelegationListActionItem?.node_identity}
          fee={0.004375}
          amount={+currentDelegationListActionItem.accumulated_rewards.amount}
        />
      )}

      {confirmationModalProps && (
        <DelegationModal
          {...confirmationModalProps}
          open={Boolean(confirmationModalProps)}
          onClose={async () => {
            setConfirmationModalProps(undefined);
            await fetchBalance();
          }}
        />
      )}
    </>
  );
};

export const DelegationPage = () => {
  const { network } = useContext(AppContext);
  return (
    <DelegationContextProvider network={network}>
      <RewardsContextProvider network={network}>
        <Delegation />
      </RewardsContextProvider>
    </DelegationContextProvider>
  );
};