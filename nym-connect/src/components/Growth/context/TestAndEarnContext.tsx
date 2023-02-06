/* eslint-disable @typescript-eslint/naming-convention */
import React, { createContext, useCallback, useContext, useMemo, useState } from 'react';
import { forage } from '@tauri-apps/tauri-forage';
import { invoke } from '@tauri-apps/api';
import { ClientId, DrawEntry, Draws, Registration } from './types';
import { useClientContext } from '../../../context/main';
import { ConnectionStatusKind } from '../../../types';

export type TTestAndEarnContext = {
  loadedOnce: boolean;
  loading: boolean;
  clientDetails?: ClientId;
  registration?: Registration;
  walletAddress?: string;
  draws?: Draws;
  isWinnerWithUnclaimedPrize?: boolean;
  isEnterWallet?: boolean;
  error?: string;
  setWalletAddress: (newWalletAddress: string) => void;
  clearStorage: () => Promise<void>;
  toggleGrowthWindow: (windowTitle?: string) => Promise<void>;
  setAndStoreClientId: (newClientId: ClientId) => Promise<void>;
  setAndStoreRegistration: (registration: Registration) => Promise<void>;
  enterDraw: (drawId: string) => Promise<DrawEntry>;
  claim: (drawId: string, walletAddress: string) => Promise<void>;
  refresh: () => Promise<void>;
};

const defaultValue: TTestAndEarnContext = {
  loadedOnce: false,
  loading: true,
  setWalletAddress: () => undefined,
  clearStorage: async () => undefined,
  toggleGrowthWindow: async () => undefined,
  setAndStoreRegistration: async () => undefined,
  setAndStoreClientId: async () => undefined,
  enterDraw: async () => ({} as DrawEntry),
  claim: async () => undefined,
  refresh: async () => undefined,
};

export const TestAndEarnContext = createContext(defaultValue);

const CLIENT_ID_KEY = 'tne_client_id';
const REGISTRATION_KEY = 'tne_registration';

