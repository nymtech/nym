import React from 'react';
import { Box, Typography } from '@mui/material';
import { useIsMobile } from '../../../hooks/useIsMobile';
import { EconomicsProgress } from './EconomicsProgress';

export const StakeSaturationProgressBar = ({ value, threshold }: { value: number; threshold: number }) => {
  const isTablet = useIsMobile('lg');
  const percentageColor = value > (threshold || 100) ? 'warning' : 'inherit';
  const textColor = percentageColor === 'warning' ? 'warning.main' : 'nym.wallet.fee';

  return (
    <Box
      sx={{ display: 'flex', alignItems: 'center', flexDirection: isTablet ? 'column' : 'row' }}
      id="field"
      color={percentageColor}
    >
      <Typography
        sx={{
          mr: isTablet ? 0 : 1,
          mb: isTablet ? 1 : 0,
          fontWeight: '600',
          fontSize: '12px',
          color: textColor,
        }}
        id="stake-saturation-progress-bar"
      >
        {value}%
      </Typography>
      <EconomicsProgress value={value} threshold={threshold} color={percentageColor} />
    </Box>
  );
};
