import React, { createContext, useEffect, useMemo, useState } from 'react';
import { useHistory } from 'react-router-dom';
import { Account, Network, TCurrency, TMixnodeBondDetails } from '../types';
import { TUseuserBalance, useGetBalance } from '../hooks/useGetBalance';
import { config } from '../../config';
import { getMixnodeBondDetails, selectNetwork, signInWithMnemonic, signOut } from '../requests';
import { currencyMap } from '../utils';

export const { ADMIN_ADDRESS, IS_DEV_MODE } = config;

export const urls = (networkName?: Network) =>
  networkName === 'MAINNET'
    ? {
        blockExplorer: 'https://blocks.nymtech.net',
        networkExplorer: 'https://explorer.nymtech.net',
      }
    : {
        blockExplorer: `https://${networkName}-blocks.nymtech.net`,
        networkExplorer: `https://${networkName}-explorer.nymtech.net`,
      };

type TClientContext = {
  mode: 'light' | 'dark';
  clientDetails?: Account;
  mixnodeDetails?: TMixnodeBondDetails | null;
  userBalance: TUseuserBalance;
  showAdmin: boolean;
  showSettings: boolean;
  network?: Network;
  currency?: TCurrency;
  isLoading: boolean;
  error?: string;
  switchNetwork: (network: Network) => void;
  getBondDetails: () => Promise<void>;
  handleShowSettings: () => void;
  handleShowAdmin: () => void;
  logIn: (mnemonic: string) => void;
  logOut: () => void;
};

export const ClientContext = createContext({} as TClientContext);

export const ClientContextProvider = ({ children }: { children: React.ReactNode }) => {
  const [clientDetails, setClientDetails] = useState<Account>();
  const [mixnodeDetails, setMixnodeDetails] = useState<TMixnodeBondDetails | null>();
  const [network, setNetwork] = useState<Network | undefined>();
  const [currency, setCurrency] = useState<TCurrency>();
  const [showAdmin, setShowAdmin] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [mode] = useState<'light' | 'dark'>('light');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string>();

  const userBalance = useGetBalance(clientDetails?.client_address);
  const history = useHistory();

  const loadAccount = async (n: Network) => {
    try {
      const client = await selectNetwork(n);
      setClientDetails(client);
    } catch (e) {
      console.error(e);
    } finally {
      setCurrency(currencyMap(n));
    }
  };

  const getBondDetails = async () => {
    setMixnodeDetails(undefined);
    try {
      const mixnode = await getMixnodeBondDetails();
      setMixnodeDetails(mixnode);
    } catch (e) {
      console.log(e);
    }
  };

  useEffect(() => {
    const refreshAccount = async () => {
      if (network) {
        await loadAccount(network);
        await getBondDetails();
        userBalance.fetchBalance();
      }
    };
    refreshAccount();
  }, [network]);

  const logIn = async (mnemonic: string) => {
    try {
      setIsLoading(true);
      await signInWithMnemonic(mnemonic || '');
      await getBondDetails();
      setNetwork('MAINNET');
      history.push('/balance');
    } catch (e) {
      setIsLoading(false);
      setError(e as string);
    }
  };

  const logOut = async () => {
    setClientDetails(undefined);
    setNetwork(undefined);
    setError(undefined);
    setIsLoading(false);
    userBalance.clearAll();
    await signOut();
  };

  const handleShowAdmin = () => setShowAdmin((show) => !show);
  const handleShowSettings = () => setShowSettings((show) => !show);
  const switchNetwork = (_network: Network) => setNetwork(_network);

  const memoizedValue = useMemo(
    () => ({
      mode,
      isLoading,
      error,
      clientDetails,
      mixnodeDetails,
      userBalance,
      showAdmin,
      showSettings,
      network,
      currency,
      switchNetwork,
      getBondDetails,
      handleShowSettings,
      handleShowAdmin,
      logIn,
      logOut,
    }),
    [mode, isLoading, error, clientDetails, mixnodeDetails, userBalance, showAdmin, showSettings, network, currency],
  );

  return <ClientContext.Provider value={memoizedValue}>{children}</ClientContext.Provider>;
};
