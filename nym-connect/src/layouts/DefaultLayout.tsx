import React from 'react';
import { Typography } from '@mui/material';
import { AppWindowFrame } from '../components/AppWindowFrame';
import { ConnectionButton } from '../components/ConnectionButton';
import { ConnectionStatusKind } from '../types';
import { NeedHelp } from '../components/NeedHelp';
import { ServiceProviderSelector } from '../components/ServiceProviderSelector';
import { ServiceProvider, Services } from '../types/directory';
import { useClientContext } from '../context/main';

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
    <AppWindowFrame>
      <Typography fontWeight="400" fontSize="12px" textAlign="center" sx={{ opacity: 0.6 }}>
        This is experimental software. <br />
        Do not rely on it for strong anonymity (yet).
      </Typography>
      <Typography fontWeight="700" fontSize="14px" textAlign="center" pt={2}>
        Connect to the
        <br />
        Nym mixnet for privacy.
      </Typography>
      <ServiceProviderSelector services={services} onChange={handleServiceProviderChange} currentSp={currentSp} />
      <ConnectionButton
        status={status}
        disabled={serviceProvider === undefined && currentSp === undefined}
        busy={busy}
        isError={isError}
        onClick={onConnectClick}
      />
      <NeedHelp />
    </AppWindowFrame>
  );
};
