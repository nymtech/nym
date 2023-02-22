import React, { useEffect } from 'react';
import { Stack, Typography } from '@mui/material';
import { DateTime } from 'luxon';

export const ConnectionTimer = ({ connectedSince }: { connectedSince?: DateTime }) => {
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
      <Typography variant="caption" sx={{ color: 'grey.600' }} fontSize="14px" fontWeight={400}>
        Connection time
      </Typography>
      <Typography letterSpacing="0.20em" fontSize="20px" fontWeight={400}>
        {duration || '00:00:00'}
      </Typography>
    </Stack>
  );
};
