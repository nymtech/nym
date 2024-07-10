import { FC, useContext, useEffect, useState } from 'react';
import { Box, Button, Paper, Stack, Typography } from '@mui/material';
import { Theme, useTheme } from '@mui/material/styles';
import { DecCoin, decimalToFloatApproximation, DelegationWithEverything, FeeDetails } from '@nymproject/types';
import { Link } from '@nymproject/react';
import { AppContext, urls } from '@src/context/main';
import { DelegationList } from '@src/components/Delegation/DelegationList';
import { TPoolOption } from '@src/components';
import { Console } from '@src/utils/console';
import { OverSaturatedBlockerModal } from '@src/components/Delegation/DelegateBlocker';
import { getSpendableCoins, userBalance } from '@src/requests';
import { LoadingModal } from '@src/components/Modals/LoadingModal';
import { getIntervalAsDate, toPercentIntegerString } from '@src/utils';
import { RewardsSummary } from '../../components/Rewards/RewardsSummary';
import { DelegationContextProvider, TDelegations, useDelegationContext } from '../../context/delegations';
import { RewardsContextProvider, useRewardsContext } from '../../context/rewards';
import { DelegateModal } from '../../components/Delegation/DelegateModal';
import { UndelegateModal } from '../../components/Delegation/UndelegateModal';
import { DelegationListItemActions } from '../../components/Delegation/DelegationActions';
import { RedeemModal } from '../../components/Rewards/RedeemModal';
import { DelegationModal, DelegationModalProps } from '../../components/Delegation/DelegationModal';
import { backDropStyles, modalStyles } from '@src/components/Modals/styles';

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
  const [confirmationModalProps, setConfirmationModalProps] = useState<DelegationModalProps | undefined>();
  const [currentDelegationListActionItem, setCurrentDelegationListActionItem] = useState<DelegationWithEverything>();
  const [saturationError, setSaturationError] = useState<{ action: 'compound' | 'delegate'; saturation: string }>();
  const [nextEpoch, setNextEpoch] = useState<string | Error>();

  const theme = useTheme();
  const {
    clientDetails,
    network,
    userBalance: { balance, originalVesting, fetchBalance },
  } = useContext(AppContext);

  const {
    delegations,
    totalDelegations,
    totalRewards,
    isLoading,
    addDelegation,
    undelegate,
    undelegateVesting,
    refresh: refreshDelegations,
  } = useDelegationContext();

  const { refresh: refreshRewards, claimRewards } = useRewardsContext();

  const refresh = async () => Promise.all([refreshDelegations(), refreshRewards()]);

  // If an action modal is open, don't show the loading modal
  const isActionModalOpen =
    showNewDelegationModal ||
    showDelegateMoreModal ||
    showUndelegateModal ||
    showRedeemRewardsModal ||
    confirmationModalProps ||
    saturationError;

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

  const getNextInterval = async () => {
    try {
      const { nextEpoch: newNextEpoch } = await getIntervalAsDate();
      setNextEpoch(newNextEpoch);
    } catch {
      setNextEpoch(Error());
    }
  };

  const refreshWithIntervalUpdate = async () => {
    refresh();
    getNextInterval();
  };

  // Refresh the rewards and delegations periodically when page is mounted
  useEffect(() => {
    const timer = setInterval(refreshWithIntervalUpdate, 5 * 60 * 1000); // every 5 minutes
    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    refreshWithIntervalUpdate();
  }, [clientDetails, confirmationModalProps]);

  const handleDelegationItemActionClick = (item: DelegationWithEverything, action: DelegationListItemActions) => {
    if (
      (action === 'delegate' || action === 'compound') &&
      item.stake_saturation &&
      decimalToFloatApproximation(item.stake_saturation) > 1
    ) {
      setSaturationError({ action, saturation: item.stake_saturation });
      return;
    }

    setCurrentDelegationListActionItem(item);
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

  const handleNewDelegation = async (
    mix_id: number,
    _: string,
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
          mix_id,
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

  const handleDelegateMore = async (
    mix_id: number,
    identityKey: string,
    amount: DecCoin,
    tokenPool: TPoolOption,
    fee?: FeeDetails,
  ) => {
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
          mix_id,
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

  const handleUndelegate = async (
    mixId: number,
    // identityKey is no longer used
    _: string,
    usesVestingContractTokens: boolean,
    fee?: FeeDetails,
  ) => {
    setConfirmationModalProps({
      status: 'loading',
      action: 'undelegate',
    });
    setShowUndelegateModal(false);
    setCurrentDelegationListActionItem(undefined);
    let tx;
    try {
      if (usesVestingContractTokens) {
        tx = await undelegateVesting(mixId);
      } else {
        tx = await undelegate(mixId, fee?.fee);
      }

      const balances = await getAllBalances();

      setConfirmationModalProps({
        status: 'success',
        action: 'undelegate',
        message: 'This operation can take up to one hour to process',
        ...balances,
        transactions: [
          {
            url: `${urls(network).blockExplorer}/transaction/${tx.transaction_hash}`,
            hash: tx.transaction_hash,
          },
        ],
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

  const handleRedeem = async (mixId: number, _: string, fee?: FeeDetails) => {
    setConfirmationModalProps({
      status: 'loading',
      action: 'redeem',
    });
    setShowRedeemRewardsModal(false);
    setCurrentDelegationListActionItem(undefined);

    try {
      const txs = await claimRewards(mixId, fee);
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

  const delegationsComponent = (delegationItems: TDelegations | undefined) => {
    if (delegationItems && Boolean(delegationItems?.length)) {
      return (
        <DelegationList
          explorerUrl={urls(network).networkExplorer}
          isLoading={isLoading && !isActionModalOpen}
          items={delegationItems}
          onItemActionClick={handleDelegationItemActionClick}
        />
      );
    }

    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'flex-end' }}>
        <Box marginRight={3} width={1}>
          <Typography variant="body2">
            Checkout the{' '}
            <Link
              href={`${urls(network).networkExplorer}/network-components/mixnodes/`}
              target="_blank"
              rel="noreferrer"
              text="list of mixnodes"
              fontWeight={theme.palette.mode === 'light' ? 400 : 600}
              noIcon
            />{' '}
            for performance and other parameters to help make delegation decisions.
          </Typography>
          <Typography variant="body2">
            Hint: In Nym explorer use <b>advanced filters</b> to precisely define what node you are looking for.
          </Typography>
        </Box>
        <Button
          variant="contained"
          disableElevation
          onClick={() => setShowNewDelegationModal(true)}
          sx={{ py: 1.5, px: 5, color: 'primary.contrastText' }}
        >
          Delegate
        </Button>
      </Box>
    );
  };

  if (isLoading) {
    return <LoadingModal />;
  }

  return (
    <>
      <Paper elevation={0} sx={{ p: 3, mt: 4 }}>
        <Stack spacing={3}>
          <Box display="flex" justifyContent="space-between">
            {' '}
            <Box display="flex" flexDirection="column">
              <Typography variant="h6" lineHeight={1.334} fontWeight={600}>
                Delegations
              </Typography>
              {!!delegations?.length && (
                <Stack marginTop={1.5} gap={0.5} direction="row" alignItems="center">
                  <Typography fontSize={14}>Select nodes to delegate to using the</Typography>
                  <Link
                    href={`${urls(network).networkExplorer}/network-components/mixnodes/`}
                    target="_blank"
                    rel="noreferrer"
                    text="network Explorer"
                    fontSize={14}
                    fontWeight={theme.palette.mode === 'light' ? 400 : 600}
                    noIcon
                  />
                </Stack>
              )}
            </Box>
            {!!delegations?.length && (
              <Button
                variant="contained"
                disableElevation
                onClick={() => setShowNewDelegationModal(true)}
                sx={{ py: 1.5, px: 5, color: 'primary.contrastText', height: 'fit-content' }}
              >
                Delegate
              </Button>
            )}
          </Box>

          {!!delegations?.length && (
            <Box display="flex" justifyContent="space-between" alignItems="end">
              <RewardsSummary isLoading={false} totalDelegation={totalDelegations} totalRewards={totalRewards} />
              {nextEpoch instanceof Error ? null : (
                <Typography fontSize={14}>
                  Next epoch starts at <b>{nextEpoch}</b>
                </Typography>
              )}
            </Box>
          )}
          {delegationsComponent(delegations)}
        </Stack>
      </Paper>

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
          profitMarginPercentage={
            currentDelegationListActionItem.cost_params?.profit_margin_percent &&
            toPercentIntegerString(currentDelegationListActionItem.cost_params?.profit_margin_percent)
          }
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
          mixId={currentDelegationListActionItem.mix_id}
          identityKey={currentDelegationListActionItem.node_identity}
        />
      )}

      {currentDelegationListActionItem?.unclaimed_rewards && showRedeemRewardsModal && (
        <RedeemModal
          open={showRedeemRewardsModal}
          onClose={() => setShowRedeemRewardsModal(false)}
          onOk={(mixId, identity, fee) => handleRedeem(mixId, identity, fee)}
          message="Claim rewards"
          denom={clientDetails?.display_mix_denom || 'nym'}
          mixId={currentDelegationListActionItem.mix_id}
          identityKey={currentDelegationListActionItem?.node_identity}
          amount={+currentDelegationListActionItem.unclaimed_rewards.amount}
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
          header={`Node saturation: ${
            Math.round(decimalToFloatApproximation(saturationError.saturation) * 100000) / 1000
          }%`}
          subHeader="This node is over saturated. Choose a new mix node to delegate to and start compounding rewards."
        />
      )}
    </>
  );
};

export const DelegationPage: FC<{ isStorybook?: boolean }> = ({ isStorybook }) => (
  <DelegationContextProvider>
    <RewardsContextProvider>
      <Delegation isStorybook={isStorybook} />
    </RewardsContextProvider>
  </DelegationContextProvider>
);
