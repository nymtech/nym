import React from 'react';
import { Box, Divider } from '@mui/material';
import { DateTime } from 'luxon';
import { IpAddressAndPortModal } from 'src/components/IpAddressAndPortModal';
import { ConnectionTimer } from 'src/components/ConntectionTimer';
import { ConnectionStatus } from '../components/ConnectionStatus';
import { ConnectionStatusKind, GatewayPerformance } from '../types';
import { ConnectionStatsItem } from '../components/ConnectionStats';
import { ConnectionButton } from '../components/ConnectionButton';
import { IpAddressAndPort } from '../components/IpAddressAndPort';
import { ServiceProvider } from '../types/directory';
import { TestAndEarnButtonArea } from '../components/Growth/TestAndEarnButtonArea';

export const ConnectedLayout: FCWithChildren<{
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
        status={ConnectionStatusKind.connected}
        serviceProvider={serviceProvider}
        gatewayPerformance={gatewayPerformance}
      />
    </Box>
    <Divider sx={{ my: 2 }} />
    <Box sx={{ mb: 3 }}>
      <IpAddressAndPort label="Socks5 address" ipAddress={ipAddress} port={port} />
    </Box>
    {/* <ConnectionStats stats={stats} /> */}
    <ConnectionTimer connectedSince={connectedSince} />
    <ConnectionButton status={status} busy={busy} onClick={onConnectClick} isError={isError} />
  </>
);
