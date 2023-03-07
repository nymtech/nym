import * as React from 'react';
import LinearProgress, { LinearProgressProps } from '@mui/material/LinearProgress';
import { useTheme } from '@mui/material/styles';
import { Box } from '@mui/system';

const parseToNumber = (value: number | undefined | string) =>
  typeof value === 'string' ? parseInt(value || '', 10) : value || 0;

export const EconomicsProgress: FCWithChildren<
  LinearProgressProps & {
    threshold?: number;
    color: string;
  }
> = ({ threshold, color, ...props }) => {
  const theme = useTheme();
  const { value } = props;

  const valueNumber: number = parseToNumber(value);
  const thresholdNumber: number = parseToNumber(threshold);
  const percentageToDisplay = Math.min(valueNumber, thresholdNumber);

  return (
    <Box
      sx={{
        width: 6 / 10,
        color: valueNumber > (threshold || 100) ? theme.palette.warning.main : theme.palette.nym.wallet.fee,
      }}
    >
      <LinearProgress
        {...props}
        variant="determinate"
        color={color}
        value={percentageToDisplay}
        sx={{ width: '100%', borderRadius: '5px' }}
      />
    </Box>
  );
};
