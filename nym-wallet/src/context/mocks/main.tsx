import React, { useMemo } from 'react';
import type { TAppContext } from '../main';
import { AppContext } from '../main';

export const MockMainContextProvider: FCWithChildren = ({ children }) => {
  const memoizedValue = useMemo<TAppContext>(
    () => ({
      mode: 'dark',
      handleSwitchMode: () => undefined,
      appEnv: {
        ADMIN_ADDRESS: null,
        SHOW_TERMINAL: null,
        ENABLE_QA_MODE: null,
      },
      appVersion: 'mock',
      isAdminAddress: false,
      isLoading: false,
      loadingPresentation: 'auth-splash',
      loadingOverlayTitle: '',
      loadingOverlaySubtitle: undefined,
      clientDetails: {
        display_mix_denom: 'nymt',
        base_mix_denom: 'unymt',
        client_address: '',
      },
      storedAccounts: undefined,
      mixnodeDetails: null,
      userBalance: {
        balance: {
          amount: {
            amount: '100',
            denom: 'nymt',
          },
          printable_balance: '100 NYMT',
        },
        error: undefined,
        tokenAllocation: undefined,
        originalVesting: undefined,
        currentVestingPeriod: undefined,
        vestingAccountInfo: undefined,
        clearAll: () => undefined,
        isLoading: false,
        clearBalance: () => undefined,
        fetchBalance: async () => undefined,
        fetchTokenAllocation: async () => undefined,
        refreshBalances: async () => undefined,
      },
      showAdmin: false,
      showTerminal: false,
      showSendModal: false,
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
      handleShowSendModal: () => undefined,
      handleShowReceiveModal: () => undefined,
      handleCloseSendModal: () => undefined,
      handleCloseReceiveModal: () => undefined,
      keepState: async () => undefined,
      printBalance: '100.0000 NYMT',
      printVestedBalance: undefined,
      mixnetContractParams: undefined,
    }),
    [],
  );

  return <AppContext.Provider value={memoizedValue}>{children}</AppContext.Provider>;
};
