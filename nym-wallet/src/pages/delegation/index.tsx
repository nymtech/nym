import React, { FC, useContext, useEffect, useState } from 'react';
import { Box, Button, Link, Paper, Stack, Typography } from '@mui/material';
import { DelegationWithEverything, MajorCurrencyAmount } from '@nymproject/types';
import { AppContext } from 'src/context/main';
import { DelegationList } from 'src/components/Delegation/DelegationList';
import { RewardsSummary } from '../../components/Rewards/RewardsSummary';
import { useDelegationContext, DelegationContextProvider } from '../../context/delegations';
import { RewardsContextProvider, useRewardsContext } from '../../context/rewards';
import { DelegateModal } from '../../components/Delegation/DelegateModal';
import { UndelegateModal } from '../../components/Delegation/UndelegateModal';
import { DelegationListItemActions } from '../../components/Delegation/DelegationActions';
import { RedeemModal } from '../../components/Rewards/RedeemModal';
import { DelegationModal, DelegationModalProps } from '../../components/Delegation/DelegationModal';
import { PendingEvents } from 'src/components/Delegation/PendingEvents';

const explorerUrl = 'https://sandbox-explorer.nymtech.net';

export const Delegation: FC = () => {
  const [showNewDelegationModal, setShowNewDelegationModal] = useState<boolean>(false);
  const [showDelegateMoreModal, setShowDelegateMoreModal] = useState<boolean>(false);
  const [showUndelegateModal, setShowUndelegateModal] = useState<boolean>(false);
  const [showRedeemRewardsModal, setShowRedeemRewardsModal] = useState<boolean>(false);
  const [showRedeemAllRewardsModal, setShowRedeemAllRewardsModal] = useState<boolean>(false);
  const [confirmationModalProps, setConfirmationModalProps] = useState<DelegationModalProps | undefined>();
  const [currentDelegationListActionItem, setCurrentDelegationListActionItem] = useState<DelegationWithEverything>();

  const { clientDetails, userBalance } = useContext(AppContext);
  const { redeemAllRewards, redeemRewards, totalRewards, isLoading: isLoadingRewards } = useRewardsContext();
  const {
    delegations,
    pendingDelegations,
    totalDelegations,
    isLoading: isLoadingDelegations,
    addDelegation,
    updateDelegation,
    undelegate,
    refresh,
  } = useDelegationContext();

  // Refresh the rewards and delegations periodically when page is mounted
  useEffect(() => {
    const timer = setInterval(refresh, 5 * 60 * 1000); // every 5 minutes
    return () => clearInterval(timer);
  }, []);

  // TODO: replace with real operation
  const getWalletBalance = async () => '1200 NYM';

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

  const handleNewDelegation = async (identityKey: string, amount: MajorCurrencyAmount) => {
    setConfirmationModalProps({
      status: 'loading',
      action: 'delegate',
    });
    setShowNewDelegationModal(false);
    setCurrentDelegationListActionItem(undefined);
    try {
      const tx = await addDelegation({
        identity: identityKey,
        amount,
      });
      await userBalance.fetchBalance();
      setConfirmationModalProps({
        status: 'success',
        action: 'delegate',
        balance: userBalance.balance?.printable_balance || '-',
        transactionUrl: tx.transaction_hash,
      });
    } catch (e) {
      setConfirmationModalProps({
        status: 'error',
        action: 'delegate',
        message: (e as Error).message,
      });
    }
  };

  const handleDelegateMore = async (identityKey: string, amount: MajorCurrencyAmount) => {
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
      const tx = await updateDelegation({
        ...currentDelegationListActionItem,
        amount,
      });
      setConfirmationModalProps({
        status: 'success',
        action: 'delegate',
        balance: await getWalletBalance(),
        transactionUrl: tx.transactionUrl,
      });
    } catch (e) {
      setConfirmationModalProps({
        status: 'error',
        action: 'delegate',
        message: (e as Error).message,
      });
    }
  };

  const handleUndelegate = async (identityKey: string) => {
    setConfirmationModalProps({
      status: 'loading',
      action: 'undelegate',
    });
    setShowUndelegateModal(false);
    setCurrentDelegationListActionItem(undefined);
    try {
      await undelegate(identityKey);
      setConfirmationModalProps({
        status: 'success',
        action: 'undelegate',
        balance: await getWalletBalance(),
        transactionUrl: '',
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
        setConfirmationModalProps({
          status: 'success',
          action: 'redeem',
          balance: await getWalletBalance(),
          recipient: clientDetails?.client_address,
          transactionUrl: tx.transactionUrl,
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
      setConfirmationModalProps({
        status: 'success',
        action: 'redeem-all',
        balance: await getWalletBalance(),
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
              href={`${explorerUrl}/network-components/mixnodes/`}
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
          estimatedMonthlyReward={50.123}
          accountBalance={userBalance.balance?.printable_balance}
          nodeUptimePercentage={99.28394}
          profitMarginPercentage={11.12334234}
          rewardInterval="weekly"
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
          estimatedMonthlyReward={0}
          accountBalance={userBalance.balance?.printable_balance}
          nodeUptimePercentage={currentDelegationListActionItem.avg_uptime_percent}
          profitMarginPercentage={currentDelegationListActionItem.profit_margin_percent}
          rewardInterval="weekly"
        />
      )}

      {currentDelegationListActionItem && showUndelegateModal && (
        <UndelegateModal
          open={showUndelegateModal}
          onClose={() => setShowUndelegateModal(false)}
          onOk={handleUndelegate}
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
