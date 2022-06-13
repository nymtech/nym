import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { DateTime } from 'luxon';
import { AppWindowFrame } from '../components/AppWindowFrame';
import { useClientContext } from '../context/main';
import { ConnectionStatusKind } from '../types';
import { DefaultLayout } from '../layouts/DefaultLayout';
import { ConnectedLayout } from '../layouts/ConnectedLayout';

export default {
  title: 'App/Flow',
  component: AppWindowFrame,
} as ComponentMeta<typeof AppWindowFrame>;

export const Mock: ComponentStory<typeof AppWindowFrame> = () => {
  const context = useClientContext();
  const [busy, setBusy] = React.useState<boolean>();
  const handleConnectClick = React.useCallback(() => {
    const oldStatus = context.connectionStatus;
    if (oldStatus === ConnectionStatusKind.connected || oldStatus === ConnectionStatusKind.disconnected) {
      setBusy(true);

      // eslint-disable-next-line default-case
      switch (oldStatus) {
        case ConnectionStatusKind.disconnected:
          context.setConnectionStatus(ConnectionStatusKind.connecting);
          break;
        case ConnectionStatusKind.connected:
          context.setConnectionStatus(ConnectionStatusKind.disconnecting);
          break;
      }

      setTimeout(() => {
        // eslint-disable-next-line default-case
        switch (oldStatus) {
          case ConnectionStatusKind.disconnected:
            context.setConnectedSince(DateTime.now());
            context.setConnectionStatus(ConnectionStatusKind.connected);
            break;
          case ConnectionStatusKind.connected:
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
      <Box p={4} sx={{ background: 'white' }}>
        <DefaultLayout status={context.connectionStatus} busy={busy} onConnectClick={handleConnectClick} />
      </Box>
    );
  }

  return (
    <Box p={4} sx={{ background: 'white' }}>
      <ConnectedLayout
        status={context.connectionStatus}
        busy={busy}
        onConnectClick={handleConnectClick}
        ipAddress="127.0.0.1"
        port={1080}
        connectedSince={context.connectedSince}
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
    </Box>
  );
};
