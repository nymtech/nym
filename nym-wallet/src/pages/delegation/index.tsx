import React, { FC, useContext, useEffect, useState } from 'react';
import { Box, Button, Link, Paper, Stack, Typography } from '@mui/material';
import { DelegationWithEverything, MajorCurrencyAmount } from '@nymproject/types';
import { AppContext, urls } from 'src/context/main';
import { DelegationList } from 'src/components/Delegation/DelegationList';
import { PendingEvents } from 'src/components/Delegation/PendingEvents';
import { TPoolOption } from 'src/components';
import { getSpendableCoins, userBalance } from 'src/requests';
import { RewardsSummary } from '../../components/Rewards/RewardsSummary';
import { useDelegationContext, DelegationContextProvider } from '../../context/delegations';
import { RewardsContextProvider, useRewardsContext } from '../../context/rewards';
import { DelegateModal } from '../../components/Delegation/DelegateModal';
import { UndelegateModal } from '../../components/Delegation/UndelegateModal';
import { DelegationListItemActions } from '../../components/Delegation/DelegationActions';
import { RedeemModal } from '../../components/Rewards/RedeemModal';
import { DelegationModal, DelegationModalProps } from '../../components/Delegation/DelegationModal';

const explorerUrl = 'https://sandbox-explorer.nymtech.net';

export const Delegation: FC = () => {
  const [showNewDelegationModal, setShowNewDelegationModal] = useState<boolean>(false);
  const [showDelegateMoreModal, setShowDelegateMoreModal] = useState<boolean>(false);
  const [showUndelegateModal, setShowUndelegateModal] = useState<boolean>(false);
  const [showRedeemRewardsModal, setShowRedeemRewardsModal] = useState<boolean>(false);
  const [showRedeemAllRewardsModal, setShowRedeemAllRewardsModal] = useState<boolean>(false);
  const [confirmationModalProps, setConfirmationModalProps] = useState<DelegationModalProps | undefined>();
  const [currentDelegationListActionItem, setCurrentDelegationListActionItem] = useState<DelegationWithEverything>();

  const {
    clientDetails,
    network,
    userBalance: { balance, originalVesting },
  } = useContext(AppContext);
  const { redeemAllRewards, redeemRewards, totalRewards, isLoading: isLoadingRewards } = useRewardsContext();
  const {
    delegations,
    pendingDelegations,
    totalDelegations,
    isLoading: isLoadingDelegations,
    addDelegation,
    undelegate,
    refresh,
  } = useDelegationContext();

  // Refresh the rewards and delegations periodically when page is mounted
  useEffect(() => {
    const timer = setInterval(refresh, 1 * 60 * 1000); // every 5 minutes
    return () => clearInterval(timer);
  }, []);

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

      const balance = await userBalance();
      let spendableLocked;

      if (tokenPool === 'locked') spendableLocked = await getSpendableCoins();

      setConfirmationModalProps({
        status: 'success',
        action: 'delegate',
        balance:
          tokenPool === 'locked'
            ? `${spendableLocked?.amount} ${spendableLocked?.denom}`
            : balance?.printable_balance || '-',
        transactionUrl: `${urls(network).blockExplorer}/transaction/${tx.transaction_hash}`,
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
      let balance = await userBalance();
      let spendableLocked;

      if (originalVesting) spendableLocked = await getSpendableCoins();

      setConfirmationModalProps({
        status: 'success',
        action: 'delegate',
        balance:
          tokenPool === 'locked' ? `${spendableLocked} ${clientDetails?.denom}` : balance?.printable_balance || '-',
        transactionUrl: `${urls(network).blockExplorer}/transaction/${tx.transaction_hash}`,
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
      const balance = await userBalance();

      setConfirmationModalProps({
        status: 'success',
        action: 'undelegate',
        balance: balance?.printable_balance || '-',
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

  const handleRedeem = async (identityKey?: string) => {
    if (!identityKey) {
      setConfirmationModalProps({
        status: 'error',
        action: 'redeem',
      });
      return;
    }
    setConfirmationModalProps({
      status: 'loading',
      action: 'redeem',
    });
    setShowRedeemRewardsModal(false);
    setCurrentDelegationListActionItem(undefined);
    if (clientDetails?.client_address) {
      try {
        const tx = await redeemRewards(identityKey);
        const balance = await userBalance();
        setConfirmationModalProps({
          status: 'success',
          action: 'redeem',
          balance: balance?.printable_balance || '-',
          recipient: clientDetails?.client_address,
          transactionUrl: `${urls(network).blockExplorer}/${tx}}`,
        });
      } catch (e) {
        setConfirmationModalProps({
          status: 'error',
          action: 'redeem',
          message: (e as Error).message,
        });
      }
    }
  };

  const handleRedeemAll = async () => {
    setConfirmationModalProps({
      status: 'loading',
      action: 'redeem-all',
    });
    setShowRedeemAllRewardsModal(false);
    setCurrentDelegationListActionItem(undefined);
    try {
      const tx = await redeemAllRewards();
      const balance = await userBalance();

      setConfirmationModalProps({
        status: 'success',
        action: 'redeem-all',
        balance: balance?.printable_balance || '-',
        recipient: clientDetails?.client_address,
        transactionUrl: tx.transactionUrl,
      });
    } catch (e) {
      setConfirmationModalProps({
        status: 'error',
        action: 'redeem-all',
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
            <RewardsSummary
              isLoading={isLoadingDelegations || isLoadingRewards}
              totalDelegation={totalDelegations}
              totalRewards={totalRewards}
              onClickRedeemAll={() => setShowRedeemAllRewardsModal(true)}
            />
            <Button
              variant="contained"
              disableElevation
              onClick={() => setShowNewDelegationModal(true)}
              sx={{ py: 1.5, px: 5 }}
            >
              New Delegation
            </Button>
          </Box>
          <DelegationList
            explorerUrl={explorerUrl}
            isLoading={isLoadingDelegations}
            items={delegations}
            onItemActionClick={handleDelegationItemActionClick}
          />
        </Stack>
      </Paper>

      {pendingDelegations && (
        <Paper elevation={0} sx={{ p: 4, mt: 2 }}>
          <Stack spacing={5}>
            <Typography variant="h6">Pending Delegation Events</Typography>
            <PendingEvents pendingEvents={pendingDelegations} explorerUrl={explorerUrl} />
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
          fee={0.004375}
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
          fee={0.004375}
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

      {currentDelegationListActionItem && showRedeemRewardsModal && (
        <RedeemModal
          open={showRedeemRewardsModal}
          onClose={() => setShowRedeemRewardsModal(false)}
          onOk={handleRedeem}
          message="Redeem rewards"
          currency={clientDetails!.denom}
          identityKey={currentDelegationListActionItem.node_identity}
          fee={0.004375}
          amount={425.65843}
        />
      )}

      {showRedeemAllRewardsModal && (
        <RedeemModal
          open={showRedeemAllRewardsModal}
          onClose={() => setShowRedeemAllRewardsModal(false)}
          onOk={handleRedeemAll}
          message="Redeem all rewards"
          currency="NYM"
          fee={0.004375}
          amount={425.65843}
        />
      )}

      {confirmationModalProps && (
        <DelegationModal
          {...confirmationModalProps}
          open={Boolean(confirmationModalProps)}
          onClose={() => setConfirmationModalProps(undefined)}
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
