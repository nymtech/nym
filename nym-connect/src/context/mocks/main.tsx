import React, { useMemo } from 'react';
import { ConnectionStatusKind } from 'src/types';
import { ClientContext, TClientContext } from '../main';

const mockValues: TClientContext = {
  appVersion: 'v1.x.x',
  mode: 'dark',
  connectionStatus: ConnectionStatusKind.disconnected,
  services: [],
  showHelp: false,
  serviceProvider: { id: '1', description: 'Keybase service provider', gateway: 'abc123', address: '123abc' },
  gatewayPerformance: 'Good',
  setMode: () => {},
  clearError: () => {},
  handleShowHelp: () => {},
  setConnectedSince: () => {},
  setConnectionStats: () => {},
  setConnectionStatus: () => {},
  setServiceProvider: () => {},
  startConnecting: async () => {},
  startDisconnecting: async () => {},
};

export const MockProvider: FCWithChildren<{
  children?: React.ReactNode;
  connectionStatus?: ConnectionStatusKind;
}> = ({ connectionStatus = ConnectionStatusKind.disconnected, children }) => {
  const value = useMemo(() => ({ ...mockValues, connectionStatus }), [connectionStatus]);
  return <ClientContext.Provider value={value}>{children}</ClientContext.Provider>;
};
