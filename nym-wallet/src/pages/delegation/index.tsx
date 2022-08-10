import React, { FC, useContext, useEffect, useState } from 'react';
import { Box, Button, Paper, Stack, Typography } from '@mui/material';
import { useTheme, Theme } from '@mui/material/styles';
import { DelegationWithEverything, FeeDetails, DecCoin } from '@nymproject/types';
import { Link } from '@nymproject/react/link/Link';
import { AppContext, urls } from 'src/context/main';
import { DelegationList } from 'src/components/Delegation/DelegationList';
import { PendingEvents } from 'src/components/Delegation/PendingEvents';
import { TPoolOption } from 'src/components';
import { Console } from 'src/utils/console';
import { CompoundModal } from 'src/components/Rewards/CompoundModal';
import { OverSaturatedBlockerModal } from 'src/components/Delegation/DelegateBlocker';
import { getSpendableCoins, userBalance } from 'src/requests';
import { RewardsSummary } from '../../components/Rewards/RewardsSummary';
import { useDelegationContext, DelegationContextProvider } from '../../context/delegations';
import { RewardsContextProvider, useRewardsContext } from '../../context/rewards';
import { DelegateModal } from '../../components/Delegation/DelegateModal';
import { UndelegateModal } from '../../components/Delegation/UndelegateModal';
import { DelegationListItemActions } from '../../components/Delegation/DelegationActions';
import { RedeemModal } from '../../components/Rewards/RedeemModal';
import { DelegationModal, DelegationModalProps } from '../../components/Delegation/DelegationModal';
import { backDropStyles, modalStyles } from '../../../.storybook/storiesStyles';

const storybookStyles = (theme: Theme, isStorybook?: boolean, backdropProps?: object) =>
  isStorybook
    ? {
        backdropProps: { ...backDropStyles(theme), ...backdropProps },
        sx: modalStyles(theme),
      }
    : {};

