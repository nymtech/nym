import React from 'react';
import { Box, CircularProgress, Stack, Typography } from '@mui/material';

export const TestProgress = ({ totalPackets, packetsSent }: { totalPackets: number; packetsSent: number }) => {
  const percentage = Math.round((packetsSent / totalPackets) * 100);

  return (
    <Stack alignItems="center" gap={3}>
      <Typography sx={{ textTransform: 'uppercase' }}>Test in progress</Typography>
      <Box
        sx={{
          display: 'flex',
          position: 'relative',
          width: 250,
          height: 250,
          justifyContent: 'center',
          alignItems: 'center',
        }}
      >
        <CircularProgress
          variant="determinate"
          value={percentage}
          size={250}
          sx={{ position: 'absolute', top: 0, left: 0 }}
        />
        <Typography fontWeight="bold" variant="h4">
          {percentage}%
        </Typography>
      </Box>
      <Typography>Sending packets...</Typography>
      <Typography>{`${packetsSent} / ${totalPackets}`}</Typography>
    </Stack>
  );
};
