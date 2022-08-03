/* eslint-disable react/no-array-index-key */
import React, { useContext } from 'react';
import { Box, Tooltip, Typography } from '@mui/material';
import { format } from 'date-fns';
import { AppContext } from '../../../context/main';

const calculateMarkerPosition = (arrLength: number, index: number) => (1 / arrLength) * 100 * index;

const Marker: React.FC<{ tooltipText: string; color: string; position: string }> = ({
  tooltipText,
  color,
  position,
}) => (
  <Tooltip title={tooltipText}>
    <rect x={position} width="4" height="12" rx="1" fill={color} style={{ cursor: 'pointer' }} />
  </Tooltip>
);

export const VestingTimeline: React.FC<{ percentageComplete: number }> = ({ percentageComplete }) => {
  const {
    userBalance: { currentVestingPeriod, vestingAccountInfo },
  } = useContext(AppContext);

  const nextPeriod =
    typeof currentVestingPeriod === 'object' && !!vestingAccountInfo?.periods
      ? Number(vestingAccountInfo?.periods[currentVestingPeriod.In + 1]?.start_time)
      : undefined;

  return (
    <Box display="flex" flexDirection="column" gap={1} position="relative" width="100%">
      <svg width="100%" height="12">
        <rect y="2" width="100%" height="6" rx="0" fill="#E6E6E6" />
        <rect y="2" width={`${percentageComplete}%`} height="6" rx="0" fill="#121726" />
        {vestingAccountInfo?.periods.map((period, i, arr) => (
          <Marker
            position={`${calculateMarkerPosition(arr.length, i)}%`}
            color={+percentageComplete.toFixed(2) >= calculateMarkerPosition(arr.length, i) ? '#121726' : '#B9B9B9'}
            tooltipText={format(new Date(Number(period.start_time) * 1000), 'HH:mm do MMM yyyy')}
            key={i}
          />
        ))}
        <Marker
          position="calc(100% - 4px)"
          color={percentageComplete === 100 ? '#121726' : '#B9B9B9'}
          tooltipText="End of vesting schedule"
        />
      </svg>
      {!!nextPeriod && (
        <Typography
          variant="caption"
          sx={{ color: (theme) => theme.palette.text.disabled, position: 'absolute', top: 15, left: 0 }}
        >
          Next vesting period: {format(new Date(nextPeriod * 1000), 'HH:mm do MMM yyyy')}
        </Typography>
      )}
    </Box>
  );
};
