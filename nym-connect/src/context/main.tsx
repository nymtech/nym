import React, { createContext, useCallback, useContext, useEffect, useMemo, useState, useRef } from 'react';
import { DateTime } from 'luxon';
import { invoke } from '@tauri-apps/api';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { forage } from '@tauri-apps/tauri-forage';
import { ConnectionStatusKind, GatewayPerformance } from '../types';
import { ConnectionStatsItem } from '../components/ConnectionStats';
import { ServiceProvider, Services } from '../types/directory';
import { Error } from 'src/types/error';
import { TauriEvent } from 'src/types/event';
import { getVersion } from '@tauri-apps/api/app';

const TAURI_EVENT_STATUS_CHANGED = 'app:connection-status-changed';

type ModeType = 'light' | 'dark';

export type TClientContext = {
  mode: ModeType;
  appVersion?: string;
  connectionStatus: ConnectionStatusKind;
  connectionStats?: ConnectionStatsItem[];
  connectedSince?: DateTime;
  services?: Services;
  serviceProvider?: ServiceProvider;
  showHelp: boolean;
  error?: Error;
  gatewayPerformance: GatewayPerformance;
  setMode: (mode: ModeType) => void;
  clearError: () => void;
  handleShowHelp: () => void;
  setConnectionStatus: (connectionStatus: ConnectionStatusKind) => void;
  setConnectionStats: (connectionStats: ConnectionStatsItem[] | undefined) => void;
  setConnectedSince: (connectedSince: DateTime | undefined) => void;
  setServiceProvider: (serviceProvider?: ServiceProvider) => void;

  startConnecting: () => Promise<void>;
  startDisconnecting: () => Promise<void>;
};

export const ClientContext = createContext({} as TClientContext);

export const ClientContextProvider = ({ children }: { children: React.ReactNode }) => {
  const [mode, setMode] = useState<ModeType>('dark');
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatusKind>(ConnectionStatusKind.disconnected);
  const [connectionStats, setConnectionStats] = useState<ConnectionStatsItem[]>();
  const [connectedSince, setConnectedSince] = useState<DateTime>();
  const [services, setServices] = React.useState<Services>([]);
  const [serviceProvider, setRawServiceProvider] = React.useState<ServiceProvider>();
  const [showHelp, setShowHelp] = useState(false);
  const [error, setError] = useState<Error>();
  const [appVersion, setAppVersion] = useState<string>();
  const [gatewayPerformance, setGatewayPerformance] = useState<GatewayPerformance>('Good');

  const getAppVersion = async () => {
    const version = await getVersion();
    setAppVersion(version);
  };

  useEffect(() => {
    invoke('get_services').then((result) => {
      setServices(result as Services);
    });
    getAppVersion();
  }, []);

  useEffect(() => {
    // when mounting, load the connection state (needed for the Growth window, that checks the connection state)
    (async () => {
      const currentStatus: ConnectionStatusKind = await invoke('get_connection_status');
      setConnectionStatus(currentStatus);
    })();
  }, []);

  useEffect(() => {
    const unlisten: UnlistenFn[] = [];

    // TODO: fix typings
    listen(TAURI_EVENT_STATUS_CHANGED, (event) => {
      const { status } = event.payload as any;
      console.log(TAURI_EVENT_STATUS_CHANGED, { status, event });
      setConnectionStatus(status);
    })
      .then((result) => {
        unlisten.push(result);
      })
      .catch((e) => console.log(e));

    listen('socks5-event', (e: TauriEvent) => {
      console.log(e);

      setError(e.payload);
    }).then((result) => {
      unlisten.push(result);
    });

    listen('socks5-status-event', (e: TauriEvent) => {
      if (e.payload.message.includes('slow')) {
        setGatewayPerformance('Poor');
      } else {
        setGatewayPerformance('Good');
      }
    }).then((result) => {
      unlisten.push(result);
    });

    return () => {
      unlisten.forEach((unsubscribe) => unsubscribe());
    };
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
      setGatewayPerformance('Good');
    } catch (e) {
      console.log(e);
    }
  }, []);

  const setSpInStorage = async (sp: ServiceProvider) => {
    await forage.setItem({
      key: 'nym-connect-sp',
      value: sp,
    } as any)();
  };

  const setServiceProvider = useCallback(async (newServiceProvider?: ServiceProvider) => {
    await invoke('set_gateway', { gateway: newServiceProvider?.gateway });
    await invoke('set_service_provider', { serviceProvider: newServiceProvider?.address });
    if (newServiceProvider) {
      await setSpInStorage(newServiceProvider);
    }
    setRawServiceProvider(newServiceProvider);
  }, []);

  const getSpFromStorage = async () => {
    try {
      const spFromStorage = await forage.getItem({ key: 'nym-connect-sp' })();
      if (spFromStorage) {
        setRawServiceProvider(spFromStorage);
        setServiceProvider(spFromStorage);
      }
    } catch (e) {
      console.warn(e);
    }
  };

  const handleShowHelp = () => setShowHelp((show) => !show);

  const clearError = () => setError(undefined);

  useEffect(() => {
    const validityCheck = async () => {
      if (services.length > 0 && serviceProvider) {
        const isValid = services.some(({ items }) => items.some(({ id }) => id === serviceProvider.id));
        if (!isValid) {
          console.warn('invalid SP, cleaning local storage');
          await forage.removeItem({
            key: 'nym-connect-sp',
          })();
          setRawServiceProvider(undefined);
        }
      }
    };
    validityCheck();
  }, [services, serviceProvider]);

  useEffect(() => {
    getSpFromStorage();
  }, []);

  const timerId = useRef<NodeJS.Timeout>();

  useEffect(() => {
    if (timerId.current) {
      clearTimeout(timerId.current);
    }

    if (gatewayPerformance !== 'Good') {
      timerId.current = setTimeout(() => {
        setGatewayPerformance('Good');
      }, 15000);
    }
  }, [gatewayPerformance]);

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
      setConnectionStats,
      connectedSince,
      setConnectedSince,
      startConnecting,
      startDisconnecting,
      services,
      serviceProvider,
      setServiceProvider,
      showHelp,
      handleShowHelp,
      gatewayPerformance,
    }),
    [
      appVersion,
      mode,
      appVersion,
      error,
      connectedSince,
      showHelp,
      connectionStatus,
      connectionStats,
      connectedSince,
      services,
      serviceProvider,
      gatewayPerformance,
    ],
  );

  return <ClientContext.Provider value={contextValue}>{children}</ClientContext.Provider>;
};

export const useClientContext = () => useContext(ClientContext);
