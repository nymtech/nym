import React, { FC, useContext, useEffect, useRef, useState } from 'react';
import { OpenInNew } from '@mui/icons-material';
import { Alert, AlertTitle, Box, Button, CircularProgress, LinearProgress, Stack, Typography } from '@mui/material';
import { alpha, useTheme } from '@mui/material/styles';
import { DecCoin, decimalToFloatApproximation, DelegationWithEverything, FeeDetails } from '@nymproject/types';
import { TauriLink as Link } from 'src/components/TauriLinkWrapper';
import { AppContext, urls } from 'src/context/main';
import { DelegationList } from 'src/components/Delegation/DelegationList';
import { TPoolOption } from 'src/components';
import { Console } from 'src/utils/console';
import { OverSaturatedBlockerModal } from 'src/components/Delegation/DelegateBlocker';
import { getSpendableCoins, migrateVestedDelegations, userBalance } from 'src/requests';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { getIntervalAsDate, toPercentIntegerString } from 'src/utils';
import { isDelegation, TDelegations, useDelegationContext } from '../../context/delegations';
import { useRewardsContext } from '../../context/rewards';
import { DelegateModal } from '../../components/Delegation/DelegateModal';
import { UndelegateModal } from '../../components/Delegation/UndelegateModal';
import { DelegationListItemActions } from '../../components/Delegation/DelegationActions';
import { RedeemModal } from '../../components/Rewards/RedeemModal';
import { DelegationModal, DelegationModalProps } from '../../components/Delegation/DelegationModal';
import { VestingWarningModal } from '../../components/VestingWarningModal';
import { PageLayout } from '../../layouts';

