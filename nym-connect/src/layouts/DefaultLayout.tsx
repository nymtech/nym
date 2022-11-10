import React, { useState } from 'react';
import { Button, Typography } from '@mui/material';
import { ConnectionButton } from '../components/ConnectionButton';
import { ConnectionStatusKind } from '../types';
import { NeedHelp } from '../components/NeedHelp';
import { ServiceProviderSelector } from '../components/ServiceProviderSelector';
import { ServiceProvider, Services } from '../types/directory';
import { useClientContext } from '../context/main';
import { ConnectionStatus } from 'src/components/ConnectionStatus';
import { Box } from '@mui/system';
import { ConnectionTime } from 'src/components/ConntectionTime';
import { InfoModal } from 'src/components/InfoModal';
import { Error } from 'src/types/error';

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
  const [serviceProvider, setServiceProvider] = React.useState<ServiceProvider | undefined>();
  const handleServiceProviderChange = (newServiceProvider: ServiceProvider) => {
    setServiceProvider(newServiceProvider);
    onServiceProviderChange?.(newServiceProvider);
  };
  const { serviceProvider: currentSp } = useClientContext();

  return (
    <Box pt={1}>
      {error && <InfoModal show={true} title={error.error} description={error?.description} onClose={clearError} />}
      <ConnectionStatus status={ConnectionStatusKind.disconnected} />
      <Typography fontWeight="700" fontSize="16px" textAlign="center" pt={2}>
        Connect to the Nym <br /> mixnet for privacy.
      </Typography>
      <ServiceProviderSelector services={services} onChange={handleServiceProviderChange} currentSp={currentSp} />
      <ConnectionTime />
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
