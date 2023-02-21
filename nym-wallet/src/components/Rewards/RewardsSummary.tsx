import React from 'react';
import { CircularProgress, Stack, Typography } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { InfoTooltip } from '../InfoToolTip';

export const RewardsSummary: React.FC<{
  isLoading?: boolean;
  totalDelegation?: string;
  totalRewards?: string;
}> = ({ isLoading, totalDelegation, totalRewards }) => {
  const theme = useTheme();
  return (
    <Stack direction="row" justifyContent="space-between">
      <Stack direction="row" spacing={4}>
        <Stack direction="row" spacing={1} alignItems="center">
          <InfoTooltip title="This is the total amount you have delegated to node(s) in the network" />
          <Typography>Total delegations:</Typography>
          <Typography fontWeight={600} fontSize={16} textTransform="uppercase">
            {isLoading ? <CircularProgress size={theme.typography.fontSize} /> : totalDelegation || '-'}
          </Typography>
        </Stack>
        <Stack direction="row" spacing={1} alignItems="center">
          <InfoTooltip title="This is the rewards you have accrued since the last time you redeemed your rewards. Rewards are automatically compounded. You can redeem your rewards at any time" />
          <Typography>New rewards:</Typography>
          <Typography fontWeight={600} fontSize={16} textTransform="uppercase">
            {isLoading ? <CircularProgress size={theme.typography.fontSize} /> : totalRewards || '-'}
          </Typography>
        </Stack>
      </Stack>
    </Stack>
  );
};
