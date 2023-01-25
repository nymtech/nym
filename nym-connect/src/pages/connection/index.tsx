import { forage } from '@tauri-apps/tauri-forage';
import { DateTime } from 'luxon';
import React, { useEffect, useState } from 'react';
import { useClientContext } from 'src/context/main';
import { useTauriEvents } from 'src/utils';
import { Connected } from './Connected';
import { Disconnected } from './Disconnected';

export const ConnectionPage = () => {
  const context = useClientContext();
  const [busy, setBusy] = React.useState<boolean>();
  const [showInfoModal, setShowInfoModal] = useState(false);

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
    1;
  }, [context.connectionStatus]);

  if (context.connectionStatus === 'connected')
    return (
      <Connected
        handleCloseInfoModal={() => setShowInfoModal(false)}
        status={context.connectionStatus}
        showInfoModal={showInfoModal}
        busy={busy}
        onConnectClick={handleConnectClick}
        ipAddress="127.0.0.1"
        port={1080}
        gatewayPerformance={context.gatewayPerformance}
        connectedSince={context.connectedSince}
        serviceProvider={context.serviceProvider}
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

  return (
    <Disconnected
      busy={busy}
      error={context.error}
      onConnectClick={handleConnectClick}
      clearError={context.clearError}
      status={context.connectionStatus}
      serviceProvider={context.serviceProvider}
    />
  );
};
