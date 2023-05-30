/* eslint-disable react/no-array-index-key */
import React, { useContext } from 'react';
import { useTheme } from '@mui/material/styles';
import { Box, Stack, Tooltip, Typography } from '@mui/material';
import { format } from 'date-fns';
import { AppContext } from 'src/context';

const calculateMarkerPosition = (arrLength: number, index: number) => (1 / arrLength) * 100 * index;

const Marker: FCWithChildren<{ tooltipText: string; color: string; position: string }> = ({
  tooltipText,
  color,
  position,
}) => (
  <Tooltip title={tooltipText}>
    <rect x={position} width="4" height="12" rx="1" fill={color} style={{ cursor: 'pointer' }} />
  </Tooltip>
);

export const VestingTimeline: FCWithChildren<{ percentageComplete: number }> = ({ percentageComplete }) => {
  const {
    userBalance: { currentVestingPeriod, vestingAccountInfo },
  } = useContext(AppContext);

  const theme = useTheme();

  const nextPeriod =
    typeof currentVestingPeriod === 'object' && !!vestingAccountInfo?.periods
      ? Number(vestingAccountInfo?.periods[currentVestingPeriod.In + 1]?.start_time)
      : undefined;

  return (
    <Box>
      <Stack direction="row" gap={1} alignItems="center">
        <Typography variant="body2">{percentageComplete}%</Typography>
        <svg width="100%" height="12">
          <rect y="2" width="100%" height="6" rx="0" fill="#E6E6E6" />
          <rect y="2" width={`${percentageComplete}%`} height="6" rx="0" fill={theme.palette.success.main} />
          {vestingAccountInfo?.periods.map((period, i, arr) => (
            <Marker
              position={`${calculateMarkerPosition(arr.length, i)}%`}
              color={
                Math.ceil(+percentageComplete) >= calculateMarkerPosition(arr.length, i)
                  ? theme.palette.success.main
                  : '#B9B9B9'
              }
              tooltipText={format(new Date(Number(period.start_time) * 1000), 'HH:mm do MMM yyyy')}
              key={i}
            />
          ))}
          <Marker
            position="calc(100% - 4px)"
            color={percentageComplete === 100 ? theme.palette.success.main : '#B9B9B9'}
            tooltipText="End of vesting schedule"
          />
        </svg>
      </Stack>
      {!!nextPeriod && (
        <Typography variant="caption" sx={{ color: 'nym.text.muted', ml: 6 }}>
          Next vesting period: {format(new Date(nextPeriod * 1000), 'HH:mm do MMM yyyy')}
        </Typography>
      )}
    </Box>
  );
};
