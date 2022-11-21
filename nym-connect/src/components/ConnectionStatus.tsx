import React from 'react';
import { Box, CircularProgress, Typography } from '@mui/material';
import { DateTime } from 'luxon';
import { ConnectionStatusKind } from '../types';
import { ServiceProvider } from '../types/directory';

const FONT_SIZE = '10px';
const FONT_WEIGHT = '600';
const FONT_STYLE = 'normal';

const ConnectionStatusContent: React.FC<{
  status: ConnectionStatusKind;
}> = ({ status }) => {
  switch (status) {
    case ConnectionStatusKind.connected:
      return (
        <Typography fontWeight={FONT_WEIGHT} fontStyle={FONT_STYLE} textAlign="center">
          Connected
        </Typography>
      );
    case ConnectionStatusKind.disconnecting:
      return (
        <Box display="flex" alignItems="center" justifyContent="center">
          <CircularProgress size={FONT_SIZE} color="inherit" />
          <Typography fontWeight={FONT_WEIGHT} fontStyle={FONT_STYLE} ml={1}>
            Disconnecting...
          </Typography>
        </Box>
      );
    case ConnectionStatusKind.connecting:
      return (
        <Box display="flex" alignItems="center" justifyContent="center">
          <CircularProgress size={FONT_SIZE} color="inherit" />
          <Typography fontWeight={FONT_WEIGHT} fontStyle={FONT_STYLE} ml={1}>
            Connecting...
          </Typography>
        </Box>
      );
    case ConnectionStatusKind.disconnected:
      return (
        <Typography
          fontWeight={FONT_WEIGHT}
          fontStyle={FONT_STYLE}
          ml={1}
          textTransform="uppercase"
          textAlign="center"
          fontSize="10px"
          sx={{ wordSpacing: 3, letterSpacing: 2 }}
        >
          You are not protected
        </Typography>
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
    status === ConnectionStatusKind.connected || status === ConnectionStatusKind.disconnecting
      ? '#21D072'
      : 'warning.main';

  return (
    <>
      <Box color={color} fontSize={FONT_SIZE} sx={{ mb: 1 }}>
        <ConnectionStatusContent status={status} />
      </Box>
      <Box>
        {serviceProvider && (
          <Typography fontSize={12} textAlign="center">
            To {serviceProvider.description}
          </Typography>
        )}
      </Box>
    </>
  );
};
