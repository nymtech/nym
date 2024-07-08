import React, { useEffect } from 'react';
import { Refresh } from '@mui/icons-material';
import { Grid, IconButton, Typography } from '@mui/material';
import { useSnackbar } from 'notistack';
import { NymCard } from '@src/components';
import { TokenTransfer } from '@src/components/Balance/cards/TokenTransfer';
import { OriginalVestingResponse } from '@nymproject/types';
import { VestingSchedule } from '@src/components/Balance/cards/VestingSchedule';

export const VestingCard = ({
  unlockedTokens,
  unlockedRewards,
  unlockedTransferable,
  originalVesting,
  onTransfer,
  fetchBalance,
  fetchTokenAllocation,
}: {
  unlockedTokens?: string;
  unlockedRewards?: string;
  unlockedTransferable?: string;
  originalVesting?: OriginalVestingResponse;
  fetchTokenAllocation: () => Promise<void>;
  fetchBalance: () => Promise<void>;
  onTransfer: () => Promise<void>;
}) => {
  const { enqueueSnackbar, closeSnackbar } = useSnackbar();

  const refreshBalances = async () => {
    await fetchBalance();
    await fetchTokenAllocation();
  };

  useEffect(() => {
    closeSnackbar();
    fetchTokenAllocation();
  }, []);

  if (!originalVesting) return null;

  return (
    <NymCard
      title="Vesting Schedule"
      subheader={
        <Typography variant="caption" sx={{ color: 'nym.text.muted' }}>
          You can use up to 10% of your locked tokens for bonding and delegating
        </Typography>
      }
      borderless
      data-testid="check-unvested-tokens"
      Action={
        <IconButton
          onClick={async () => {
            await refreshBalances();
            enqueueSnackbar('Balances updated', { variant: 'success', preventDuplicate: true });
          }}
        >
          <Refresh />
        </IconButton>
      }
    >
      <Grid container spacing={3}>
        <Grid item xs={12} md={7} lg={8}>
          <VestingSchedule />
        </Grid>
        <Grid item xs={12} md={5} lg={4}>
          <TokenTransfer
            onTransfer={onTransfer}
            unlockedTokens={unlockedTokens}
            unlockedRewards={unlockedRewards}
            unlockedTransferable={unlockedTransferable}
          />
        </Grid>
      </Grid>
    </NymCard>
  );
};
