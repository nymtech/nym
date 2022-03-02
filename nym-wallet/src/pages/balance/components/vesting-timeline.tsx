import React, { useContext } from 'react'
import { Box, Tooltip, Typography } from '@mui/material'
import { format } from 'date-fns'
import { ClientContext } from '../../../context/main'

const calculateNubPosition = (arrLength: number, index: number) => (1 / arrLength) * 100 * (index + 1)

export const VestingTimeline: React.FC<{ percentageComplete: number }> = ({ percentageComplete }) => {
  const {
    userBalance: { currentVestingPeriod, vestingAccountInfo },
  } = useContext(ClientContext)

  const nextPeriod =
    typeof currentVestingPeriod === 'object' && !!vestingAccountInfo?.periods
      ? Number(vestingAccountInfo?.periods[currentVestingPeriod.In]?.start_time)
      : undefined

  return (
    <Box display="flex" flexDirection="column" gap={1} position="relative" width="100%">
      <svg width="100%" height="12">
        <rect y="2" width="100%" height="6" rx="0" fill="#E6E6E6" />
        <rect y="2" width={`${percentageComplete}%`} height="6" rx="0" fill="#121726" />
        <rect width="4" height="12" rx="1" fill="#121726" />
        {vestingAccountInfo?.periods.map((period, i, arr) => (
          <Tooltip title={format(new Date(Number(period.start_time) * 1000), 'HH:mm do MMM yyyy')} key={i}>
            <rect
              x={`${calculateNubPosition(arr.length, i)}%`}
              width="4"
              height="12"
              rx="1"
              fill={+percentageComplete.toFixed(2) >= calculateNubPosition(arr.length, i) ? '#121726' : '#B9B9B9'}
              style={{ cursor: 'pointer' }}
            />
          </Tooltip>
        ))}
      </svg>
      {nextPeriod && (
        <Typography variant="caption" sx={{ color: 'grey.500', position: 'absolute', top: 15, left: 0 }}>
          Next vesting period: {format(new Date(nextPeriod * 1000), 'HH:mm do MMM yyyy')}
        </Typography>
      )}
    </Box>
  )
}
