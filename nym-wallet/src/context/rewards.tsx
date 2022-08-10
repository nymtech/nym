import React, { createContext, FC, useContext, useEffect, useMemo, useState } from 'react';
import { FeeDetails, TransactionExecuteResult } from '@nymproject/types';
import { useDelegationContext } from './delegations';
import { claimDelegatorRewards, compoundDelegatorRewards } from '../requests';

type TRewardsContext = {
  isLoading: boolean;
  error?: string;
  totalRewards?: string;
  refresh: () => Promise<void>;
  claimRewards: (identity: string, fee?: FeeDetails) => Promise<TransactionExecuteResult[]>;
  compoundRewards: (identity: string, fee?: FeeDetails) => Promise<TransactionExecuteResult[]>;
};

export type TRewardsTransaction = {
  transactionUrl: string;
  transactionHash: string;
};

export const RewardsContext = createContext<TRewardsContext>({
  isLoading: true,
  refresh: async () => undefined,
  claimRewards: async () => {
    throw new Error('Not implemented');
  },
  compoundRewards: async () => {
    throw new Error('Not implemented');
  },
});

export const RewardsContextProvider: FC<{}> = ({ children }) => {
  const { isLoading, totalRewards, refresh } = useDelegationContext();
  const [error, setError] = useState<string>();

  const resetState = async () => {
    setError(undefined);
  };

  useEffect(() => {
    resetState();
  }, []);

  const memoizedValue = useMemo(
    () => ({
      isLoading,
      error,
      totalRewards,
      refresh,
      claimRewards: claimDelegatorRewards,
      compoundRewards: compoundDelegatorRewards,
      redeemAllRewards: async () => {
        throw new Error('Not implemented');
      },
    }),
    [isLoading, error, totalRewards],
  );

  return <RewardsContext.Provider value={memoizedValue}>{children}</RewardsContext.Provider>;
};

export const useRewardsContext = () => useContext<TRewardsContext>(RewardsContext);
