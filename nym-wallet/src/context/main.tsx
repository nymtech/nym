import React, { useMemo, createContext, useEffect, useState } from 'react';
import { useHistory } from 'react-router-dom';
import { useSnackbar } from 'notistack';
import { Account, Network, TCurrency, TMixnodeBondDetails, AccountEntry, AppEnv } from '../types';
import { TUseuserBalance, useGetBalance } from '../hooks/useGetBalance';
import {
  getMixnodeBondDetails,
  selectNetwork,
  signInWithMnemonic,
  signInWithPassword,
  signOut,
  switchAccount,
  getEnv,
  listAccounts,
} from '../requests';
import { currencyMap } from '../utils';
import { Console } from '../utils/console';

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

type TLoginType = 'mnemonic' | 'password';

type TAppContext = {
  mode: 'light' | 'dark';
  appEnv?: AppEnv;
  clientDetails?: Account;
  storedAccounts?: AccountEntry[];
  mixnodeDetails?: TMixnodeBondDetails | null;
  userBalance: TUseuserBalance;
  showAdmin: boolean;
  showSettings: boolean;
  showTerminal: boolean;
  network?: Network;
  currency?: TCurrency;
  isLoading: boolean;
  isAdminAddress: boolean;
  error?: string;
  loginType?: TLoginType;
  setIsLoading: (isLoading: boolean) => void;
  setError: (value?: string) => void;
  switchNetwork: (network: Network) => void;
  getBondDetails: () => Promise<void>;
  handleShowSettings: () => void;
  handleShowAdmin: () => void;
  logIn: (opts: { type: TLoginType; value: string }) => void;
  handleShowTerminal: () => void;
  signInWithPassword: (password: string) => void;
  logOut: () => void;
  onAccountChange: (accountId: string) => void;
};

export const AppContext = createContext({} as TAppContext);

export const AppProvider = ({ children }: { children: React.ReactNode }) => {
  const [clientDetails, setClientDetails] = useState<Account>();
  const [storedAccounts, setStoredAccounts] = useState<AccountEntry[]>();
  const [mixnodeDetails, setMixnodeDetails] = useState<TMixnodeBondDetails | null>();
  const [network, setNetwork] = useState<Network | undefined>();
  const [appEnv, setAppEnv] = useState<AppEnv>();
  const [currency, setCurrency] = useState<TCurrency>();
  const [showAdmin, setShowAdmin] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showTerminal, setShowTerminal] = useState(false);
  const [mode] = useState<'light' | 'dark'>('light');
  const [loginType, setLoginType] = useState<'mnemonic' | 'password'>();
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string>();

  const userBalance = useGetBalance(clientDetails?.client_address);
  const history = useHistory();
  const { enqueueSnackbar } = useSnackbar();

  const clearState = () => {
    userBalance.clearAll();
    setStoredAccounts(undefined);
    setNetwork(undefined);
    setError(undefined);
    setIsLoading(false);
    setMixnodeDetails(undefined);
  };

  const loadAccount = async (n: Network) => {
    try {
      const client = await selectNetwork(n);
      setClientDetails(client);
    } catch (e) {
      enqueueSnackbar('Error loading account', { variant: 'error' });
      Console.error(e as string);
    } finally {
      setCurrency(currencyMap(n));
    }
  };

  const loadStoredAccounts = async () => {
    const accounts = await listAccounts();
    setStoredAccounts(accounts);
  };

  const getBondDetails = async () => {
    setMixnodeDetails(undefined);
    try {
      const mixnode = await getMixnodeBondDetails();
      setMixnodeDetails(mixnode);
    } catch (e) {
      Console.error(e as string);
    }
  };

  const refreshAccount = async (_network: Network) => {
    await loadAccount(_network);
    if (loginType === 'password') {
      await loadStoredAccounts();
    }
  };

  useEffect(() => {
    if (!clientDetails) {
      clearState();
      history.push('/');
    }
  }, [clientDetails]);

  useEffect(() => {
    if (network) {
      refreshAccount(network);
      getEnv().then(setAppEnv);
    }
  }, [network]);

  const logIn = async ({ type, value }: { type: TLoginType; value: string }) => {
    if (value.length === 0) {
      setError(`A ${type} must be provided`);
      return;
    }
    try {
      setIsLoading(true);
      if (type === 'mnemonic') {
        await signInWithMnemonic(value);
        setLoginType('mnemonic');
      } else {
        await signInWithPassword(value);
        setLoginType('password');
      }
      setNetwork('MAINNET');
      history.push('/balance');
    } catch (e) {
      setError(e as string);
    } finally {
      setIsLoading(false);
    }
  };

  const logOut = async () => {
    await signOut();
    setClientDetails(undefined);
    enqueueSnackbar('Successfully logged out', { variant: 'success' });
  };

  const onAccountChange = async (accountId: string) => {
    if (network) {
      setIsLoading(true);
      try {
        await switchAccount(accountId);
        await loadAccount(network);
        enqueueSnackbar('Account switch success', { variant: 'success', preventDuplicate: true });
      } catch (e) {
        enqueueSnackbar(`Error swtiching account: ${e}`, { variant: 'error' });
      } finally {
        setIsLoading(false);
      }
    }
  };

  const handleShowAdmin = () => setShowAdmin((show) => !show);
  const handleShowSettings = () => setShowSettings((show) => !show);
  const handleShowTerminal = () => setShowTerminal((show) => !show);
  const switchNetwork = (_network: Network) => setNetwork(_network);

  const memoizedValue = useMemo(
    () => ({
      mode,
      appEnv,
      isAdminAddress: Boolean(appEnv?.ADMIN_ADDRESS && clientDetails?.client_address === appEnv.ADMIN_ADDRESS),
      isLoading,
      error,
      clientDetails,
      storedAccounts,
      mixnodeDetails,
      userBalance,
      showAdmin,
      showSettings,
      showTerminal,
      network,
      currency,
      loginType,
      setIsLoading,
      setError,
      signInWithPassword,
      switchNetwork,
      getBondDetails,
      handleShowSettings,
      handleShowAdmin,
      handleShowTerminal,
      logIn,
      logOut,
      onAccountChange,
    }),
    [
      loginType,
      mode,
      appEnv,
      isLoading,
      error,
      clientDetails,
      mixnodeDetails,
      userBalance,
      showAdmin,
      showSettings,
      network,
      currency,
      storedAccounts,
      showTerminal,
    ],
  );

  return <AppContext.Provider value={memoizedValue}>{children}</AppContext.Provider>;
};
