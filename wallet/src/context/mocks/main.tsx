import { useMemo } from 'react';
import type { TAppContext } from '../main';
import { AppContext } from '../main';

export const MockMainContextProvider: FCWithChildren = ({ children }) => {
  const memoizedValue = useMemo<TAppContext>(
    () => ({
      mode: 'light',
      handleSwitchMode: () => undefined,
      appEnv: {
        ADMIN_ADDRESS: null,
        SHOW_TERMINAL: null,
        ENABLE_QA_MODE: null,
      },
      appVersion: 'mock',
      isAdminAddress: false,
      isLoading: false,
      clientDetails: {
        display_mix_denom: 'nymt',
        base_mix_denom: 'unymt',
        client_address: '',
      },
      userBalance: {
        balance: {
          amount: {
            amount: '100',
            denom: 'nymt',
          },
          printable_balance: '100 NYMT',
        },
        clearAll: () => undefined,
        isLoading: false,
        clearBalance: () => undefined,
        fetchBalance: async () => undefined,
        fetchTokenAllocation: async () => undefined,
        refreshBalances: async () => {},
      },
      displayDenom: 'NYM',
      showAdmin: false,
      showTerminal: false,
      showSettings: false,
      showSendModal: true,
      showReceiveModal: false,
      network: 'SANDBOX',
      loginType: 'mnemonic',
      setIsLoading: () => undefined,
      setError: () => undefined,
      signInWithPassword: () => undefined,
      switchNetwork: () => undefined,
      getBondDetails: async () => undefined,
      handleShowAdmin: () => undefined,
      handleShowTerminal: () => undefined,
      logIn: () => undefined,
      logOut: () => undefined,
      onAccountChange: () => undefined,
      handleShowSettings: () => undefined,
      handleShowSendModal: () => undefined,
      handleShowReceiveModal: () => undefined,
      keepState: async () => undefined,
      printBalance: '100.0000 NYMT',
    }),
    [],
  );

  return <AppContext.Provider value={memoizedValue}>{children}</AppContext.Provider>;
};
