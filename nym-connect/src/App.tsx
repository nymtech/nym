import React, { useEffect } from 'react';
import { DateTime } from 'luxon';
import { ConnectionStatusKind } from './types';
import { useClientContext } from './context/main';
import { DefaultLayout } from './layouts/DefaultLayout';
import { ConnectedLayout } from './layouts/ConnectedLayout';
import { HelpGuideLayout } from './layouts/HelpGuideLayout';

export const App: React.FC = () => {
  const context = useClientContext();
  const [busy, setBusy] = React.useState<boolean>();
  const [showInfoModal, setShowInfoModal] = React.useState(false);

  const handleConnectClick = React.useCallback(async () => {
    const currentStatus = context.connectionStatus;
    if (currentStatus === ConnectionStatusKind.connected || currentStatus === ConnectionStatusKind.disconnected) {
      setBusy(true);

      // eslint-disable-next-line default-case
      switch (currentStatus) {
        case ConnectionStatusKind.disconnected:
          await context.startConnecting();
          context.setConnectedSince(DateTime.now());
          break;
        case ConnectionStatusKind.connected:
          await context.startDisconnecting();
          context.setConnectedSince(undefined);
          break;
      }
      setBusy(false);
    }
  }, [context.connectionStatus]);

  useEffect(() => {
    if (context.connectionStatus === ConnectionStatusKind.connected) setShowInfoModal(true);
  }, [context.connectionStatus]);

  if (context.showHelp) return <HelpGuideLayout />;

  if (
    context.connectionStatus === ConnectionStatusKind.disconnected ||
    context.connectionStatus === ConnectionStatusKind.connecting
  ) {
    return (
      <DefaultLayout
        error={context.error}
        clearError={context.clearError}
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
      showInfoModal={showInfoModal}
      handleCloseInfoModal={() => setShowInfoModal(false)}
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
