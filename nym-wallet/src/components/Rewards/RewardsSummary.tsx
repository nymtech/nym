import React from 'react';
import { Button, CircularProgress, Stack, Tooltip, Typography } from '@mui/material';
import { useTheme } from '@mui/material/styles';

export const RewardsSummary: React.FC<{
  isLoading?: boolean;
  totalDelegation?: string;
  totalRewards?: string;
  onClickRedeemAll?: () => void;
}> = ({ isLoading, totalDelegation, totalRewards, onClickRedeemAll }) => {
  const theme = useTheme();
  return (
    <Stack direction="row" justifyContent="space-between" alignItems="center">
      <Stack direction="row" spacing={4}>
        <Stack direction="row" spacing={2}>
          <Typography>Total delegation amount:</Typography>
          <Typography fontWeight={600}>
            {isLoading ? <CircularProgress size={theme.typography.fontSize} /> : totalDelegation || '-'}
          </Typography>
        </Stack>
        <Stack direction="row" spacing={2}>
          <Typography>Total unreedemed rewards:</Typography>
          <Typography fontWeight={600}>
            {isLoading ? <CircularProgress size={theme.typography.fontSize} /> : totalRewards || '-'}
          </Typography>
        </Stack>
      </Stack>
      <Tooltip title="Redeeming all rewards at once will be cheaper" arrow placement="left">
        <span>
          {/* <Button
            variant="outlined"
            color="secondary"
            size="large"
            onClick={onClickRedeemAll}
            disabled={!totalRewards || isLoading}
          >
            Redeem all rewards
          </Button> */}
        </span>
      </Tooltip>
    </Stack>
  );
};
