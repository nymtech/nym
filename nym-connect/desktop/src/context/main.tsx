import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { DateTime } from 'luxon';
import { invoke } from '@tauri-apps/api';
import { Error } from 'src/types/error';
import { getVersion } from '@tauri-apps/api/app';
import * as Sentry from '@sentry/react';
import { useEvents } from 'src/hooks/events';
import { UserDefinedGateway, UserDefinedSPAddress } from 'src/types/service-provider';
import { getItemFromStorage, setItemInStorage } from 'src/utils';
import { ConnectionStatusKind, GatewayPerformance, PrivacyLevel, UserData } from '../types';
import { ConnectionStatsItem } from '../components/ConnectionStats';
import { ServiceProvider, Gateway } from '../types/directory';
import initSentry from '../sentry';

const FORAGE_GATEWAY_KEY = 'nym-connect-user-gateway';
const FORAGE_SP_KEY = 'nym-connect-user-sp';

type ModeType = 'light' | 'dark';

export type TClientContext = {
  mode: ModeType;
  appVersion?: string;
  userData?: UserData;
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
  gateways?: Gateway[];
  setMode: (mode: ModeType) => void;
  clearError: () => void;
  setConnectionStatus: (connectionStatus: ConnectionStatusKind) => void;
  setConnectionStats: (connectionStats: ConnectionStatsItem[] | undefined) => void;
  setConnectedSince: (connectedSince: DateTime | undefined) => void;
  setShowInfoModal: (show: boolean) => void;
  setServiceProvider: () => void;
  setGateway: () => void;
  startConnecting: () => Promise<void>;
  startDisconnecting: () => Promise<void>;
  setUserDefinedGateway: React.Dispatch<React.SetStateAction<UserDefinedGateway>>;
  setUserDefinedSPAddress: React.Dispatch<React.SetStateAction<UserDefinedSPAddress>>;
  setMonitoring: (value: boolean) => Promise<void>;
  setPrivacyLevel: (value: PrivacyLevel) => Promise<void>;
};

export const ClientContext = createContext({} as TClientContext);

