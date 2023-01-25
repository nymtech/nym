import React from 'react';
import { Box } from '@mui/material';
import { DateTime } from 'luxon';
import { IpAddressAndPortModal } from 'src/components/IpAddressAndPortModal';
import { ConnectionTimer } from 'src/components/ConntectionTimer';
import { ConnectionStatus } from 'src/components/ConnectionStatus';
import { ConnectionStatusKind, GatewayPerformance } from 'src/types';
import { ConnectionStatsItem } from 'src/components/ConnectionStats';
import { ConnectionButton } from 'src/components/ConnectionButton';
import { IpAddressAndPort } from 'src/components/IpAddressAndPort';
import { ServiceProvider } from 'src/types/directory';
import { ExperimentalWarning } from 'src/components/ExperimentalWarning';

export const Connected: FCWithChildren<{
  status: ConnectionStatusKind;
  gatewayPerformance: GatewayPerformance;
  stats: ConnectionStatsItem[];
  ipAddress: string;
  port: number;
  connectedSince?: DateTime;
  busy?: boolean;
  showInfoModal: boolean;
  isError?: boolean;
  handleCloseInfoModal: () => void;
  onConnectClick?: (status: ConnectionStatusKind) => void;
  serviceProvider?: ServiceProvider;
}> = ({
  status,
  gatewayPerformance,
  showInfoModal,
  handleCloseInfoModal,
  ipAddress,
  port,
  connectedSince,
  busy,
  isError,
  serviceProvider,
  onConnectClick,
}) => (
  <>
    <IpAddressAndPortModal show={showInfoModal} onClose={handleCloseInfoModal} ipAddress={ipAddress} port={port} />
    <Box pb={1}>
      <ConnectionStatus
        status={'connected'}
        gatewayPerformance={gatewayPerformance}
        serviceProvider={serviceProvider}
      />
    </Box>
    <ConnectionTimer connectedSince={connectedSince} />
    <ConnectionButton status={status} busy={busy} onClick={onConnectClick} isError={isError} />
    <Box sx={{ mb: 2 }}>
      <IpAddressAndPort label="Socks5 address" ipAddress={ipAddress} port={port} />
    </Box>
    <ExperimentalWarning />
  </>
);