export const DelegationPage: FC = () => {
  const [showNewDelegationModal, setShowNewDelegationModal] = useState<boolean>(false);
  const [showDelegateMoreModal, setShowDelegateMoreModal] = useState<boolean>(false);
  const [showUndelegateModal, setShowUndelegateModal] = useState<boolean>(false);
  const [showRedeemRewardsModal, setShowRedeemRewardsModal] = useState<boolean>(false);
  const [confirmationModalProps, setConfirmationModalProps] = useState<DelegationModalProps | undefined>();
  const [currentDelegationListActionItem, setCurrentDelegationListActionItem] = useState<DelegationWithEverything>();
  const [saturationError, setSaturationError] = useState<{ action: 'compound' | 'delegate'; saturation: string }>();
  const [nextEpoch, setNextEpoch] = useState<string | Error>();
  const [showVestingWarningModal, setShowVestingWarningModal] = useState<boolean>(false);
  const [showVestingMigrationProgressModal, setShowVestingMigrationProgressModal] = useState<boolean>(false);

  const theme = useTheme();
  const {
    clientDetails,
    network,
    userBalance: { balance, originalVesting, fetchBalance },
  } = useContext(AppContext);

  const {
    delegations,
    isLoading,
    addDelegation,
    undelegate,
    undelegateVesting,
    refresh: refreshDelegations,
  } = useDelegationContext();

  const delegationsUseVestingTokens: boolean = React.useMemo(
    () => Boolean(delegations?.filter((d) => isDelegation(d) && d.uses_vesting_contract_tokens).length),
    [delegations],
  );

  const { claimRewards } = useRewardsContext();

  const refresh = async () => refreshDelegations(delegations !== undefined ? { background: true } : undefined);

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

  const doMigrateNow = async () => {
    setShowVestingMigrationProgressModal(true);
    await migrateVestedDelegations();
    await refreshDelegations(undefined);
    setShowVestingMigrationProgressModal(false);
  };

  useEffect(() => {
    getNextInterval().catch((err) => {
      Console.error(err);
    });
  }, [clientDetails]);

  const prevConfirmationModalProps = useRef<DelegationModalProps | undefined>(undefined);
  useEffect(() => {
    if (prevConfirmationModalProps.current !== undefined && confirmationModalProps === undefined) {
      refreshWithIntervalUpdate().catch((err) => {
        Console.error(err);
      });
    }
    prevConfirmationModalProps.current = confirmationModalProps;
  }, [confirmationModalProps]);

  const handleDelegationItemActionClick = (item: DelegationWithEverything, action: DelegationListItemActions) => {
    if (
      (action === 'delegate' || action === 'compound') &&
      item.stake_saturation &&
      decimalToFloatApproximation(item.stake_saturation) > 1
    ) {
      setSaturationError({ action, saturation: item.stake_saturation });
      return;
    }

    if (item.uses_vesting_contract_tokens) {
      setShowVestingWarningModal(true);
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
        transactions: [{ url: `${urls(network).blockExplorer}/tx/${tx.transaction_hash}`, hash: tx.transaction_hash }],
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
        transactions: [{ url: `${urls(network).blockExplorer}/tx/${tx.transaction_hash}`, hash: tx.transaction_hash }],
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
            url: `${urls(network).blockExplorer}/tx/${tx.transaction_hash}`,
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

  const handleRedeem = async (mixId: number, identityKey: string, fee?: FeeDetails) => {
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
          url: `${urls(network).blockExplorer}/tx/${tx.transaction_hash}`,
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
    if (delegationItems === undefined) {
      return (
        <Box sx={{ py: 10, display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
          <CircularProgress size={36} aria-label="Loading delegations" />
        </Box>
      );
    }

    if (delegationItems.length > 0) {
      return (
        <>
          {delegationsUseVestingTokens && (
            <>
              <Alert severity="warning">
                <AlertTitle sx={{ fontWeight: 600 }}>
                  Some of your delegations are using tokens from the vesting contract!
                </AlertTitle>
                <Typography>
                  In order to claim your rewards, you will need to migrate them out of the vesting contract.{' '}
                </Typography>
                <Typography mt={1}>
                  <strong>Never fear</strong>, if you do not migrate them,{' '}
                  <strong>you will continue to get rewards</strong>. However, please migrate your delegations as soon as
                  possible.
                </Typography>
                <Button
                  variant="contained"
                  size="small"
                  sx={{ mt: 1 }}
                  onClick={() => setShowVestingWarningModal(true)}
                >
                  Migrate now
                </Button>
              </Alert>
              <VestingWarningModal
                kind="delegations"
                isVisible={showVestingWarningModal}
                handleMigrate={doMigrateNow}
                handleClose={() => setShowVestingWarningModal(false)}
              />
              {showVestingMigrationProgressModal && <LoadingModal text="Migrating delegations, please wait..." />}
            </>
          )}
          <DelegationList
            explorerUrl={urls(network).networkExplorer}
            items={delegationItems}
            onItemActionClick={handleDelegationItemActionClick}
            nextEpoch={nextEpoch}
          />
        </>
      );
    }

    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'flex-end' }}>
        <Box marginRight={3} width={1}>
          <Typography variant="body2">
            Checkout the{' '}
            <Link
              href={`${urls(network).networkExplorer}/nodes/`}
              target="_blank"
              rel="noreferrer"
              text="list of nym-nodes"
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

  return (
    <>
      <PageLayout>
        <Stack spacing={3}>
          <Box
            sx={{
              p: { xs: 2, md: 3 },
              maxWidth: '100%',
              overflowX: 'hidden',
              borderRadius: 4,
              bgcolor: 'background.paper',
              border: (t) => `1px solid ${t.palette.divider}`,
              boxShadow: (t) => t.palette.nym.nymWallet.shadows.light,
            }}
          >
            <Stack spacing={3}>
              {!!delegations?.length && (
                <Stack
                  direction={{ xs: 'column', sm: 'row' }}
                  alignItems={{ xs: 'stretch', sm: 'center' }}
                  justifyContent="space-between"
                  gap={2}
                >
                  <Box
                    sx={{
                      flex: 1,
                      minWidth: 0,
                      display: 'flex',
                      alignItems: 'flex-start',
                      gap: 1.25,
                      p: 1.75,
                      borderRadius: 2,
                      border: (t) => `1px solid ${t.palette.divider}`,
                      bgcolor: (t) =>
                        t.palette.mode === 'dark'
                          ? alpha(t.palette.common.white, 0.04)
                          : alpha(t.palette.common.black, 0.04),
                    }}
                  >
                    <OpenInNew
                      sx={{
                        fontSize: 20,
                        mt: 0.125,
                        color: 'primary.main',
                        flexShrink: 0,
                      }}
                    />
                    <Typography fontSize={14} color="text.secondary" sx={{ lineHeight: 1.5 }}>
                      Select nodes using the{' '}
                      <Link
                        href={`${urls(network).networkExplorer}/nodes`}
                        target="_blank"
                        rel="noreferrer"
                        text="network explorer"
                        fontSize={14}
                        fontWeight={600}
                        noIcon
                      />
                      . Compare performance and filters before you delegate.
                    </Typography>
                  </Box>
                  <Button
                    variant="contained"
                    disableElevation
                    onClick={() => setShowNewDelegationModal(true)}
                    sx={{
                      py: 1.5,
                      px: 4,
                      color: 'primary.contrastText',
                      flexShrink: 0,
                      alignSelf: { xs: 'stretch', sm: 'center' },
                    }}
                  >
                    New delegation
                  </Button>
                </Stack>
              )}

              {isLoading && delegations !== undefined && !isActionModalOpen && (
                <LinearProgress
                  sx={{
                    height: 3,
                    borderRadius: 3,
                    '& .MuiLinearProgress-bar': { borderRadius: 3 },
                  }}
                />
              )}

              <Box sx={{ width: '100%', overflowX: 'hidden' }}>{delegationsComponent(delegations)}</Box>
            </Stack>
          </Box>
        </Stack>
      </PageLayout>

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
