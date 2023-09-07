import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { DateTime } from 'luxon';
import { invoke } from '@tauri-apps/api';
import { Error } from 'src/types/error';
import { getVersion } from '@tauri-apps/api/app';
import * as Sentry from '@sentry/react';
import { useEvents } from 'src/hooks/events';
import { UserDefinedGateway, UserDefinedSPAddress } from 'src/types/service-provider';
import { ConnectionStatusKind, GatewayPerformance, PrivacyLevel, UserData } from '../types';
import { ConnectionStatsItem } from '../components/ConnectionStats';
import { ServiceProvider, Gateway } from '../types/directory';
import initSentry from '../sentry';

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
  selectedGateway?: Gateway;
  showInfoModal: boolean;
  userDefinedGateway?: UserDefinedGateway;
  userDefinedSPAddress: UserDefinedSPAddress;
  serviceProviders?: ServiceProvider[];
  gateways?: Gateway[];
  showFeedbackNote: boolean;
  setMode: (mode: ModeType) => void;
  clearError: () => void;
  setConnectionStatus: (connectionStatus: ConnectionStatusKind) => void;
  setConnectionStats: (connectionStats: ConnectionStatsItem[] | undefined) => void;
  setConnectedSince: (connectedSince: DateTime | undefined) => void;
  setShowInfoModal: (show: boolean) => void;
  setServiceProvider: () => Promise<void>;
  setGateway: () => Promise<void>;
  startConnecting: () => Promise<void>;
  startDisconnecting: () => Promise<void>;
  setUserDefinedGateway: React.Dispatch<React.SetStateAction<UserDefinedGateway>>;
  setUserDefinedSPAddress: React.Dispatch<React.SetStateAction<UserDefinedSPAddress>>;
  setMonitoring: (value: boolean) => Promise<void>;
  setPrivacyLevel: (value: PrivacyLevel) => Promise<void>;
  setShowFeedbackNote: (value: boolean) => void;
};

function getRandomFromList<T>(items: T[]): T {
  return items[Math.floor(Math.random() * items.length)];
}

export const ClientContext = createContext({} as TClientContext);

export const ClientContextProvider: FCWithChildren = ({ children }) => {
  const [mode, setMode] = useState<ModeType>('dark');
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatusKind>(ConnectionStatusKind.connected);
  const [connectionStats, setConnectionStats] = useState<ConnectionStatsItem[]>();
  const [connectedSince, setConnectedSince] = useState<DateTime>();
  const [selectedProvider, setSelectedProvider] = React.useState<ServiceProvider>();
  const [selectedGateway, setSelectedGateway] = React.useState<Gateway>();
  const [serviceProviders, setServiceProviders] = React.useState<ServiceProvider[]>();
  const [gateways, setGateways] = React.useState<Gateway[]>();
  const [error, setError] = useState<Error>();
  const [appVersion, setAppVersion] = useState<string>();
  const [gatewayPerformance, setGatewayPerformance] = useState<GatewayPerformance>('Good');
  const [showInfoModal, setShowInfoModal] = useState(false);
  const [userDefinedGateway, setUserDefinedGateway] = useState<UserDefinedGateway>({
    isActive: false,
    address: undefined,
  });
  const [userDefinedSPAddress, setUserDefinedSPAddress] = useState<UserDefinedSPAddress>({
    isActive: false,
    address: undefined,
  });
  const [userData, setUserData] = useState<UserData>();
  const [showFeedbackNote, setShowFeedbackNote] = useState(true);

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
    if (data.selected_gateway) {
      setUserDefinedGateway({
        address: data.selected_gateway.address,
        isActive: data.selected_gateway.is_active || false,
      });
    }
    if (data.selected_sp) {
      setUserDefinedSPAddress({ address: data.selected_sp.address, isActive: data.selected_sp.is_active || false });
    }
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
    const saveUserGateway = async () => {
      await invoke('set_selected_gateway', {
        gateway: { address: userDefinedGateway.address, is_active: userDefinedGateway.isActive },
      });
    };
    saveUserGateway();
  }, [userDefinedGateway]);

  useEffect(() => {
    const saveUserServiceProvider = async () => {
      await invoke('set_selected_sp', {
        serviceProvider: { address: userDefinedSPAddress.address, is_active: userDefinedSPAddress.isActive },
      });
    };
    saveUserServiceProvider();
  }, [userDefinedSPAddress]);

  const initialiseApp = async () => {
    const fetchedServices = await invoke<ServiceProvider[]>('get_services');
    const fetchedGateways = await invoke<Gateway[]>('get_gateways');
    const AppVersion = await getAppVersion();

    setAppVersion(AppVersion);
    setServiceProviders(fetchedServices);
    setGateways(fetchedGateways);
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

  const afterDisconnection = useCallback(async () => {
    setGatewayPerformance('Good');
    setConnectedSince(undefined);
  }, []);

  const startDisconnecting = useCallback(async () => {
    try {
      await invoke('start_disconnecting');
      afterDisconnection();
    } catch (e) {
      console.log(e);
      Sentry.captureException(e);
    }
  }, []);

  const shouldUseUserGateway = !!userDefinedGateway.address && userDefinedGateway.isActive;
  const shouldUseUserSP = !!userDefinedSPAddress.address && userDefinedSPAddress.isActive;

  const buildServiceProvider = async (serviceProvider: ServiceProvider) => {
    const sp = { ...serviceProvider };
    if (shouldUseUserSP) sp.address = userDefinedSPAddress.address as string;
    return sp;
  };

  const buildGateway = async (gateway: Gateway) => {
    const gw = { ...gateway };
    if (shouldUseUserGateway) gw.identity = userDefinedGateway.address as string;
    return gw;
  };

  const setServiceProvider = async () => {
    if (serviceProviders) {
      const randomServiceProvider = getRandomFromList(serviceProviders);
      const withUserDefinitions = await buildServiceProvider(randomServiceProvider);
      await invoke('set_service_provider', {
        serviceProvider: shouldUseUserSP ? userDefinedSPAddress.address : withUserDefinitions.address,
      });
      setSelectedProvider(withUserDefinitions);
    }
    return undefined;
  };

  const setGateway = async () => {
    if (gateways) {
      let randomGateway;
      if (userData?.privacy_level === 'Medium') {
        randomGateway = await invoke<Gateway>('select_gateway_with_low_latency_from_list', { gateways });
      } else {
        randomGateway = getRandomFromList(gateways);
      }
      const withUserDefinitions = await buildGateway(randomGateway);
      await invoke('set_gateway', {
        gateway: shouldUseUserGateway ? userDefinedGateway.address : withUserDefinitions.identity,
      });
      setSelectedGateway(withUserDefinitions);
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
    // refresh service providers list
    const fetchedServices = await invoke<ServiceProvider[]>('get_services');
    setServiceProviders(fetchedServices);
    // reset any previously selected SP
    await invoke('set_selected_sp', {
      serviceProvider: { is_active: false },
    });
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
      selectedGateway,
      serviceProviders,
      connectedSince,
      userData,
      showFeedbackNote,
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
      setShowFeedbackNote,
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
      selectedGateway,
      userDefinedGateway,
      userDefinedSPAddress,
      userData,
      showFeedbackNote,
    ],
  );

  return <ClientContext.Provider value={contextValue}>{children}</ClientContext.Provider>;
};

export const useClientContext = () => useContext(ClientContext);
