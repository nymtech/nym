import React from 'react';
import { Typography } from '@mui/material';
import { AppWindowFrame } from '../components/AppWindowFrame';
import { ConnectionButton } from '../components/ConnectionButton';
import { ConnectionStatusKind } from '../types';
import { NeedHelp } from '../components/NeedHelp';
import { ServiceProviderSelector } from '../components/ServiceProviderSelector';
import { ServiceProvider, Services } from '../types/directory';

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
  return (
    <AppWindowFrame>
      <Typography fontWeight="700" fontSize="14px" textAlign="center">
        Connect, your privacy will be 100% protected thanks to the Nym Mixnet
      </Typography>
      <Typography fontWeight="700" fontSize="14px" textAlign="center" color="#60D6EF" pt={2}>
        You are not protected now
      </Typography>
      <ServiceProviderSelector services={services} onChange={handleServiceProviderChange} />
      <ConnectionButton
        status={status}
        disabled={serviceProvider === undefined}
        busy={busy}
        isError={isError}
        onClick={onConnectClick}
      />
      <NeedHelp />
    </AppWindowFrame>
  );
};
