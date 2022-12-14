import React from 'react';
import { ConnectionStatusKind } from 'src/types';
import { ClientContext, TClientContext } from '../main';

const mockValues: TClientContext = {
  mode: 'dark',
  appVersion: '1.1.1',
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

export const MockProvider = ({ children }: { children: React.ReactNode }) => {
  return <ClientContext.Provider value={mockValues}>{children}</ClientContext.Provider>;
};
