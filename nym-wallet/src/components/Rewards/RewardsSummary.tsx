import React, { useMemo } from 'react';
import { CircularProgress, Stack, Typography } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { InfoTooltip } from '../InfoToolTip';
import { useDelegationContext } from 'src/context/delegations';

export const RewardsSummary: FCWithChildren<{
  isLoading?: boolean;
  totalDelegation?: string;
  totalRewards?: string;
}> = ({ isLoading, totalDelegation, totalRewards }) => {
  const theme = useTheme();

  const { totalDelegationsAndRewards } = useDelegationContext();
  return (
    <Stack direction="row" justifyContent="space-between" marginTop={3}>
      <Stack direction="row" spacing={5}>
        <Stack direction="row" spacing={1} alignItems="center">
          <InfoTooltip title="This is the total amount you have delegated to node(s) in the network" />
          <Typography>Total delegations:</Typography>
          <Typography fontWeight={600} fontSize={16} textTransform="uppercase">
            {isLoading ? <CircularProgress size={theme.typography.fontSize} /> : totalDelegationsAndRewards || '-'}
          </Typography>
        </Stack>
        <Stack direction="row" spacing={1} alignItems="center">
          <InfoTooltip title="This is the total amount you have delegated to node(s) in the network" />
          <Typography>Original delegations:</Typography>
          <Typography fontWeight={600} fontSize={16} textTransform="uppercase">
            {isLoading ? <CircularProgress size={theme.typography.fontSize} /> : totalDelegation || '-'}
          </Typography>
        </Stack>
        <Stack direction="row" spacing={1} alignItems="center">
          <InfoTooltip title="The total amount you have delegated to node(s) in the network. The amount also includes the rewards you have accrued since last time you claimed your rewards" />
          <Typography>Total rewards:</Typography>
          <Typography fontWeight={600} fontSize={16} textTransform="uppercase">
            {isLoading ? <CircularProgress size={theme.typography.fontSize} /> : totalRewards || '-'}
          </Typography>
        </Stack>
      </Stack>
    </Stack>
  );
};
