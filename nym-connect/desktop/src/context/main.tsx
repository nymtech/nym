import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { DateTime } from 'luxon';
import { invoke } from '@tauri-apps/api';
import { Error } from 'src/types/error';
import { getVersion } from '@tauri-apps/api/app';
import { useEvents } from 'src/hooks/events';
import { UserDefinedGateway, UserDefinedSPAddress } from 'src/types/service-provider';
import { getItemFromStorage, setItemInStorage } from 'src/utils';
import { ConnectionStatusKind, GatewayPerformance } from '../types';
import { ConnectionStatsItem } from '../components/ConnectionStats';
import { ServiceProvider } from '../types/directory';

const FORAGE_GATEWAY_KEY = 'nym-connect-user-gateway';
const FORAGE_SP_KEY = 'nym-connect-user-sp';

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
  userDefinedGateway?: UserDefinedGateway;
  userDefinedSPAddress: UserDefinedSPAddress;
  serviceProviders?: ServiceProvider[];
  setMode: (mode: ModeType) => void;
  clearError: () => void;
  setConnectionStatus: (connectionStatus: ConnectionStatusKind) => void;
  setConnectionStats: (connectionStats: ConnectionStatsItem[] | undefined) => void;
  setConnectedSince: (connectedSince: DateTime | undefined) => void;
  setShowInfoModal: (show: boolean) => void;
  setSerivceProvider: () => void;
  startConnecting: () => Promise<void>;
  startDisconnecting: () => Promise<void>;
  setUserDefinedGateway: React.Dispatch<React.SetStateAction<UserDefinedGateway>>;
  setUserDefinedSPAddress: React.Dispatch<React.SetStateAction<UserDefinedSPAddress>>;
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
  const [userDefinedGateway, setUserDefinedGateway] = useState<UserDefinedGateway>({
    isActive: false,
    gateway: undefined,
  });
  const [userDefinedSPAddress, setUserDefinedSPAddress] = useState<UserDefinedSPAddress>({
    isActive: false,
    address: undefined,
  });

  const getAppVersion = async () => {
    const version = await getVersion();
    return version;
  };

  useEffect(() => {
    setItemInStorage({ key: FORAGE_GATEWAY_KEY, value: userDefinedGateway });
  }, [userDefinedGateway]);

  useEffect(() => {
    setItemInStorage({ key: FORAGE_SP_KEY, value: userDefinedSPAddress });
  }, [userDefinedSPAddress]);

  const initialiseApp = async () => {
    const services = await invoke('get_services');
    const AppVersion = await getAppVersion();
    const storedUserDefinedGateway = await getItemFromStorage({ key: FORAGE_GATEWAY_KEY });
    const storedUserDefinedSP = await getItemFromStorage({ key: FORAGE_SP_KEY });

    setAppVersion(AppVersion);
    setServiceProviders(services as ServiceProvider[]);

    if (storedUserDefinedGateway) setUserDefinedGateway(storedUserDefinedGateway);
    if (storedUserDefinedSP) setUserDefinedSPAddress(storedUserDefinedSP);
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

  const shouldUseUserGateway = !!userDefinedGateway.gateway && userDefinedGateway.isActive;
  const shouldUseUserSP = !!userDefinedSPAddress.address && userDefinedSPAddress.isActive;

  const setServiceProvider = async (newServiceProvider: ServiceProvider) => {
    await invoke('set_gateway', {
      gateway: shouldUseUserGateway ? userDefinedGateway.gateway : newServiceProvider.gateway,
    });
    await invoke('set_service_provider', {
      serviceProvider: shouldUseUserSP ? userDefinedSPAddress.address : newServiceProvider.address,
    });
  };

  const getRandomSPFromList = (services: ServiceProvider[]) => {
    const randomSelection = services[Math.floor(Math.random() * services.length)];
    return randomSelection;
  };

  const buildServiceProvider = async (serviceProvider: ServiceProvider) => {
    const sp = { ...serviceProvider };

    if (shouldUseUserGateway) sp.gateway = userDefinedGateway.gateway as string;
    if (shouldUseUserSP) sp.address = userDefinedSPAddress.address as string;

    return sp;
  };

  const setSerivceProvider = async () => {
    if (serviceProviders) {
      const randomServiceProvider = getRandomSPFromList(serviceProviders);
      const withUserDefinitions = await buildServiceProvider(randomServiceProvider);
      await setServiceProvider(withUserDefinitions);
      setSelectedProvider(withUserDefinitions);
    }
    return undefined;
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
      serviceProviders,
      connectedSince,
      setConnectedSince,
      setSerivceProvider,
      startConnecting,
      startDisconnecting,
      gatewayPerformance,
      setShowInfoModal,
      userDefinedSPAddress,
      userDefinedGateway,
      setUserDefinedGateway,
      setUserDefinedSPAddress,
    }),
    [
      mode,
      appVersion,
      error,
      showInfoModal,
      serviceProviders,
      connectedSince,
      connectionStatus,
      connectionStats,
      connectedSince,
      gatewayPerformance,
      selectedProvider,
      userDefinedGateway,
      userDefinedSPAddress,
    ],
  );

  return <ClientContext.Provider value={contextValue}>{children}</ClientContext.Provider>;
};

export const useClientContext = () => useContext(ClientContext);
