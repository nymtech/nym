import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { DateTime } from 'luxon';
import { invoke } from '@tauri-apps/api';
import { Error } from 'src/types/error';
import { getVersion } from '@tauri-apps/api/app';
import { useEvents } from 'src/hooks/events';
import { ConnectionStatusKind, GatewayPerformance } from '../types';
import { ConnectionStatsItem } from '../components/ConnectionStats';
import { ServiceProvider } from '../types/directory';

type ModeType = 'light' | 'dark';

export type TClientContext = {
  mode: ModeType;
  appVersion?: string;
  connectionStatus: ConnectionStatusKind;
  connectionStats?: ConnectionStatsItem[];
  connectedSince?: DateTime;
  error?: Error;
  gatewayPerformance: GatewayPerformance;
  selectedProvider?: ServiceProvider;
  showInfoModal: boolean;
  setMode: (mode: ModeType) => void;
  clearError: () => void;
  setConnectionStatus: (connectionStatus: ConnectionStatusKind) => void;
  setConnectionStats: (connectionStats: ConnectionStatsItem[] | undefined) => void;
  setConnectedSince: (connectedSince: DateTime | undefined) => void;
  setShowInfoModal: (show: boolean) => void;
  setRandomSerivceProvider: () => void;
  startConnecting: () => Promise<void>;
  startDisconnecting: () => Promise<void>;
};

export const ClientContext = createContext({} as TClientContext);

export const ClientContextProvider: FCWithChildren = ({ children }) => {
  const [mode, setMode] = useState<ModeType>('dark');
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatusKind>(ConnectionStatusKind.connected);
  const [connectionStats, setConnectionStats] = useState<ConnectionStatsItem[]>();
  const [connectedSince, setConnectedSince] = useState<DateTime>();
  const [selectedProvider, setSelectedProvider] = React.useState<ServiceProvider>();
  const [serviceProviders, setServiceProviders] = React.useState<ServiceProvider[]>();
  const [error, setError] = useState<Error>();
  const [appVersion, setAppVersion] = useState<string>();
  const [gatewayPerformance, setGatewayPerformance] = useState<GatewayPerformance>('Good');
  const [showInfoModal, setShowInfoModal] = useState(false);

  const getAppVersion = async () => {
    const version = await getVersion();
    return version;
  };

  const initialiseApp = async () => {
    const services = await invoke('get_services');
    const AppVersion = await getAppVersion();
    console.log(services);

    setAppVersion(AppVersion);
    setServiceProviders(services as ServiceProvider[]);
  };

  useEvents({
    onError: (e) => setError(e),
    onGatewayPerformanceChange: (performance) => setGatewayPerformance(performance),
    onStatusChange: (status) => setConnectionStatus(status),
  });

  useEffect(() => {
    initialiseApp();
  }, []);

  useEffect(() => {
    // when mounting, load the connection state (needed for the Growth window, that checks the connection state)
    (async () => {
      const currentStatus: ConnectionStatusKind = await invoke('get_connection_status');
      setConnectionStatus(currentStatus);
    })();
  }, []);

  const startConnecting = useCallback(async () => {
    try {
      await invoke('start_connecting');
    } catch (e) {
      setError({ title: 'Could not connect', message: e as string });
      console.log(e);
    }
  }, []);

  const startDisconnecting = useCallback(async () => {
    try {
      await invoke('start_disconnecting');
    } catch (e) {
      console.log(e);
    }
  }, []);

  const setServiceProvider = async (newServiceProvider?: ServiceProvider) => {
    if (newServiceProvider) {
      await invoke('set_gateway', { gateway: newServiceProvider.gateway });
      await invoke('set_service_provider', { serviceProvider: newServiceProvider.address });
    }
  };

  const getRandomSPFromList = (services: ServiceProvider[]) => {
    const randomSelection = services[Math.floor(Math.random() * services.length)];
    return randomSelection;
  };

  const setRandomSerivceProvider = async () => {
    if (serviceProviders) {
      const randomServiceProvider = getRandomSPFromList(serviceProviders);
      await setServiceProvider(randomServiceProvider);
      setSelectedProvider(randomServiceProvider);
    }
  };

  const clearError = () => setError(undefined);

  const contextValue = useMemo(
    () => ({
      mode,
      appVersion,
      setMode,
      error,
      clearError,
      connectionStatus,
      setConnectionStatus,
      connectionStats,
      showInfoModal,
      setConnectionStats,
      selectedProvider,
      connectedSince,
      setConnectedSince,
      setRandomSerivceProvider,
      startConnecting,
      startDisconnecting,
      gatewayPerformance,
      setShowInfoModal,
    }),
    [
      mode,
      appVersion,
      error,
      showInfoModal,
      connectedSince,
      connectionStatus,
      connectionStats,
      connectedSince,
      gatewayPerformance,
      selectedProvider,
    ],
  );

  return <ClientContext.Provider value={contextValue}>{children}</ClientContext.Provider>;
};

export const useClientContext = () => useContext(ClientContext);
