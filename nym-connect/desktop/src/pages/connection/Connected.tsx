import React from 'react';
import { Box, Stack } from '@mui/material';
import { DateTime } from 'luxon';
import { IpAddressAndPortModal } from 'src/components/IpAddressAndPortModal';
import { ConnectionTimer } from 'src/components/ConntectionTimer';
import { ConnectionStatus } from 'src/components/ConnectionStatus';
import { ConnectionStatusKind, GatewayPerformance } from 'src/types';
import { ConnectionStatsItem } from 'src/components/ConnectionStats';
import { IpAddressAndPort } from 'src/components/IpAddressAndPort';
import { ServiceProvider, Gateway } from 'src/types/directory';
import { ExperimentalWarning } from 'src/components/ExperimentalWarning';
import { ConnectionLayout } from 'src/layouts/ConnectionLayout';
import { PowerButton } from 'src/components/PowerButton/PowerButton';
import { Error } from 'src/types/error';
import { InfoModal } from 'src/components/InfoModal';

export const Connected: FCWithChildren<{
  error?: Error;
  status: ConnectionStatusKind;
  showInfoModal: boolean;
  gatewayPerformance: GatewayPerformance;
  stats: ConnectionStatsItem[];
  ipAddress: string;
  port: number;
  connectedSince?: DateTime;
  busy?: boolean;
  isError?: boolean;
  serviceProvider?: ServiceProvider;
  gateway?: Gateway;
  clearError: () => void;
  onConnectClick: (status: ConnectionStatusKind) => void;
  closeInfoModal: () => void;
}> = ({
  error,
  status,
  showInfoModal,
  gatewayPerformance,
  ipAddress,
  port,
  connectedSince,
  busy,
  isError,
  serviceProvider,
  gateway,
  clearError,
  onConnectClick,
  closeInfoModal,
}) => (
  <>
    {error && <InfoModal show title={error.title} description={error.message} onClose={clearError} />}
    <IpAddressAndPortModal show={showInfoModal} onClose={closeInfoModal} ipAddress={ipAddress} port={port} />
    <ConnectionLayout
      TopContent={
        <Box>
          <ConnectionStatus
            status={ConnectionStatusKind.connected}
            gatewayPerformance={gatewayPerformance}
            serviceProvider={serviceProvider}
            gateway={gateway}
          />
          <ConnectionTimer connectedSince={connectedSince} />
        </Box>
      }
      ConnectButton={
        <PowerButton
          status={status}
          busy={busy}
          onClick={onConnectClick}
          isError={isError}
          disabled={status === 'disconnecting'}
        />
      }
      BottomContent={
        <Stack justifyContent="space-between">
          <Box sx={{ mb: 2 }}>
            <IpAddressAndPort label="Socks5 address" ipAddress={ipAddress} port={port} />
          </Box>
          <ExperimentalWarning />
        </Stack>
      }
    />
  </>
);
