import React, { createContext, FC, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { getDelegationSummary, undelegateFromMixnode } from 'src/requests/delegation';
import {
  DelegationEvent,
  DelegationWithEverything,
  MajorCurrencyAmount,
  TransactionExecuteResult,
} from '@nymproject/types';
import type { Network } from 'src/types';
import { delegateToMixnode, getAllPendingDelegations } from 'src/requests';

export type TDelegationContext = {
  isLoading: boolean;
  error?: string;
  delegations?: DelegationWithEverything[];
  pendingDelegations?: DelegationEvent[];
  totalDelegations?: string;
  totalRewards?: string;
  refresh: () => Promise<void>;
  addDelegation: (data: { identity: string; amount: MajorCurrencyAmount }) => Promise<TransactionExecuteResult>;
  updateDelegation: (newDelegation: DelegationWithEverything) => Promise<TDelegationTransaction>;
  undelegate: (identity: string) => Promise<TransactionExecuteResult>;
};

export type TDelegationTransaction = {
  transactionUrl: string;
};

export const DelegationContext = createContext<TDelegationContext>({
  isLoading: true,
  refresh: async () => undefined,
  addDelegation: async () => {
    throw new Error('Not implemented');
  },
  updateDelegation: async () => {
    throw new Error('Not implemented');
  },
  undelegate: async () => {
    throw new Error('Not implemented');
  },
});

export const DelegationContextProvider: FC<{
  network?: Network;
}> = ({ network, children }) => {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [delegations, setDelegations] = useState<undefined | DelegationWithEverything[]>();
  const [totalDelegations, setTotalDelegations] = useState<undefined | string>();
  const [totalRewards, setTotalRewards] = useState<undefined | string>();
  const [pendingDelegations, setPendingDelegations] = useState<DelegationEvent[]>();

  const addDelegation = async (data: { identity: string; amount: MajorCurrencyAmount }) => {
    const tx = await delegateToMixnode(data);
    await refresh();
    return tx;
  };

  const updateDelegation = async (): Promise<TDelegationTransaction> => {
    throw new Error('Not implemented');
  };

  const undelegate = async (identity: string) => {
    const delegationResult = await undelegateFromMixnode(identity);
    await refresh();
    return delegationResult;
  };

  const resetState = () => {
    setIsLoading(true);
    setError(undefined);
    setTotalDelegations(undefined);
    setTotalRewards(undefined);
    setDelegations([]);
  };

  const refresh = useCallback(async () => {
    try {
      const data = await getDelegationSummary();
      const pending = await getAllPendingDelegations();
      setPendingDelegations(pending);
      setDelegations(data.delegations);
      setTotalDelegations(`${data.total_delegations.amount} ${data.total_delegations.denom}`);
      setTotalRewards(`${data.total_rewards.amount} ${data.total_rewards.denom}`);
    } catch (e) {
      setError((e as Error).message);
    }
    setIsLoading(false);
  }, [network]);

  useEffect(() => {
    // reset state and refresh
    resetState();
    refresh();
  }, [network]);

  const memoizedValue = useMemo(
    () => ({
      isLoading,
      error,
      delegations,
      pendingDelegations,
      totalDelegations,
      totalRewards,
      refresh,
      addDelegation,
      updateDelegation,
      undelegate,
    }),
    [isLoading, error, delegations, pendingDelegations, totalDelegations],
  );

  return <DelegationContext.Provider value={memoizedValue}>{children}</DelegationContext.Provider>;
};

export const useDelegationContext = () => useContext<TDelegationContext>(DelegationContext);
