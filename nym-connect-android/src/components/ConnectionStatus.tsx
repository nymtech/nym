import React from 'react';
import { Box, CircularProgress, Tooltip, Typography } from '@mui/material';
import { DateTime } from 'luxon';
import { ErrorOutline, InfoOutlined } from '@mui/icons-material';
import { ConnectionStatusKind, GatewayPerformance } from '../types';
import { ServiceProvider } from '../types/directory';
import { GatwayWarningInfo, ServiceProviderInfo } from './TooltipInfo';

const FONT_SIZE = '14px';
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
          <Typography
            fontWeight={400}
            fontStyle={FONT_STYLE}
            fontSize="14px"
            textAlign="center"
            sx={{ textDecorationLine: 'underline' }}
          >
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
            <Typography
              fontWeight={400}
              fontStyle={FONT_STYLE}
              fontSize="14px"
              textAlign="center"
              sx={{ textDecorationLine: 'underline' }}
            >
              Connected to Nym Mixnet
            </Typography>
          </Box>
        </Tooltip>
      );
    case 'disconnected':
      return (
        <Typography fontWeight={400} fontStyle={FONT_STYLE} textAlign="center" fontSize="20px">
          Connect to the mixnet
        </Typography>
      );
    case 'disconnecting':
      return (
        <Box display="flex" alignItems="center" justifyContent="center">
          <CircularProgress size={FONT_SIZE} color="inherit" />
          <Typography fontWeight={400} fontStyle={FONT_STYLE} fontSize="20px">
            Disconnecting...
          </Typography>
        </Box>
      );
    case 'connecting':
      return (
        <Box display="flex" alignItems="center" justifyContent="center">
          <CircularProgress size={FONT_SIZE} color="inherit" />
          <Typography fontWeight={400} fontStyle={FONT_STYLE} ml={1} fontSize="20px">
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
    <Box color={color} sx={{ mb: 3 }}>
      <ConnectionStatusContent
        status={status}
        serviceProvider={serviceProvider}
        gatewayError={gatewayPerformance !== 'Good'}
      />
    </Box>
  );
};
