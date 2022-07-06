import React, { createContext, useCallback, useContext, useEffect, useState } from 'react';
import { DateTime } from 'luxon';
import { invoke } from '@tauri-apps/api/tauri';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { ConnectionStatusKind } from '../types';
import { ConnectionStatsItem } from '../components/ConnectionStats';
import { ServiceProvider, Services } from '../types/directory';

const TAURI_EVENT_STATUS_CHANGED = 'app:connection-status-changed';

type ModeType = 'light' | 'dark';

type TClientContext = {
  mode: ModeType;
  connectionStatus: ConnectionStatusKind;
  connectionStats?: ConnectionStatsItem[];
  connectedSince?: DateTime;
  services?: Services;
  serviceProvider?: ServiceProvider;

  setMode: (mode: ModeType) => void;
  setConnectionStatus: (connectionStatus: ConnectionStatusKind) => void;
  setConnectionStats: (connectionStats: ConnectionStatsItem[] | undefined) => void;
  setConnectedSince: (connectedSince: DateTime | undefined) => void;
  setServiceProvider: (serviceProvider: ServiceProvider) => void;

  startConnecting: () => Promise<void>;
  startDisconnecting: () => Promise<void>;
};

export const ClientContext = createContext({} as TClientContext);

export const ClientContextProvider = ({ children }: { children: React.ReactNode }) => {
  const [mode, setMode] = useState<ModeType>('dark');
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatusKind>(ConnectionStatusKind.disconnected);
  const [connectionStats, setConnectionStats] = useState<ConnectionStatsItem[]>();
  const [connectedSince, setConnectedSince] = useState<DateTime>();
  const [services, setServices] = React.useState<Services>();
  const [serviceProvider, setRawServiceProvider] = React.useState<ServiceProvider>();

  useEffect(() => {
    invoke('get_services').then((result) => {
      setServices(result as Services);
    });
  }, []);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    // TODO: fix typings
    listen(TAURI_EVENT_STATUS_CHANGED, (event) => {
      const { status } = event.payload as any;
      console.log(TAURI_EVENT_STATUS_CHANGED, { status, event });
      setConnectionStatus(status);
    }).then((result) => {
      unlisten = result;
    });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  const startConnecting = useCallback(async () => {
    await invoke('start_connecting');
  }, []);

  const startDisconnecting = useCallback(async () => {
    await invoke('start_disconnecting');
  }, []);

  const setServiceProvider = useCallback(async (newServiceProvider: ServiceProvider) => {
    await invoke('set_gateway', { gateway: newServiceProvider.gateway });
    await invoke('set_service_provider', { serviceProvider: newServiceProvider.address });
    setRawServiceProvider(newServiceProvider);
  }, []);

  return (
    <ClientContext.Provider
      value={{
        mode,
        setMode,
        connectionStatus,
        setConnectionStatus,
        connectionStats,
        setConnectionStats,
        connectedSince,
        setConnectedSince,
        startConnecting,
        startDisconnecting,
        services,
        serviceProvider,
        setServiceProvider,
      }}
    >
      {children}
    </ClientContext.Provider>
  );
};

export const useClientContext = () => useContext(ClientContext);
