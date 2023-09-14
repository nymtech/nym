import React, { useMemo } from 'react';
import { ConnectionStatusKind } from 'src/types';
import { ClientContext, TClientContext } from '../main';

const mockValues: TClientContext = {
  appVersion: 'v1.x.x',
  mode: 'dark',
  connectionStatus: ConnectionStatusKind.disconnected,
  selectedProvider: { id: '1', description: 'Keybase service provider', address: '123abc' },
  gatewayPerformance: 'Good',
  showInfoModal: false,
  userDefinedGateway: { isActive: false, address: '' },
  userDefinedSPAddress: { isActive: false, address: '' },
  userData: { monitoring: false, privacy_level: 'High' },
  showFeedbackNote: false,
  setShowInfoModal: () => {},
  setMode: () => {},
  clearError: () => {},
  setConnectedSince: () => {},
  setConnectionStats: () => {},
  setConnectionStatus: () => {},
  startConnecting: async () => {},
  startDisconnecting: async () => {},
  setServiceProvider: async () => {},
  setGateway: async () => {},
  setUserDefinedGateway: () => {},
  setUserDefinedSPAddress: () => {},
  setMonitoring: async () => {},
  setPrivacyLevel: async () => {},
  setShowFeedbackNote: () => {},
};

export const MockProvider: FCWithChildren<{
  children?: React.ReactNode;
  connectionStatus?: ConnectionStatusKind;
}> = ({ connectionStatus = ConnectionStatusKind.disconnected, children }) => {
  const value = useMemo(() => ({ ...mockValues, connectionStatus }), [connectionStatus]);
  return <ClientContext.Provider value={value}>{children}</ClientContext.Provider>;
};
