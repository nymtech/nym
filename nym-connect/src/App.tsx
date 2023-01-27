import React, { useEffect } from 'react';
import { DateTime } from 'luxon';
import { forage } from '@tauri-apps/tauri-forage';
import { useClientContext } from './context/main';
import { useTauriEvents } from './utils';
import { AppRoutes } from './routes';
import { Connected } from './pages/connection/Connected';

export const App: FCWithChildren = () => {
  const context = useClientContext();
  const [busy, setBusy] = React.useState<boolean>();

  useTauriEvents('help://clear-storage', (_event) => {
    console.log('About to clear local storage...');
    // clear local storage
    try {
      forage.clear()();
      console.log('Local storage cleared');
    } catch (e) {
      console.error('Failed to clear local storage', e);
    }
  });

  const handleConnectClick = React.useCallback(async () => {
    const currentStatus = context.connectionStatus;
    if (currentStatus === 'connected' || currentStatus === 'disconnected') {
      setBusy(true);

      // eslint-disable-next-line default-case
      switch (currentStatus) {
        case 'disconnected':
          await context.startConnecting();
          context.setConnectedSince(DateTime.now());
          break;
        case 'connected':
          await context.startDisconnecting();
          context.setConnectedSince(undefined);
          break;
      }
      setBusy(false);
    }
  }, [context.connectionStatus]);

  if (context.connectionStatus === 'disconnected' || context.connectionStatus === 'connecting') {
    return <AppRoutes />;
  }

  return (
    <Connected
      status={context.connectionStatus}
      busy={busy}
      onConnectClick={handleConnectClick}
      ipAddress="127.0.0.1"
      port={1080}
      gatewayPerformance={context.gatewayPerformance}
      connectedSince={context.connectedSince}
      serviceProvider={context.selectedProvider}
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
  );
};
