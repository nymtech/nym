import React, { createContext, useEffect, useMemo, useState } from 'react';
import { forage } from '@tauri-apps/tauri-forage';
import { useNavigate } from 'react-router-dom';
import { useSnackbar } from 'notistack';
import { Account, AccountEntry, MixNodeDetails } from '@nymproject/types';
import { getVersion } from '@tauri-apps/api/app';
import { AppEnv, Network } from '../types';
import { TUseuserBalance, useGetBalance } from '../hooks/useGetBalance';
import {
  getEnv,
  getMixnodeBondDetails,
  listAccounts,
  selectNetwork,
  signInWithMnemonic,
  signInWithPassword,
  signOut,
  switchAccount,
} from '../requests';
import Big from 'big.js';

import { Console } from '../utils/console';
import { createSignInWindow, getReactState, setReactState } from '../requests/app';
// import { toDisplay } from '../utils';

const toDisplay = (val: string | number | Big, dp = 4) => {
  let displayValue;
  try {
    displayValue = Big(val).toFixed(dp);
  } catch (e: any) {
    Console.warn(`${displayValue} not a valid decimal number: ${e}`);
  }
  return displayValue;
};

export const urls = (networkName?: Network) =>
  networkName === 'MAINNET'
    ? {
        mixnetExplorer: 'https://mixnet.explorers.guru/',
        blockExplorer: 'https://blocks.nymtech.net',
        networkExplorer: 'https://explorer.nymtech.net',
      }
    : {
        blockExplorer: `https://${networkName}-blocks.nymtech.net`,
        networkExplorer: `https://${networkName}-explorer.nymtech.net`,
      };

type TLoginType = 'mnemonic' | 'password';

export type TAppContext = {
  mode: 'light' | 'dark';
  appEnv?: AppEnv;
  appVersion?: string;
  clientDetails?: Account;
  storedAccounts?: AccountEntry[];
  mixnodeDetails?: MixNodeDetails | null;
  userBalance: TUseuserBalance;
  showAdmin: boolean;
  showTerminal: boolean;
  network?: Network;
  isLoading: boolean;
  isAdminAddress: boolean;
  error?: string;
  loginType?: TLoginType;
  showSendModal: boolean;
  showReceiveModal: boolean;
  onAccountChange: ({ accountId, password }: { accountId: string; password: string }) => void;
  handleSwitchMode: () => void;
  handleShowSendModal: () => void;
  handleShowReceiveModal: () => void;
  setIsLoading: (isLoading: boolean) => void;
  setError: (value?: string) => void;
  switchNetwork: (network: Network) => void;
  getBondDetails: () => Promise<void>;
  handleShowAdmin: () => void;
  logIn: (opts: { type: TLoginType; value: string }) => void;
  handleShowTerminal: () => void;
  signInWithPassword: (password: string) => void;
  logOut: () => void;
  keepState: () => Promise<void>;
  printBalance: string;
  printVestedBalance?: string; // spendable vested token
};

interface RustState {
  network?: Network;
  loginType?: 'mnemonic' | 'password';
}

export const AppContext = createContext({} as TAppContext);

