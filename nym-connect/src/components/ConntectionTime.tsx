import React, { useEffect } from 'react';
import { Stack, Typography } from '@mui/material';
import { DateTime } from 'luxon';

export const ConnectionTime = ({ connectedSince }: { connectedSince?: DateTime }) => {
  const [duration, setDuration] = React.useState<string>();
  useEffect(() => {
    const intervalId = setInterval(() => {
      if (connectedSince) {
        setDuration(DateTime.now().diff(connectedSince).toFormat('hh:mm:ss'));
      }
    }, 500);
    return () => {
      clearInterval(intervalId);
    };
  }, [connectedSince]);
  return (
    <Stack alignItems="center">
      <Typography variant="caption" sx={{ color: 'grey.600' }}>
        Connection time
      </Typography>
      <Typography letterSpacing="0.15em">{duration || '00:00:00'}</Typography>
    </Stack>
  );
};
