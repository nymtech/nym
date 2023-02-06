import React from 'react';
import { Box, CircularProgress, Tooltip, Typography } from '@mui/material';
import { DateTime } from 'luxon';
import { ConnectionStatusKind, GatewayPerformance } from '../types';
import { ServiceProvider } from '../types/directory';
import { GatwayWarningInfo, ServiceProviderInfo } from './TooltipInfo';
import { ErrorOutline, InfoOutlined } from '@mui/icons-material';

const FONT_SIZE = '14px';
const FONT_WEIGHT = '600';
const FONT_STYLE = 'normal';

const ConnectionStatusContent: FCWithChildren<{
  status: ConnectionStatusKind;
  serviceProvider?: ServiceProvider;
  gatewayError: boolean;
}> = ({ status, serviceProvider, gatewayError }) => {
  if (gatewayError) {
    return (
      <Tooltip title={serviceProvider ? <GatwayWarningInfo /> : undefined}>
        <Box
          display="flex"
          alignItems="center"
          gap={0.5}
          justifyContent="center"
          sx={{ cursor: 'pointer' }}
          color="warning.main"
        >
          <ErrorOutline sx={{ fontSize: 14 }} />
          <Typography fontWeight={FONT_WEIGHT} fontStyle={FONT_STYLE} fontSize={FONT_SIZE} textAlign="center">
            Gateway has issues
          </Typography>
        </Box>
      </Tooltip>
    );
  }
  switch (status) {
    case 'connected':
      return (
        <Tooltip title={serviceProvider ? <ServiceProviderInfo serviceProvider={serviceProvider} /> : undefined}>
          <Box display="flex" alignItems="center" gap={0.5} justifyContent="center" sx={{ cursor: 'pointer' }}>
            <InfoOutlined sx={{ fontSize: 14 }} />
            <Typography fontWeight={FONT_WEIGHT} fontStyle={FONT_STYLE} fontSize={FONT_SIZE} textAlign="center">
              Connected to Nym Mixnet
            </Typography>
          </Box>
        </Tooltip>
      );
    case 'disconnected':
      return (
        <Typography fontWeight={FONT_WEIGHT} fontStyle={FONT_STYLE} textAlign="center" fontSize={FONT_SIZE}>
          Connect to the mixnet
        </Typography>
      );
    case 'disconnecting':
      return (
        <Box display="flex" alignItems="center" justifyContent="center">
          <CircularProgress size={FONT_SIZE} color="inherit" />
          <Typography fontWeight={FONT_WEIGHT} fontStyle={FONT_STYLE}>
            Disconnecting...
          </Typography>
        </Box>
      );
    case 'connecting':
      return (
        <Box display="flex" alignItems="center" justifyContent="center">
          <CircularProgress size={FONT_SIZE} color="inherit" />
          <Typography fontWeight={FONT_WEIGHT} fontStyle={FONT_STYLE} ml={1}>
            Connecting...
          </Typography>
        </Box>
      );

    default:
      return null;
  }
};

export const ConnectionStatus: FCWithChildren<{
  status: ConnectionStatusKind;
  gatewayPerformance?: GatewayPerformance;
  connectedSince?: DateTime;
  serviceProvider?: ServiceProvider;
}> = ({ status, serviceProvider, gatewayPerformance }) => {
  const color = status === 'connected' || status === 'disconnecting' ? '#21D072' : 'white';

  return (
    <>
      <Box color={color} sx={{ mb: 2 }}>
        <ConnectionStatusContent
          status={status}
          serviceProvider={serviceProvider}
          gatewayError={gatewayPerformance !== 'Good'}
        />
      </Box>
    </>
  );
};