export const TestAndEarnContextProvider: FCWithChildren = ({ children }) => {
  const clientContext = useClientContext();
  const [loadedOnce, setLoadedOnce] = useState(false);
  const [loading, setLoading] = useState(true);
  const [walletAddress, setWalletAddress] = useState<string>();
  const [registration, setRegistration] = useState<Registration>();
  const [clientDetails, setClientDetails] = useState<ClientId>();
  const [draws, setDraws] = useState<Draws>();

  const setAndStoreClientId = async (newClientId: ClientId) => {
    await forage.setItem({ key: CLIENT_ID_KEY, value: newClientId } as any)();
    setClientDetails((prevState) => {
      if (
        prevState?.client_id !== newClientId.client_id ||
        prevState?.client_id_signature !== newClientId.client_id_signature
      ) {
        console.log('Setting client details');
        return newClientId;
      }
      console.log('Skipping client details');
      return prevState;
    });
  };
  const loadClientDetails = async () => {
    const data: ClientId | undefined = await forage.getItem({ key: CLIENT_ID_KEY })();
    if (data) {
      try {
        setClientDetails((prevState) => {
          if (prevState?.client_id !== data.client_id || prevState?.client_id_signature !== data.client_id_signature) {
            console.log('Setting client details');
            return data;
          }
          console.log('Skipping client details');
          return prevState;
        });
      } catch (e) {
        console.error('Failed to get registration');
      }
    } else {
      const clientId: ClientId = await invoke('growth_tne_get_client_id');
      await setAndStoreClientId(clientId);
    }
  };

  const loadRegistration = async () => {
    const data: Registration | undefined = await forage.getItem({ key: REGISTRATION_KEY })();
    if (data) {
      try {
        setRegistration((prevState) => {
          if (
            prevState?.timestamp !== data.timestamp ||
            prevState.client_id_signature !== data.client_id_signature ||
            prevState.id !== data.id
          ) {
            console.log('Setting registration');
            return data;
          }
          console.log('Skipping registration');
          return prevState;
        });
      } catch (e) {
        console.error('Failed to get registration');
      }
    }
  };

  const loadDraws = React.useCallback(async () => {
    setLoading(true);
    let clientDetailsForDraws = clientDetails;
    try {
      if (!clientDetailsForDraws) {
        console.log('[loadDraws] client details not set, trying to get...');
        clientDetailsForDraws = await invoke('growth_tne_get_client_id');
      }

      if (!clientDetailsForDraws) {
        console.log('[loadDraws] failed to get client details not set, skipping...');
        setLoading(false);
        setLoadedOnce(true);
        return;
      }

      const newDraws: Draws = await invoke('growth_tne_get_draws', { clientDetails: clientDetailsForDraws });
      console.log('[loadDraws] draws = ', newDraws);

      // find the entered draw and keep a reference
      const entered = newDraws.draws.find((draw) => draw.draw_id === newDraws.current?.id);
      if (newDraws.current) {
        newDraws.current.entry = entered;
      }

      console.log('[loadDraws] setting draws');
      setDraws(newDraws);
    } catch (e) {
      console.error('Could not get draws: ', e);
    }
    setLoading(false);
    setLoadedOnce(true);
    console.log('[loadDraws] done, loaded once');
  }, [clientDetails]);

  React.useEffect(() => {
    loadClientDetails().catch(console.error);
    loadRegistration().catch(console.error);
  }, []);

  React.useEffect(() => {
    if (registration && clientContext.connectionStatus === 'connected') {
      setTimeout(() => {
        loadDraws().catch(console.error);
      }, 1000 * 3);
    }
  }, [registration?.id, registration?.timestamp, clientContext.connectionStatus]);

  const refresh = React.useCallback(async () => {
    console.log('Refreshing...');

    console.log('Loading client details...');
    await loadClientDetails();

    console.log('Loading registration...');
    await loadRegistration();

    console.log('Loading draws...');
    await loadDraws();

    console.log('Refresh complete.');
  }, [clientDetails]);

  const clearStorage = async () => {
    await forage.setItem({ key: REGISTRATION_KEY, value: undefined })();
  };

  const toggleGrowthWindow = useCallback(async (windowTitle?: string) => {
    try {
      await invoke('growth_tne_toggle_window', { windowTitle });
    } catch (e) {
      console.error('Failed to toggle growth window', e);
    }
  }, []);

  const setAndStoreRegistration = async (newRegistration: Registration) => {
    await forage.setItem({ key: REGISTRATION_KEY, value: newRegistration } as any)();
    setRegistration(newRegistration);
  };

  const enterDraw = async (drawId: string): Promise<DrawEntry> => {
    if (!clientDetails) {
      throw new Error('No client details set');
    }
    if (!draws) {
      throw new Error('No draws set');
    }

    const existingEntry: DrawEntry | undefined = draws.draws.filter((d) => d.draw_id === drawId)[0];
    if (existingEntry) {
      throw new Error('Already entered into draw');
    }

    const entry: DrawEntry = await invoke('growth_tne_enter_draw', { clientDetails, drawId });
    console.log('Entered draw', { entry });

    await loadDraws();

    return entry;
  };

  const claim = async (drawId: string, newWalletAddress: string) => {
    if (!clientDetails) {
      throw new Error('No client details set');
    }
    if (!draws) {
      throw new Error('No draws set');
    }
    if (!registration) {
      throw new Error('No registration set');
    }

    const registrationId = registration.id;

    const args = {
      registrationId,
      clientDetails,
      drawId,
      walletAddress: newWalletAddress,
    };
    console.log({ args });
    await invoke('growth_tne_submit_wallet_address', args);

    await loadDraws();
  };

  const contextValue = useMemo<TTestAndEarnContext>(
    () => ({
      loadedOnce,
      loading,
      clientDetails,
      registration,
      walletAddress,
      draws,
      clearStorage,
      toggleGrowthWindow,
      setWalletAddress,
      setAndStoreClientId,
      setAndStoreRegistration,
      enterDraw,
      refresh,
      claim,
    }),
    [
      loadedOnce,
      loading,
      walletAddress,
      registration,
      refresh,
      draws,
      draws?.current?.last_modified,
      draws?.current?.entry,
      draws?.draws.length,
      clientDetails,
    ],
  );
  return <TestAndEarnContext.Provider value={contextValue}>{children}</TestAndEarnContext.Provider>;
};

export const useTestAndEarnContext = () => useContext(TestAndEarnContext);
