import React from 'react';
import { forage } from '@tauri-apps/tauri-forage';
import * as Sentry from '@sentry/react';
import { DateTime } from 'luxon';
import { useClientContext } from 'src/context/main';
import { useTauriEvents } from 'src/utils';
import { Connected } from './Connected';
import { Disconnected } from './Disconnected';

export const ConnectionPage = () => {
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

  const handleConnectClick = async () => {
    const currentStatus = context.connectionStatus;
    if (currentStatus === 'connected' || currentStatus === 'disconnected') {
      setBusy(true);
      // eslint-disable-next-line default-case
      switch (currentStatus) {
        case 'disconnected':
          Sentry.captureMessage('start connect', 'info');
          await context.setSerivceProvider();
          await context.startConnecting();
          context.setConnectedSince(DateTime.now());
          context.setShowInfoModal(true);
          break;
        case 'connected':
          Sentry.captureMessage('start disconnect', 'info');
          await context.startDisconnecting();
          context.setConnectedSince(undefined);
          break;
      }
      setBusy(false);
    }
  };

  const closeInfoModal = () => context.setShowInfoModal(false);

  if (context.connectionStatus === 'connected')
    return (
      <Connected
        error={context.error}
        clearError={context.clearError}
        status={context.connectionStatus}
        showInfoModal={context.showInfoModal}
        busy={busy}
        onConnectClick={handleConnectClick}
        ipAddress="127.0.0.1"
        port={1080}
        gatewayPerformance={context.gatewayPerformance}
        connectedSince={context.connectedSince}
        serviceProvider={context.selectedProvider}
        closeInfoModal={closeInfoModal}
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
      serviceProvider={context.selectedProvider}
    />
  );
};
