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

export const DefaultLayout: React.FC<{
  status: ConnectionStatusKind;
  services?: Services;
  busy?: boolean;
  isError?: boolean;
  onConnectClick?: (status: ConnectionStatusKind) => void;
  onServiceProviderChange?: (serviceProvider: ServiceProvider) => void;
}> = ({ status, services, busy, isError, onConnectClick, onServiceProviderChange }) => {
  const [serviceProvider, setServiceProvider] = React.useState<ServiceProvider | undefined>();
  const handleServiceProviderChange = (newServiceProvider: ServiceProvider) => {
    setServiceProvider(newServiceProvider);
    onServiceProviderChange?.(newServiceProvider);
  };
  const { serviceProvider: currentSp } = useClientContext();

  return (
    <Box pt={1}>
      <ConnectionStatus status={ConnectionStatusKind.disconnected} />
      <Typography fontWeight="700" fontSize="16px" textAlign="center" pt={2}>
        Connect to the Nym <br /> mixnet for privacy.
      </Typography>
      <ServiceProviderSelector services={services} onChange={handleServiceProviderChange} currentSp={currentSp} />
      <ConnectionTime />
      <ConnectionButton
        status={status}
        disabled={serviceProvider === undefined && currentSp === undefined}
        busy={busy}
        isError={isError}
        onClick={onConnectClick}
      />
    </Box>
  );
};
