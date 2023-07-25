import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { DateTime } from 'luxon';
import { Disconnected } from 'src/pages/connection/Disconnected';
import { Connected } from 'src/pages/connection/Connected';
import { ConnectionStatusKind } from 'src/types';
import { AppWindowFrame } from '../components/AppWindowFrame';
import { useClientContext } from '../context/main';
import { Services } from '../types/directory';

export default {
  title: 'App/Flow',
  component: AppWindowFrame,
} as ComponentMeta<typeof AppWindowFrame>;

const width = 240;
const height = 575;

export const Mock: ComponentStory<typeof AppWindowFrame> = () => {
  const context = useClientContext();
  const [busy, setBusy] = React.useState<boolean>();
  const services: Services = [
    {
      id: 'keybase',
      description: 'Keybase',
      items: [
        {
          id: 'nym-keybase',
          description: 'Nym Keybase Service Provider',
          address: '1234.5678',
        },
      ],
    },
  ];
  const handleConnectClick = React.useCallback(() => {
    const oldStatus = context.connectionStatus;
    if (oldStatus === 'connected' || oldStatus === 'disconnected') {
      setBusy(true);

      // eslint-disable-next-line default-case
      switch (oldStatus) {
        case 'disconnected':
          context.setConnectionStatus(ConnectionStatusKind.connecting);
          break;
        case 'connected':
          context.setConnectionStatus(ConnectionStatusKind.disconnecting);
          break;
      }

      setTimeout(() => {
        // eslint-disable-next-line default-case
        switch (oldStatus) {
          case 'disconnected':
            context.setConnectedSince(DateTime.now());
            context.setConnectionStatus(ConnectionStatusKind.connected);
            break;
          case 'connected':
            context.setConnectionStatus(ConnectionStatusKind.disconnected);
            break;
        }
        setBusy(false);
      }, 5000);
    }
  }, [context.connectionStatus]);

  if (
    context.connectionStatus === ConnectionStatusKind.disconnected ||
    context.connectionStatus === ConnectionStatusKind.connecting
  ) {
    return (
      <Box width={width} height={height}>
        <AppWindowFrame>
          <Disconnected
            status={context.connectionStatus}
            busy={busy}
            onConnectClick={handleConnectClick}
            services={services}
            clearError={() => {}}
          />
        </AppWindowFrame>
      </Box>
    );
  }

  return (
    <AppWindowFrame>
      <Connected
        clearError={() => {}}
        gatewayPerformance="Good"
        showInfoModal={false}
        closeInfoModal={() => undefined}
        status={context.connectionStatus}
        busy={busy}
        onConnectClick={handleConnectClick}
        ipAddress="127.0.0.1"
        port={1080}
        connectedSince={context.connectedSince}
        serviceProvider={services[0].items[0]}
        stats={[
          {
            label: 'in:',
            totalBytes: 1024,
            rateBytesPerSecond: 1024 * 1024 * 1024 + 10,
          },
          {
            label: 'out:',
            totalBytes: 1024 * 1024 * 1024 * 1024 * 20,
            rateBytesPerSecond: 1024 * 1024 + 10,
          },
        ]}
      />
    </AppWindowFrame>
  );
};
