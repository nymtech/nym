import React, { createContext, useCallback, useEffect, useMemo, useState } from 'react';
import { forage } from '@tauri-apps/tauri-forage';
import { useNavigate } from 'react-router-dom';
import { useSnackbar } from 'notistack';
import { Account, AccountEntry, MixNodeDetails } from '@nymproject/types';
import { getVersion } from '@tauri-apps/api/app';
import { AppEnv, Network, TauriContractStateParams } from '../types';
import { TUseuserBalance, useGetBalance } from '../hooks/useGetBalance';
import {
  getContractParams,
  getEnv,
  getMixnodeBondDetails,
  listAccounts,
  selectNetwork,
  signInWithMnemonic,
  signInWithPassword,
  signOut,
  switchAccount,
} from '../requests';
import { Console } from '../utils/console';
import { createSignInWindow, getReactState, setReactState } from '../requests/app';
import { toDisplay } from '../utils';

export const urls = (networkName?: Network) =>
  networkName === 'MAINNET'
    ? {
        mixnetExplorer: 'https://explorer.nym.spectredao.net/',
        blockExplorer: 'https://ping.pub/nyx',
        networkExplorer: 'https://explorer.nym.spectredao.net',
      }
    : {
        blockExplorer: `https://${networkName}-blocks.nymtech.net`,
        networkExplorer: `https://${networkName}-explorer.nymtech.net`,
      };

type TLoginType = 'mnemonic' | 'password';

/** `auth-splash`: full-screen auth-style loader (fallback). `app-overlay`: blurred in-app card (sign-in, account switch, sign-out). */
export type AppLoadingPresentation = 'auth-splash' | 'app-overlay';

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
  /** While `isLoading` is true, selects {@link LoadingPage} vs {@link AppSessionLoadingOverlay}. */
  loadingPresentation: AppLoadingPresentation;
  /** Short line shown on the in-app loading overlay. */
  loadingOverlayTitle: string;
  loadingOverlaySubtitle?: string;
  isAdminAddress: boolean;
  error?: string;
  loginType?: TLoginType;
  showSendModal: boolean;
  showReceiveModal: boolean;
  onAccountChange: ({ accountId, password }: { accountId: string; password: string }) => void;
  handleSwitchMode: () => void;
  handleShowSendModal: () => void;
  handleShowReceiveModal: () => void;
  handleCloseSendModal: () => void;
  handleCloseReceiveModal: () => void;
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
  mixnetContractParams?: TauriContractStateParams;
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
  /** Default dark avoids a light flash before forage restores a saved preference. */
  const [mode, setMode] = useState<'light' | 'dark'>('dark');
  const [loginType, setLoginType] = useState<'mnemonic' | 'password'>();
  const [isLoading, setIsLoadingInternal] = useState(false);
  const [loadingPresentation, setLoadingPresentation] = useState<AppLoadingPresentation>('auth-splash');
  const [loadingOverlayTitle, setLoadingOverlayTitle] = useState('');
  const [loadingOverlaySubtitle, setLoadingOverlaySubtitle] = useState<string | undefined>();
  const [error, setError] = useState<string>();

  /** Context-exposed setter: turning loading off also resets overlay copy and presentation to defaults. */
  const publishSetIsLoading = useCallback((loading: boolean) => {
    setIsLoadingInternal(loading);
    if (!loading) {
      setLoadingPresentation('auth-splash');
      setLoadingOverlayTitle('');
      setLoadingOverlaySubtitle(undefined);
    }
  }, []);
  const [appVersion, setAppVersion] = useState<string>();
  const [isAdminAddress, setIsAdminAddress] = useState<boolean>(false);
  const [showSendModal, setShowSendModal] = useState(false);
  const [showReceiveModal, setShowReceiveModal] = useState(false);
  const [printBalance, setPrintBalance] = useState<string>('-');
  const [printVestedBalance, setPrintVestedBalance] = useState<string | undefined>();
  const [mixnetContractParams, setMixnetContractParams] = useState<TauriContractStateParams>();

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
    publishSetIsLoading(false);
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

    getContractParams().then((params) => {
      setMixnetContractParams(params);
    });
  }, [appEnv, network, clientDetails?.client_address]);

  const logIn = async ({ type, value }: { type: TLoginType; value: string }) => {
    if (value.length === 0) {
      setError(`A ${type} must be provided`);
      return;
    }
    try {
      setLoadingPresentation('app-overlay');
      setLoadingOverlayTitle('Signing in');
      setLoadingOverlaySubtitle(
        type === 'mnemonic'
          ? 'Restoring your wallet from your recovery phrase.'
          : 'Unlocking your wallet and connecting to the network.',
      );
      setIsLoadingInternal(true);
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
      publishSetIsLoading(false);
    }
  };

  const logOut = async () => {
    setLoadingPresentation('app-overlay');
    setLoadingOverlayTitle('Signing out');
    setLoadingOverlaySubtitle('Closing your session safely.');
    setIsLoadingInternal(true);
    try {
      await signOut();
      await setReactState(undefined);
      setClientDetails(undefined);
      enqueueSnackbar('Successfully logged out', { variant: 'success' });
      await createSignInWindow();
    } finally {
      publishSetIsLoading(false);
    }
  };

  const onAccountChange = async ({ accountId, password }: { accountId: string; password: string }) => {
    if (network) {
      setLoadingPresentation('app-overlay');
      setLoadingOverlayTitle('Switching account');
      setLoadingOverlaySubtitle('Refreshing your wallet and balances.');
      setIsLoadingInternal(true);
      try {
        await switchAccount({ accountId, password });
        await loadAccount(network);
        enqueueSnackbar('Account switch success', { variant: 'success', preventDuplicate: true });
      } catch (e) {
        throw new Error(`Error swtiching account: ${e}`);
      } finally {
        publishSetIsLoading(false);
      }
    }
  };

  const handleShowAdmin = () => setShowAdmin((show) => !show);
  const handleShowTerminal = () => setShowTerminal((show) => !show);
  const switchNetwork = (_network: Network) => setNetwork(_network);
  const handleShowSendModal = () => setShowSendModal(true);
  const handleShowReceiveModal = () => setShowReceiveModal(true);
  const handleCloseSendModal = () => setShowSendModal(false);
  const handleCloseReceiveModal = () => setShowReceiveModal(false);
  const handleSwitchMode = () =>
    setMode((currentMode) => {
      const newMode = currentMode === 'light' ? 'dark' : 'light';
      setModeInStorage(newMode);
      return newMode;
    });

  const memoizedValue = useMemo(
    (): TAppContext => ({
      mode,
      appEnv,
      appVersion,
      isAdminAddress,
      isLoading,
      loadingPresentation,
      loadingOverlayTitle,
      loadingOverlaySubtitle,
      error,
      clientDetails,
      storedAccounts,
      mixnodeDetails,
      userBalance,
      showAdmin,
      showTerminal,
      network,
      loginType,
      setIsLoading: publishSetIsLoading,
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
      handleCloseSendModal,
      handleCloseReceiveModal,
      handleSwitchMode,
      printBalance,
      printVestedBalance,
      mixnetContractParams,
    }),
    [
      appVersion,
      loginType,
      isAdminAddress,
      mode,
      appEnv,
      isLoading,
      loadingPresentation,
      loadingOverlayTitle,
      loadingOverlaySubtitle,
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
      mixnetContractParams,
      publishSetIsLoading,
    ],
  );

  return <AppContext.Provider value={memoizedValue}>{children}</AppContext.Provider>;
};
