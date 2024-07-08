import React from 'react';
import { CircularProgress, Stack, StackProps, Typography, useMediaQuery } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { useDelegationContext } from '@src/context/delegations';
import { InfoTooltip } from '../InfoToolTip';

const RewardSummaryField = ({
  title,
  value,
  Tooltip,
  isLoading,
}: {
  title: string;
  value: string;
  Tooltip?: React.ReactNode;
  isLoading?: boolean;
}) => {
  const breakpoint = useMediaQuery(useTheme().breakpoints.down('xl'));
  const alignProps: { gap: number; direction: StackProps['direction'] } = {
    gap: breakpoint ? 0 : 1,
    direction: breakpoint ? 'column' : 'row',
  };

  return (
    <Stack {...alignProps} alignItems="start">
      <Stack direction="row" alignItems="center" gap={1}>
        {Tooltip}
        <Typography>{title}:</Typography>
      </Stack>
      <Typography fontWeight={600} fontSize={16} textTransform="uppercase">
        {isLoading ? <CircularProgress size={16} /> : value}
      </Typography>
    </Stack>
  );
};

export const RewardsSummary: FCWithChildren<{
  isLoading?: boolean;
  totalDelegation?: string;
  totalRewards?: string;
}> = ({ isLoading, totalDelegation, totalRewards }) => {
  const { totalDelegationsAndRewards } = useDelegationContext();
  return (
    <Stack direction="row" justifyContent="space-between" marginTop={3}>
      <Stack direction="row" spacing={5}>
        <RewardSummaryField
          title="Total delegations"
          value={totalDelegationsAndRewards || '-'}
          isLoading={isLoading}
          Tooltip={
            <InfoTooltip title="The total amount you have delegated to node(s) in the network. The amount also includes the rewards you have accrued since last time you claimed your rewards" />
          }
        />
        <RewardSummaryField
          title="Original delegations"
          value={totalDelegation || '-'}
          isLoading={isLoading}
          Tooltip={<InfoTooltip title="The initial amount you delegated to the node(s)" />}
        />
        <RewardSummaryField
          title="Total rewards"
          value={totalRewards || '-'}
          isLoading={isLoading}
          Tooltip={
            <InfoTooltip title="The rewards you have accrued since the last time you claimed your rewards. Rewards are automatically compounded. You can claim your rewards at any time." />
          }
        />
      </Stack>
    </Stack>
  );
};
