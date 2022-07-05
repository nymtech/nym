import React from 'react';
import { ConnectionStatusKind } from './types';
import { useClientContext } from './context/main';
import { DefaultLayout } from './layouts/DefaultLayout';
import { ConnectedLayout } from './layouts/ConnectedLayout';

export const App: React.FC = () => {
  const context = useClientContext();
  const [busy, setBusy] = React.useState<boolean>();

  const handleConnectClick = React.useCallback(async () => {
    const oldStatus = context.connectionStatus;
    if (oldStatus === ConnectionStatusKind.connected || oldStatus === ConnectionStatusKind.disconnected) {
      setBusy(true);

      // eslint-disable-next-line default-case
      switch (oldStatus) {
        case ConnectionStatusKind.disconnected:
          await context.startConnecting();
          break;
        case ConnectionStatusKind.connected:
          await context.startDisconnecting();
          break;
      }
      setBusy(false);
    }
  }, [context.connectionStatus]);

  if (
    context.connectionStatus === ConnectionStatusKind.disconnected ||
    context.connectionStatus === ConnectionStatusKind.connecting
  ) {
    return (
      <DefaultLayout
        status={context.connectionStatus}
        busy={busy}
        onConnectClick={handleConnectClick}
        services={context.services}
        onServiceProviderChange={context.setServiceProvider}
      />
    );
  }

  return (
    <ConnectedLayout
      status={context.connectionStatus}
      busy={busy}
      onConnectClick={handleConnectClick}
      ipAddress="127.0.0.1"
      port={1080}
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
};
