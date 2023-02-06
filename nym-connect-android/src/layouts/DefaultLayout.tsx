import React from 'react';
import { Box, Stack, Typography } from '@mui/material';
import { ConnectionStatus } from 'src/components/ConnectionStatus';
import { ConnectionTimer } from 'src/components/ConntectionTimer';
import { InfoModal } from 'src/components/InfoModal';
import { Error } from 'src/types/error';
import { ConnectionButton } from '../components/ConnectionButton';
import { ServiceSelector } from '../components/ServiceSelector';
import { useClientContext } from '../context/main';
import { ConnectionStatusKind } from '../types';
import { Services } from '../types/directory';

export const DefaultLayout: FCWithChildren<{
  error?: Error;
  status: ConnectionStatusKind;
  services?: Services;
  busy?: boolean;
  isError?: boolean;
  clearError: () => void;
  onConnectClick?: (status: ConnectionStatusKind) => void;
}> = ({ status, error, services, busy, isError, onConnectClick, clearError }) => {
  const context = useClientContext();

  return (
    <Box pt={1}>
      {error && <InfoModal show title={error.title} description={error.message} onClose={clearError} />}
      <ConnectionStatus status={ConnectionStatusKind.disconnected} />
      <Box px={2}>
        <Typography fontWeight="400" fontSize="16px" textAlign="center" pt={2}>
          Connect to the Nym <br /> mixnet for privacy.
        </Typography>
        <Typography textAlign="center" fontSize="small" sx={{ color: 'grey.500' }}>
          This is experimental software. Do not rely on it for strong anonymity (yet).
        </Typography>
      </Box>
      <ServiceSelector services={services} onChange={context.setServiceProvider} currentSp={context.serviceProvider} />
      <ConnectionTimer />
      <Stack mt={3} direction="row" justifyContent="center" alignItems="center">
        <ConnectionButton
          status={status}
          disabled={context.serviceProvider === undefined}
          busy={busy}
          isError={isError}
          onClick={onConnectClick}
        />
      </Stack>
    </Box>
  );
};
