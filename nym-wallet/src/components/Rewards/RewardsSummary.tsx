import React from 'react';
import { CircularProgress, Stack, Typography } from '@mui/material';
import { useTheme } from '@mui/material/styles';

export const RewardsSummary: React.FC<{
  isLoading?: boolean;
  totalDelegation?: string;
  totalRewards?: string;
}> = ({ isLoading, totalDelegation, totalRewards }) => {
  const theme = useTheme();
  return (
    <Stack direction="row" justifyContent="space-between" alignItems="center">
      <Stack direction="row" spacing={4}>
        <Stack direction="row" spacing={2}>
          <Typography>Total delegations:</Typography>
          <Typography fontWeight={600}>
            {isLoading ? <CircularProgress size={theme.typography.fontSize} /> : totalDelegation || '-'}
          </Typography>
        </Stack>
        <Stack direction="row" spacing={2}>
          <Typography>New rewards:</Typography>
          <Typography fontWeight={600}>
            {isLoading ? <CircularProgress size={theme.typography.fontSize} /> : totalRewards || '-'}
          </Typography>
        </Stack>
      </Stack>
    </Stack>
  );
};
