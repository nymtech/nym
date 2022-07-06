import React, { FC, useMemo } from 'react';
import type { TAppContext } from '../main';
import { AppContext } from '../main';

export const MockMainContextProvider: FC<{}> = ({ children }) => {
  const memoizedValue = useMemo<TAppContext>(
    () => ({
      mode: 'light',
      appEnv: {
        ADMIN_ADDRESS: null,
        SHOW_TERMINAL: null,
        ENABLE_QA_MODE: null,
      },
      appVersion: 'mock',
      isAdminAddress: false,
      isLoading: false,
      clientDetails: {
        denom: 'NYMT',
        client_address: '',
        contract_address: '',
      },
      userBalance: {
        balance: {
          amount: {
            amount: '100',
            denom: 'NYMT',
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
      showAdmin: false,
      showTerminal: false,
      showSettings: false,
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
    }),
    [],
  );

  return <AppContext.Provider value={memoizedValue}>{children}</AppContext.Provider>;
};
