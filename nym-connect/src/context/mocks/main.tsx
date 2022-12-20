import React from 'react';
import { ConnectionStatusKind } from 'src/types';
import { ClientContext, TClientContext } from '../main';

const mockValues: TClientContext = {
  appVersion: 'v1.x.x',
  mode: 'dark',
  connectionStatus: ConnectionStatusKind.disconnected,
  services: [],
  showHelp: false,
  serviceProvider: { id: '1', description: 'Keybase service provider', gateway: 'abc123', address: '123abc' },
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

export const MockProvider: React.FC<{
  children?: React.ReactNode;
  connectionStatus?: ConnectionStatusKind;
}> = ({ connectionStatus = ConnectionStatusKind.disconnected, children }) => (
  <ClientContext.Provider value={{ ...mockValues, connectionStatus }}>{children}</ClientContext.Provider>
);