export const Delegation: FC<{ isStorybook?: boolean }> = ({ isStorybook }) => {
  const [showNewDelegationModal, setShowNewDelegationModal] = useState<boolean>(false);
  const [showDelegateMoreModal, setShowDelegateMoreModal] = useState<boolean>(false);
  const [showUndelegateModal, setShowUndelegateModal] = useState<boolean>(false);
  const [showRedeemRewardsModal, setShowRedeemRewardsModal] = useState<boolean>(false);
  const [showCompoundRewardsModal, setShowCompoundRewardsModal] = useState<boolean>(false);
  const [confirmationModalProps, setConfirmationModalProps] = useState<DelegationModalProps | undefined>();
  const [currentDelegationListActionItem, setCurrentDelegationListActionItem] = useState<DelegationWithEverything>();
  const [saturationError, setSaturationError] = useState<{ action: 'compound' | 'delegate'; saturation: number }>();

  const theme = useTheme();

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
    refresh: refreshDelegations,
  } = useDelegationContext();

  const { refresh: refreshRewards, claimRewards, compoundRewards } = useRewardsContext();

  const refresh = async () => Promise.all([refreshDelegations(), refreshRewards()]);

  const getAllBalances = async () => {
    const resBalance = (await userBalance()).printable_balance;
    let resVesting: DecCoin | undefined;
    try {
      resVesting = await getSpendableCoins();
    } catch (e) {
      // ignore errors
    }

    return {
      balance: resBalance,
      balanceVested: resVesting ? `${resVesting.amount} ${resVesting.denom}` : undefined,
    };
  };

  // Refresh the rewards and delegations periodically when page is mounted
  useEffect(() => {
    const timer = setInterval(refresh, 1 * 60 * 1000); // every 1 minute
    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    refresh();
  }, [clientDetails, confirmationModalProps]);

  const handleDelegationItemActionClick = (item: DelegationWithEverything, action: DelegationListItemActions) => {
    if ((action === 'delegate' || action === 'compound') && item.stake_saturation && item.stake_saturation > 1) {
      setSaturationError({ action, saturation: item.stake_saturation });
      return;
    }

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

  const handleNewDelegation = async (
    identityKey: string,
    amount: DecCoin,
    tokenPool: TPoolOption,
    fee?: FeeDetails,
  ) => {
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
        fee,
      );

      const balances = await getAllBalances();

      setConfirmationModalProps({
        status: 'success',
        action: 'delegate',
        message: 'This operation can take up to one hour to process',
        ...balances,
        transactions: [
          { url: `${urls(network).blockExplorer}/transaction/${tx.transaction_hash}`, hash: tx.transaction_hash },
        ],
      });
    } catch (e) {
      Console.error('Failed to addDelegation', e);
      setConfirmationModalProps({
        status: 'error',
        action: 'delegate',
        message: (e as Error).message,
      });
    }
  };

  const handleDelegateMore = async (identityKey: string, amount: DecCoin, tokenPool: TPoolOption, fee?: FeeDetails) => {
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
        fee,
      );
      const balances = await getAllBalances();

      setConfirmationModalProps({
        status: 'success',
        action: 'delegate',
        ...balances,
        transactions: [
          { url: `${urls(network).blockExplorer}/transaction/${tx.transaction_hash}`, hash: tx.transaction_hash },
        ],
      });
    } catch (e) {
      Console.error('Failed to addMoreDelegation', e);
      setConfirmationModalProps({
        status: 'error',
        action: 'delegate',
        message: (e as Error).message,
      });
    }
  };

  const handleUndelegate = async (identityKey: string, usesVestingContractTokens: boolean, fee?: FeeDetails) => {
    setConfirmationModalProps({
      status: 'loading',
      action: 'undelegate',
    });
    setShowUndelegateModal(false);
    setCurrentDelegationListActionItem(undefined);

    try {
      const txs = await undelegate(identityKey, usesVestingContractTokens, fee);
      const balances = await getAllBalances();

      setConfirmationModalProps({
        status: 'success',
        action: 'undelegate',
        ...balances,
        transactions: txs.map((tx) => ({
          url: `${urls(network).blockExplorer}/transaction/${tx.transaction_hash}`,
          hash: tx.transaction_hash,
        })),
      });
    } catch (e) {
      Console.error('Failed to undelegate', e);
      setConfirmationModalProps({
        status: 'error',
        action: 'undelegate',
        message: (e as Error).message,
      });
    }
  };

  const handleRedeem = async (identityKey: string, fee?: FeeDetails) => {
    setConfirmationModalProps({
      status: 'loading',
      action: 'redeem',
    });
    setShowRedeemRewardsModal(false);
    setCurrentDelegationListActionItem(undefined);

    try {
      const txs = await claimRewards(identityKey, fee);
      setConfirmationModalProps({
        status: 'success',
        action: 'redeem',
        transactions: txs.map((tx) => ({
          url: `${urls(network).blockExplorer}/transaction/${tx.transaction_hash}`,
          hash: tx.transaction_hash,
        })),
      });
    } catch (e) {
      Console.error('Failed to claimRewards', e);
      setConfirmationModalProps({
        status: 'error',
        action: 'redeem',
        message: (e as Error).message,
      });
    }
  };

  const handleCompound = async (identityKey: string, fee?: FeeDetails) => {
    setConfirmationModalProps({
      status: 'loading',
      action: 'compound',
    });
    setShowCompoundRewardsModal(false);
    setCurrentDelegationListActionItem(undefined);

    try {
      const txs = await compoundRewards(identityKey, fee);
      setConfirmationModalProps({
        status: 'success',
        action: 'compound',
        transactions: txs.map((tx) => ({
          url: `${urls(network).blockExplorer}/transaction/${tx.transaction_hash}`,
          hash: tx.transaction_hash,
        })),
      });
    } catch (e) {
      Console.error('Failed to compoundRewards', e);
      setConfirmationModalProps({
        status: 'error',
        action: 'redeem',
        message: (e as Error).message,
      });
    }
  };

  return (
    <>
      <Paper elevation={0} sx={{ p: 3, mt: 4 }}>
        <Stack spacing={5}>
          <Box display="flex" justifyContent="space-between" alignItems="center">
            <Typography variant="h6">Delegations</Typography>
            <Link
              href={`${urls(network).networkExplorer}/network-components/mixnodes/`}
              target="_blank"
              rel="noreferrer"
              text="Network Explorer"
              fontSize={14}
              fontWeight={theme.palette.mode === 'light' ? 400 : 600}
              noIcon
            />
          </Box>
          <Box display="flex" justifyContent="space-between" alignItems="end">
            <RewardsSummary isLoading={isLoading} totalDelegation={totalDelegations} totalRewards={totalRewards} />
            <Button
              variant="contained"
              disableElevation
              onClick={() => setShowNewDelegationModal(true)}
              sx={{ py: 1.5, px: 5, color: 'primary.contrastText' }}
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
        <Paper elevation={0} sx={{ p: 3, mt: 2 }}>
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
          denom={clientDetails?.display_mix_denom || 'nym'}
          accountBalance={balance?.printable_balance}
          rewardInterval="weekly"
          hasVestingContract={Boolean(originalVesting)}
          {...storybookStyles(theme, isStorybook)}
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
          denom={clientDetails?.display_mix_denom || 'nym'}
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
          usesVestingContractTokens={currentDelegationListActionItem.uses_vesting_contract_tokens}
          currency={currentDelegationListActionItem.amount.denom}
          amount={+currentDelegationListActionItem.amount.amount}
          identityKey={currentDelegationListActionItem.node_identity}
        />
      )}

      {currentDelegationListActionItem?.accumulated_rewards && showRedeemRewardsModal && (
        <RedeemModal
          open={showRedeemRewardsModal}
          onClose={() => setShowRedeemRewardsModal(false)}
          onOk={(identity, fee) => handleRedeem(identity, fee)}
          message="Redeem rewards"
          denom={clientDetails?.display_mix_denom || 'nym'}
          identityKey={currentDelegationListActionItem?.node_identity}
          amount={+currentDelegationListActionItem.accumulated_rewards.amount}
          usesVestingTokens={currentDelegationListActionItem.uses_vesting_contract_tokens}
        />
      )}

      {currentDelegationListActionItem?.accumulated_rewards && showCompoundRewardsModal && (
        <CompoundModal
          open={showCompoundRewardsModal}
          onClose={() => setShowCompoundRewardsModal(false)}
          onOk={(identity, fee) => handleCompound(identity, fee)}
          message="Compound rewards"
          denom={clientDetails?.display_mix_denom || 'nym'}
          identityKey={currentDelegationListActionItem?.node_identity}
          amount={+currentDelegationListActionItem.accumulated_rewards.amount}
          usesVestingTokens={currentDelegationListActionItem.uses_vesting_contract_tokens}
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

      {!!saturationError && (
        <OverSaturatedBlockerModal
          open={Boolean(saturationError)}
          onClose={() => setSaturationError(undefined)}
          header={`Node saturation: ${Math.round(saturationError.saturation * 100000) / 1000}%`}
          subHeader="This node is over saturated. Choose a new mix node to delegate to and start compounding rewards."
        />
      )}
    </>
  );
};

export const DelegationPage: FC<{ isStorybook?: boolean }> = ({ isStorybook }) => {
  const { network } = useContext(AppContext);
  return (
    <DelegationContextProvider>
      <RewardsContextProvider>
        <Delegation isStorybook={isStorybook} />
      </RewardsContextProvider>
    </DelegationContextProvider>
  );
};
