import React, { createContext, FC, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { getDelegationSummary, undelegateAllFromMixnode } from 'src/requests/delegation';
import {
  DelegationEvent,
  DelegationWithEverything,
  FeeDetails,
  DecCoin,
  TransactionExecuteResult,
} from '@nymproject/types';
import type { Network } from 'src/types';
import { delegateToMixnode, getAllPendingDelegations, vestingDelegateToMixnode } from 'src/requests';
import { TPoolOption } from 'src/components';

export type TDelegationContext = {
  isLoading: boolean;
  error?: string;
  delegations?: TDelegations;
  pendingDelegations?: DelegationEvent[];
  totalDelegations?: string;
  totalRewards?: string;
  refresh: () => Promise<void>;
  addDelegation: (
    data: { identity: string; amount: DecCoin },
    tokenPool: TPoolOption,
    fee?: FeeDetails,
  ) => Promise<TransactionExecuteResult>;
  undelegate: (
    identity: string,
    usesVestingContractTokens: boolean,
    fee?: FeeDetails,
  ) => Promise<TransactionExecuteResult[]>;
};

export type TDelegationTransaction = {
  transactionUrl: string;
};

export type DelegationWithEvent = DelegationWithEverything | DelegationEvent;
export type TDelegations = DelegationWithEvent[];

export const isPendingDelegation = (delegation: DelegationWithEvent): delegation is DelegationEvent =>
  'kind' in delegation;

export const DelegationContext = createContext<TDelegationContext>({
  isLoading: true,
  refresh: async () => undefined,
  addDelegation: async () => {
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
  const [delegations, setDelegations] = useState<undefined | TDelegations>();
  const [totalDelegations, setTotalDelegations] = useState<undefined | string>();
  const [totalRewards, setTotalRewards] = useState<undefined | string>();
  const [pendingDelegations, setPendingDelegations] = useState<DelegationEvent[]>();

  const addDelegation = async (
    data: { identity: string; amount: DecCoin },
    tokenPool: TPoolOption,
    fee?: FeeDetails,
  ) => {
    try {
      let tx;

      if (tokenPool === 'locked') tx = await vestingDelegateToMixnode({ ...data, fee });
      else tx = await delegateToMixnode(data);

      return tx;
    } catch (e) {
      throw new Error(e as string);
    }
  };

  const resetState = () => {
    setError(undefined);
    setTotalDelegations(undefined);
    setTotalRewards(undefined);
    setDelegations([]);
  };

  const refresh = useCallback(async () => {
    resetState();
    setIsLoading(true);
    try {
      const data = await getDelegationSummary();
      const pending = await getAllPendingDelegations();

      const pendingOnNewNodes = pending.filter((event) => {
        const some = data.delegations.some(({ node_identity }) => node_identity === event.node_identity);
        return !some;
      });

      setPendingDelegations(pending);
      setDelegations([...data.delegations, ...pendingOnNewNodes]);
      setTotalDelegations(`${data.total_delegations.amount} ${data.total_delegations.denom}`);
      setTotalRewards(`${data.total_rewards.amount} ${data.total_rewards.denom}`);
    } catch (e) {
      setError((e as Error).message);
    }
    setIsLoading(false);
  }, []);

  useEffect(() => {
    refresh();
  }, []);

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
      undelegate: undelegateAllFromMixnode,
    }),
    [isLoading, error, delegations, pendingDelegations, totalDelegations],
  );

  return <DelegationContext.Provider value={memoizedValue}>{children}</DelegationContext.Provider>;
};

export const useDelegationContext = () => useContext<TDelegationContext>(DelegationContext);
