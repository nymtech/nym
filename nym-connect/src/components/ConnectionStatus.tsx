import React from 'react';
import { Box, CircularProgress, Typography } from '@mui/material';
import CheckCircleOutlineIcon from '@mui/icons-material/CheckCircleOutline';
import CircleOutlinedIcon from '@mui/icons-material/CircleOutlined';
import { DateTime } from 'luxon';
import { ConnectionStatusKind } from '../types';
import { ServiceProvider } from '../types/directory';

const FONT_SIZE = '16px';
const FONT_WEIGHT = '600';
const FONT_STYLE = 'normal';

const ConnectionStatusContent: React.FC<{
  status: ConnectionStatusKind;
}> = ({ status }) => {
  switch (status) {
    case ConnectionStatusKind.connected:
      return (
        <>
          <CheckCircleOutlineIcon />
          <Typography fontWeight={FONT_WEIGHT} fontStyle={FONT_STYLE} ml={1}>
            Connected
          </Typography>
        </>
      );
    case ConnectionStatusKind.disconnecting:
      return (
        <>
          <CircularProgress size={FONT_SIZE} color="inherit" />
          <Typography fontWeight={FONT_WEIGHT} fontStyle={FONT_STYLE} ml={1}>
            Disconnecting...
          </Typography>
        </>
      );
    case ConnectionStatusKind.connecting:
      return (
        <>
          <CircularProgress size={FONT_SIZE} color="inherit" />
          <Typography fontWeight={FONT_WEIGHT} fontStyle={FONT_STYLE} ml={1}>
            Connecting...
          </Typography>
        </>
      );
    case ConnectionStatusKind.disconnected:
      return (
        <>
          <CircleOutlinedIcon />
          <Typography fontWeight={FONT_WEIGHT} fontStyle={FONT_STYLE} ml={1}>
            Disconnected
          </Typography>
        </>
      );
    default:
      return null;
  }
};

export const ConnectionStatus: React.FC<{
  status: ConnectionStatusKind;
  connectedSince?: DateTime;
  serviceProvider?: ServiceProvider;
}> = ({ status, connectedSince, serviceProvider }) => {
  const color =
    status === ConnectionStatusKind.connected || status === ConnectionStatusKind.disconnecting ? '#21D072' : '#888';
  const [duration, setDuration] = React.useState<string>();
  React.useEffect(() => {
    const intervalId = setInterval(() => {
      if (connectedSince) {
        setDuration(DateTime.now().diff(connectedSince).toFormat('hh:mm:ss'));
      }
    }, 500);
    return () => {
      clearInterval(intervalId);
    };
  }, [status, connectedSince]);
  return (
    <>
      <Box display="flex" justifyContent="space-between">
        <Box color={color} fontSize={FONT_SIZE} display="flex" alignItems="center">
          <ConnectionStatusContent status={status} />
        </Box>
        <Typography color={color} fontWeight={FONT_WEIGHT} fontStyle={FONT_STYLE}>
          {status === ConnectionStatusKind.connected && duration}
        </Typography>
      </Box>
      <Box>{serviceProvider && <Typography fontSize={12}>{serviceProvider.description}</Typography>}</Box>
    </>
  );
};
