import React from 'react';
import { CircularProgress, Stack, Typography } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { InfoTooltip } from '../InfoToolTip';
import { Link } from '@nymproject/react/link/Link';

export const RewardsSummary: React.FC<{
  isLoading?: boolean;
  explorerUrl?: string;
  totalDelegation?: string;
  totalRewards?: string;
}> = ({ isLoading, explorerUrl, totalDelegation, totalRewards }) => {
  if (explorerUrl)
    return (
      <Typography>
        Check out a <Link target="_blank" text="list of mixnodes " href={explorerUrl} noIcon /> for performance and
        other parameters to help make a delegation decision
      </Typography>
    );

  const theme = useTheme();
  return (
    <Stack direction="row" justifyContent="space-between">
      <Stack direction="row" spacing={4}>
        <Stack direction="row" spacing={1} alignItems="center">
          <InfoTooltip title="This is the total amount you have delgated across multiple nodes" />
          <Typography>Total delegations:</Typography>
          <Typography fontWeight={600} fontSize={16} textTransform="uppercase">
            {isLoading ? <CircularProgress size={theme.typography.fontSize} /> : totalDelegation || '-'}
          </Typography>
        </Stack>
        <Stack direction="row" spacing={1} alignItems="center">
          <InfoTooltip title="Awaiting rewards accrue per epoch (hourly). You can redeem or compound them" />
          <Typography>New rewards:</Typography>
          <Typography fontWeight={600} fontSize={16} textTransform="uppercase">
            {isLoading ? <CircularProgress size={theme.typography.fontSize} /> : totalRewards || '-'}
          </Typography>
        </Stack>
      </Stack>
    </Stack>
  );
};
