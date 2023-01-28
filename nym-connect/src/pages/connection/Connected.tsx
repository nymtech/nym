import React, { useState } from 'react';
import { Box, Stack } from '@mui/material';
import { DateTime } from 'luxon';
import { IpAddressAndPortModal } from 'src/components/IpAddressAndPortModal';
import { ConnectionTimer } from 'src/components/ConntectionTimer';
import { ConnectionStatus } from 'src/components/ConnectionStatus';
import { ConnectionStatusKind, GatewayPerformance } from 'src/types';
import { ConnectionStatsItem } from 'src/components/ConnectionStats';
import { IpAddressAndPort } from 'src/components/IpAddressAndPort';
import { ServiceProvider } from 'src/types/directory';
import { ExperimentalWarning } from 'src/components/ExperimentalWarning';
import { ConnectionLayout } from 'src/layouts/ConnectionLayout';
import { PowerButton } from 'src/components/PowerButton';

export const Connected: FCWithChildren<{
  status: ConnectionStatusKind;
  gatewayPerformance: GatewayPerformance;
  stats: ConnectionStatsItem[];
  ipAddress: string;
  port: number;
  connectedSince?: DateTime;
  busy?: boolean;
  isError?: boolean;
  onConnectClick: (status: ConnectionStatusKind) => void;
  serviceProvider?: ServiceProvider;
}> = ({
  status,
  gatewayPerformance,
  ipAddress,
  port,
  connectedSince,
  busy,
  isError,
  serviceProvider,
  onConnectClick,
}) => {
  const [showInfoModal, setShowInfoMdal] = useState(true);

  return (
    <>
      <IpAddressAndPortModal
        show={showInfoModal}
        onClose={() => setShowInfoMdal(false)}
        ipAddress={ipAddress}
        port={port}
      />

      <ConnectionLayout
        TopContent={
          <Box>
            <ConnectionStatus
              status={'connected'}
              gatewayPerformance={gatewayPerformance}
              serviceProvider={serviceProvider}
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
            disabled={status === 'connecting' || status === 'disconnecting'}
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
};