export const AppProvider: FCWithChildren = ({ children }) => {
  const [clientDetails, setClientDetails] = useState<Account>();
  const [storedAccounts, setStoredAccounts] = useState<AccountEntry[]>();
  const [mixnodeDetails, setMixnodeDetails] = useState<MixNodeDetails | null>(null);
  const [network, setNetwork] = useState<Network | undefined>();
  const [appEnv, setAppEnv] = useState<AppEnv>();
  const [showAdmin, setShowAdmin] = useState(false);
  const [showTerminal, setShowTerminal] = useState(false);
  const [mode, setMode] = useState<'light' | 'dark'>('light');
  const [loginType, setLoginType] = useState<'mnemonic' | 'password'>();
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string>();
  const [appVersion, setAppVersion] = useState<string>();
  const [isAdminAddress, setIsAdminAddress] = useState<boolean>(false);
  const [showSendModal, setShowSendModal] = useState(false);
  const [showReceiveModal, setShowReceiveModal] = useState(false);
  const [printBalance, setPrintBalance] = useState<string>('-');
  const [printVestedBalance, setPrintVestedBalance] = useState<string | undefined>();

  const userBalance = useGetBalance(clientDetails);
  const navigate = useNavigate();
  const { enqueueSnackbar } = useSnackbar();

  const initFromRustState = async () => {
    const stateJson = await getReactState();
    if (stateJson) {
      const state: RustState = JSON.parse(stateJson);
      setNetwork(state.network);
      setLoginType(state.loginType);
    }
  };

  useEffect(() => {
    initFromRustState();
  }, []);

  const keepState = async () => {
    // add any state from this context to store in the Rust process
    const state: RustState = {
      network,
      loginType,
    };
    setReactState(JSON.stringify(state));
  };

  const clearState = () => {
    userBalance.clearAll();
    setStoredAccounts(undefined);
    setNetwork(undefined);
    setError(undefined);
    setIsLoading(false);
    setMixnodeDetails(null);
  };

  const loadAccount = async (n: Network) => {
    try {
      const client = await selectNetwork(n);
      setClientDetails(client);
    } catch (e) {
      enqueueSnackbar('Error loading account', { variant: 'error' });
      Console.error(e as string);
    }
  };

  const loadStoredAccounts = async () => {
    const accounts = await listAccounts();
    setStoredAccounts(accounts);
  };

  const getBondDetails = async () => {
    setMixnodeDetails(null);
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

  const getModeFromStorage = async () => {
    try {
      const modeFromStorage = await forage.getItem({ key: 'nym-wallet-mode' })();
      if (modeFromStorage) setMode(modeFromStorage);
    } catch (e) {
      Console.error(e);
    }
  };

  const setModeInStorage = async (newMode: 'light' | 'dark') => {
    await forage.setItem({
      key: 'nym-wallet-mode',
      value: newMode,
    })();
  };

  useEffect(() => {
    getVersion().then(setAppVersion);
    getModeFromStorage();
  }, []);

  useEffect(() => {
    if (!clientDetails) {
      clearState();
      navigate('/');
    }
  }, [clientDetails]);

  useEffect(() => {
    if (network) {
      refreshAccount(network);
      getEnv().then(setAppEnv);
    }
  }, [network]);

  useEffect(() => {
    const currency = clientDetails?.display_mix_denom.toUpperCase() || 'NYM';
    if (userBalance.originalVesting) {
      setPrintVestedBalance(`${toDisplay(userBalance.tokenAllocation?.spendableVestedCoins || 0)} ${currency}`);
    }
    if (userBalance?.balance?.amount) {
      setPrintBalance(`${toDisplay(userBalance.balance.amount.amount)} ${currency}`);
    } else {
      setPrintBalance(`${toDisplay(0)} ${currency}`);
    }
  }, [userBalance, clientDetails]);

  useEffect(() => {
    let newValue = false;
    if (network && appEnv?.ADMIN_ADDRESS && clientDetails?.client_address) {
      try {
        const adminAddressMap = JSON.parse(appEnv.ADMIN_ADDRESS);
        const adminAddresses = adminAddressMap[network] || [];
        if (adminAddresses.length) {
          newValue = adminAddresses.includes(clientDetails?.client_address);
          if (newValue) {
            Console.log('Wallet is in admin mode: ', {
              network,
              adminAddress: adminAddressMap[network],
              clientAddress: clientDetails?.client_address,
            });
          }
        }
      } catch (e) {
        Console.error('Failed to check admin addresses', e);
      }
    }
    setIsAdminAddress(newValue);
  }, [appEnv, network, clientDetails?.client_address]);

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
      navigate('/balance');
    } catch (e) {
      setError(e as string);
    } finally {
      setIsLoading(false);
    }
  };

  const logOut = async () => {
    setIsLoading(true);
    try {
      await signOut();
      await setReactState(undefined);
      setClientDetails(undefined);
      enqueueSnackbar('Successfully logged out', { variant: 'success' });
      await createSignInWindow();
    } finally {
      setIsLoading(false);
    }
  };

  const onAccountChange = async ({ accountId, password }: { accountId: string; password: string }) => {
    if (network) {
      setIsLoading(true);
      try {
        await switchAccount({ accountId, password });
        await loadAccount(network);
        enqueueSnackbar('Account switch success', { variant: 'success', preventDuplicate: true });
      } catch (e) {
        throw new Error(`Error swtiching account: ${e}`);
      } finally {
        setIsLoading(false);
      }
    }
  };

  const handleShowAdmin = () => setShowAdmin((show) => !show);
  const handleShowTerminal = () => setShowTerminal((show) => !show);
  const switchNetwork = (_network: Network) => setNetwork(_network);
  const handleShowSendModal = () => setShowSendModal((show) => !show);
  const handleShowReceiveModal = () => setShowReceiveModal((show) => !show);
  const handleSwitchMode = () =>
    setMode((currentMode) => {
      const newMode = currentMode === 'light' ? 'dark' : 'light';
      setModeInStorage(newMode);
      return newMode;
    });

  const memoizedValue = useMemo(
    () => ({
      mode,
      appEnv,
      appVersion,
      isAdminAddress,
      isLoading,
      error,
      clientDetails,
      storedAccounts,
      mixnodeDetails,
      userBalance,
      showAdmin,
      showTerminal,
      network,
      loginType,
      setIsLoading,
      setError,
      signInWithPassword,
      switchNetwork,
      getBondDetails,
      handleShowAdmin,
      handleShowTerminal,
      logIn,
      logOut,
      keepState,
      onAccountChange,
      showSendModal,
      showReceiveModal,
      handleShowSendModal,
      handleShowReceiveModal,
      handleSwitchMode,
      printBalance,
      printVestedBalance,
    }),
    [
      appVersion,
      loginType,
      isAdminAddress,
      mode,
      appEnv,
      isLoading,
      error,
      clientDetails,
      mixnodeDetails,
      userBalance,
      showAdmin,
      network,
      storedAccounts,
      showTerminal,
      showSendModal,
      showReceiveModal,
    ],
  );

  return <AppContext.Provider value={memoizedValue}>{children}</AppContext.Provider>;
};