export const ClientContextProvider: FCWithChildren = ({ children }) => {
  const [mode, setMode] = useState<ModeType>('dark');
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatusKind>(ConnectionStatusKind.connected);
  const [connectionStats, setConnectionStats] = useState<ConnectionStatsItem[]>();
  const [connectedSince, setConnectedSince] = useState<DateTime>();
  const [selectedProvider, setSelectedProvider] = React.useState<ServiceProvider>();
  const [serviceProviders, setServiceProviders] = React.useState<ServiceProvider[]>();
  const [gateways, setGateways] = React.useState<Gateway[]>();
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
  const [userData, setUserData] = useState<UserData>();

  const getAppVersion = async () => {
    const version = await getVersion();
    return version;
  };

  const getUserData = async () => {
    const data = await invoke<UserData>('get_user_data');
    if (!data.privacy_level) {
      data.privacy_level = 'High';
    }
    setUserData(data);
    return data;
  };

  useEffect(() => {
    const initSentryClient = async () => {
      const data = await getUserData();
      if (data.monitoring) {
        await initSentry();
      }
    };

    initSentryClient();
  }, []);

  useEffect(() => {
    setItemInStorage({ key: FORAGE_GATEWAY_KEY, value: userDefinedGateway });
  }, [userDefinedGateway]);

  useEffect(() => {
    setItemInStorage({ key: FORAGE_SP_KEY, value: userDefinedSPAddress });
  }, [userDefinedSPAddress]);

  const initialiseApp = async () => {
    const fetchedServices = await invoke('get_services');
    const fetchedGateways = await invoke('get_gateways');
    const AppVersion = await getAppVersion();
    const storedUserDefinedGateway = await getItemFromStorage({ key: FORAGE_GATEWAY_KEY });
    const storedUserDefinedSP = await getItemFromStorage({ key: FORAGE_SP_KEY });

    setAppVersion(AppVersion);
    setServiceProviders(fetchedServices as ServiceProvider[]);
    setGateways(fetchedGateways as Gateway[]);

    if (storedUserDefinedGateway) {
      setUserDefinedGateway(storedUserDefinedGateway);
    }
    if (storedUserDefinedSP) {
      setUserDefinedSPAddress(storedUserDefinedSP);
    }
  };

  useEvents({
    onError: (e) => {
      setError(e);
      Sentry.captureException(e);
    },
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
      Sentry.captureException(e);
    }
  }, []);

  const startDisconnecting = useCallback(async () => {
    try {
      await invoke('start_disconnecting');
    } catch (e) {
      console.log(e);
      Sentry.captureException(e);
    }
  }, []);

  const shouldUseUserGateway = !!userDefinedGateway.gateway && userDefinedGateway.isActive;
  const shouldUseUserSP = !!userDefinedSPAddress.address && userDefinedSPAddress.isActive;

  const applyServiceProvider = async (newServiceProvider: ServiceProvider) => {
    await invoke('set_service_provider', {
      serviceProvider: shouldUseUserSP ? userDefinedSPAddress.address : newServiceProvider.address,
    });
  };

  const applyGateway = async (newGateway: Gateway) => {
    await invoke('set_gateway', {
      gateway: shouldUseUserGateway ? userDefinedGateway.gateway : newGateway.identity,
    });
  }

  const getRandomSPFromList = (services: ServiceProvider[]) => {
    const randomSelection = services[Math.floor(Math.random() * services.length)];
    return randomSelection;
  };

  const getRandomGatewayFromList = (theGateways: Gateway[]) => {
    const randomSelection = theGateways[Math.floor(Math.random() * theGateways.length)];
    return randomSelection;
  };

  const buildServiceProvider = async (serviceProvider: ServiceProvider) => {
    const sp = { ...serviceProvider };
    if (shouldUseUserSP) sp.address = userDefinedSPAddress.address as string;
    return sp;
  };

  const buildGateway = async (gateway: Gateway) => {
    const gw = { ...gateway };
    if (shouldUseUserGateway) gw.identity = userDefinedGateway.gateway as string;
    return gw;
  };

  const setServiceProvider = async () => {
    if (serviceProviders) {
      const randomServiceProvider = getRandomSPFromList(serviceProviders);
      const withUserDefinitions = await buildServiceProvider(randomServiceProvider);
      await applyServiceProvider(withUserDefinitions);
      setSelectedProvider(withUserDefinitions);
    }
    return undefined;
  };

  const setGateway = async () => {
    if (gateways) {
      const randomGateway = getRandomGatewayFromList(gateways);
      const withUserDefinitionsForGateway = await buildGateway(randomGateway);
      await applyGateway(withUserDefinitionsForGateway);
      // Do we need a setSelectedGateway?
    }
    return undefined;
  };

  const clearError = () => setError(undefined);

  const setMonitoring = async (value: boolean) => {
    await invoke('set_monitoring', { enabled: value });
    // refresh user data
    await getUserData();
  };

  const setPrivacyLevel = async (value: PrivacyLevel) => {
    await invoke('set_privacy_level', { privacyLevel: value });
    // refresh user data
    await getUserData();
  };

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
      userData,
      setConnectedSince,
      setServiceProvider,
      setGateway,
      startConnecting,
      startDisconnecting,
      gatewayPerformance,
      setShowInfoModal,
      userDefinedSPAddress,
      userDefinedGateway,
      setUserDefinedGateway,
      setUserDefinedSPAddress,
      setMonitoring,
      setPrivacyLevel,
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
      userData,
    ],
  );

  return <ClientContext.Provider value={contextValue}>{children}</ClientContext.Provider>;
};

export const useClientContext = () => useContext(ClientContext);
