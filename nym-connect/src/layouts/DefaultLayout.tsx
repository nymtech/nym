import React from 'react';
import { Typography } from '@mui/material';
import { Box } from '@mui/material';
import { ConnectionStatus } from 'src/components/ConnectionStatus';
import { ConnectionTimer } from 'src/components/ConntectionTimer';
import { InfoModal } from 'src/components/InfoModal';
import { Error } from 'src/types/error';
import { ConnectionButton } from '../components/ConnectionButton';
import { ServiceProviderSelector } from '../components/ServiceProviderSelector';
import { useClientContext } from '../context/main';
import { ConnectionStatusKind } from '../types';
import { ServiceProvider, Services } from '../types/directory';

export const DefaultLayout: React.FC<{
  error?: Error;
  status: ConnectionStatusKind;
  services?: Services;
  busy?: boolean;
  isError?: boolean;
  clearError: () => void;
  onConnectClick?: (status: ConnectionStatusKind) => void;
  onServiceProviderChange?: (serviceProvider: ServiceProvider) => void;
}> = ({ status, error, services, busy, isError, onConnectClick, onServiceProviderChange, clearError }) => {
  const handleServiceProviderChange = (newServiceProvider: ServiceProvider) => {
    onServiceProviderChange?.(newServiceProvider);
  };

  const { serviceProvider: currentSp } = useClientContext();

  return (
    <Box pt={1}>
      {error && <InfoModal show title={error.title} description={error.message} onClose={clearError} />}
      <ConnectionStatus status={ConnectionStatusKind.disconnected} />
      <Box px={2}>
        <Typography fontWeight="400" fontSize="16px" textAlign="center" mb={1}>
          Connect to the Nym mixnet
        </Typography>
        <Typography textAlign="center" fontSize="small" sx={{ color: 'grey.500' }}>
          This is experimental software. Do not rely on it for strong anonymity (yet).
        </Typography>
      </Box>
      <ServiceProviderSelector services={services} onChange={handleServiceProviderChange} currentSp={currentSp} />
      <ConnectionTimer />
      <ConnectionButton
        status={status}
        disabled={currentSp === undefined}
        busy={busy}
        isError={isError}
        onClick={onConnectClick}
      />
    </Box>
  );
};
