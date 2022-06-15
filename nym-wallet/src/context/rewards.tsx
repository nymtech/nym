import React, { createContext, FC, useContext, useEffect, useMemo, useState } from 'react';
import { Network } from 'src/types';
import { useDelegationContext } from './delegations';

type TRewardsContext = {
  isLoading: boolean;
  error?: string;
  totalRewards?: string;
  refresh: () => Promise<void>;
  redeemRewards: (mixnodeAddress: string) => Promise<TRewardsTransaction>;
  redeemAllRewards: () => Promise<TRewardsTransaction>;
};

export type TRewardsTransaction = {
  transactionUrl: string;
};

export const RewardsContext = createContext<TRewardsContext>({
  isLoading: true,
  refresh: async () => undefined,
  redeemRewards: async () => {
    throw new Error('Not implemented');
  },
  redeemAllRewards: async () => {
    throw new Error('Not implemented');
  },
});

export const RewardsContextProvider: FC<{
  network?: Network;
}> = ({ network, children }) => {
  const { isLoading, totalRewards, refresh } = useDelegationContext();
  const [currentNetwork, setCurrentNetwork] = useState<undefined | Network>();
  const [error, setError] = useState<string>();

  const resetState = async () => {
    setError(undefined);
  };

  useEffect(() => {
    if (currentNetwork !== network) {
      // reset state and refresh
      resetState();
      setCurrentNetwork(network);
    }
  }, [network]);

  const memoizedValue = useMemo(
    () => ({
      isLoading,
      error,
      totalRewards,
      refresh,
      redeemRewards: async () => {
        throw new Error('Not implemented');
      },
      redeemAllRewards: async () => {
        throw new Error('Not implemented');
      },
    }),
    [isLoading, error, totalRewards, network],
  );

  return <RewardsContext.Provider value={memoizedValue}>{children}</RewardsContext.Provider>;
};

export const useRewardsContext = () => useContext<TRewardsContext>(RewardsContext);
