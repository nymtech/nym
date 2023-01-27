import React, { createContext, useCallback, useContext, useEffect, useMemo, useState, useRef } from 'react';
import { DateTime } from 'luxon';
import { invoke } from '@tauri-apps/api';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { forage } from '@tauri-apps/tauri-forage';
import { Error } from 'src/types/error';
import { TauriEvent } from 'src/types/event';
import { getVersion } from '@tauri-apps/api/app';
import { ConnectionStatusKind, GatewayPerformance } from '../types';
import { ConnectionStatsItem } from '../components/ConnectionStats';
import { ServiceProvider, Services } from '../types/directory';

const TAURI_EVENT_STATUS_CHANGED = 'app:connection-status-changed';

type ModeType = 'light' | 'dark';

export type TClientContext = {
  mode: ModeType;
  appVersion?: string;
  connectionStatus: ConnectionStatusKind;
  connectionStats?: ConnectionStatsItem[];
  connectedSince?: DateTime;
  error?: Error;
  gatewayPerformance: GatewayPerformance;
  selectedProvider: ServiceProvider | undefined;
  setMode: (mode: ModeType) => void;
  clearError: () => void;
  setConnectionStatus: (connectionStatus: ConnectionStatusKind) => void;
  setConnectionStats: (connectionStats: ConnectionStatsItem[] | undefined) => void;
  setConnectedSince: (connectedSince: DateTime | undefined) => void;
  setRandomSerivceProvider: () => void;
  startConnecting: () => Promise<void>;
  startDisconnecting: () => Promise<void>;
};

export const ClientContext = createContext({} as TClientContext);

export const ClientContextProvider: FCWithChildren = ({ children }) => {
  const [mode, setMode] = useState<ModeType>('dark');
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatusKind>('connected');
  const [connectionStats, setConnectionStats] = useState<ConnectionStatsItem[]>();
  const [connectedSince, setConnectedSince] = useState<DateTime>();
  const [selectedProvider, setSelectedProvider] = React.useState<ServiceProvider>();
  const [serviceProviders, setServiceProviders] = React.useState<ServiceProvider[]>();
  const [error, setError] = useState<Error>();
  const [appVersion, setAppVersion] = useState<string>();
  const [gatewayPerformance, setGatewayPerformance] = useState<GatewayPerformance>('Good');

  const getAppVersion = async () => {
    const version = await getVersion();
    return version;
  };

  const timerId = useRef<NodeJS.Timeout>();

  const flattenProviders = (services: Services) => {
    return services.reduce((a: ServiceProvider[], b) => {
      return [...a, ...b.items];
    }, []);
  };

  const initialiseApp = async () => {
    const services = await invoke('get_services');
    const allServiceProviders = flattenProviders(services as Services);
    const AppVersion = await getAppVersion();

    setAppVersion(AppVersion);
    setServiceProviders(allServiceProviders);
  };

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

        if (timerId.current) {
          clearTimeout(timerId.current);
        }

        timerId.current = setTimeout(() => {
          setGatewayPerformance('Good');
        }, 10000);
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
    } catch (e) {
      console.log(e);
    }
  }, []);

  const setServiceProvider = useCallback(async (newServiceProvider?: ServiceProvider) => {
    if (newServiceProvider) {
      await invoke('set_gateway', { gateway: newServiceProvider.gateway });
      await invoke('set_service_provider', { serviceProvider: newServiceProvider.address });
    }
  }, []);

  const setSpInStorage = async (sp: ServiceProvider) => {
    await forage.setItem({
      key: 'nym-connect-sp',
      value: sp,
    } as any)();
  };

  const removeSpFromStorage = async () => {
    await forage.removeItem({
      key: 'nym-connect-sp',
    })();
  };

  const getSpFromStorage = async (): Promise<ServiceProvider | undefined> => {
    try {
      const spFromStorage = await forage.getItem({ key: 'nym-connect-sp' })();
      return spFromStorage;
    } catch (e) {
      console.warn(e);
    }
  };

  const getRandomSPFromList = (serviceProviders: ServiceProvider[]) => {
    const randomSelection = serviceProviders[Math.floor(Math.random() * serviceProviders.length)];
    return randomSelection;
  };

  const setRandomSerivceProvider = async () => {
    if (serviceProviders) {
      const randomServiceProvider = getRandomSPFromList(serviceProviders);
      setSelectedProvider(randomServiceProvider);
    }
  };
  const clearError = () => setError(undefined);

  const handleUpdateServiceProvider = async (serviceProvider: ServiceProvider, serviceProviders: ServiceProvider[]) => {
    const isSelectedProviderInList = serviceProviders.some(({ address }) => serviceProvider.address === address);

    if (!isSelectedProviderInList) {
      console.warn('invalid SP, cleaning local storage');
      setSelectedProvider(undefined);
    } else {
      await setServiceProvider(serviceProvider);
    }
  };

  useEffect(() => {
    if (serviceProviders && selectedProvider) {
      handleUpdateServiceProvider(selectedProvider, serviceProviders);
    }
  }, [selectedProvider, serviceProviders]);

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
      selectedProvider,
      connectedSince,
      setConnectedSince,
      setRandomSerivceProvider,
      startConnecting,
      startDisconnecting,
      gatewayPerformance,
    }),
    [
      appVersion,
      mode,
      appVersion,
      error,
      connectedSince,
      connectionStatus,
      connectionStats,
      connectedSince,
      gatewayPerformance,
    ],
  );

  return <ClientContext.Provider value={contextValue}>{children}</ClientContext.Provider>;
};

export const useClientContext = () => useContext(ClientContext);
