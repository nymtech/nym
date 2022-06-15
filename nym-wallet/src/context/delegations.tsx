import React, { createContext, FC, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { getDelegationSummary, undelegateFromMixnode } from 'src/requests/delegation';
import {
  DelegationEvent,
  DelegationWithEverything,
  MajorCurrencyAmount,
  TransactionExecuteResult,
} from '@nymproject/types';
import type { Network } from 'src/types';
import {
  claimDelegatorRewards,
  compoundDelegatorRewards,
  delegateToMixnode,
  getAllPendingDelegations,
  vestingClaimDelegatorRewards,
  vestingCompoundDelegatorRewards,
  vestingDelegateToMixnode,
  vestingUndelegateFromMixnode,
} from 'src/requests';
import { TPoolOption } from 'src/components';

export type TDelegationContext = {
  isLoading: boolean;
  error?: string;
  delegations?: DelegationWithEverything[];
  pendingDelegations?: DelegationEvent[];
  totalDelegations?: string;
  totalRewards?: string;
  refresh: () => Promise<void>;
  addDelegation: (
    data: { identity: string; amount: MajorCurrencyAmount },
    tokenPool: TPoolOption,
  ) => Promise<TransactionExecuteResult>;
  undelegate: (identity: string, proxy: string | null) => Promise<TransactionExecuteResult>;
  redeemRewards: (identity: string, proxy: string | null) => Promise<TransactionExecuteResult>;
  compoundRewards: (identity: string, proxy: string | null) => Promise<TransactionExecuteResult>;
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
  undelegate: async () => {
    throw new Error('Not implemented');
  },
  redeemRewards: async () => {
    throw new Error('Not implemented');
  },
  compoundRewards: async () => {
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

  const addDelegation = async (data: { identity: string; amount: MajorCurrencyAmount }, tokenPool: TPoolOption) => {
    try {
      let tx;

      if (tokenPool === 'locked') tx = await vestingDelegateToMixnode(data);
      else tx = await delegateToMixnode(data);

      return tx;
    } catch (e) {
      throw new Error(e as string);
    }
  };

  const undelegate = async (identity: string, proxy: string | null) => {
    let delegationResult;
    try {
      if ((proxy || '').trim().length === 0) {
        // the owner of the delegation is main account (the owner of the vesting account), so it is delegation with unlocked tokens
        delegationResult = await undelegateFromMixnode(identity);
      } else {
        // the delegation is with locked tokens, so use the vesting contract
        delegationResult = await vestingUndelegateFromMixnode(identity);
      }
      return delegationResult;
    } catch (e) {
      throw new Error(e as string);
    }
  };

  const redeemRewards = async (identity: string, proxy: string | null) => {
    try {
      if ((proxy || '').trim().length === 0) {
        // the owner of the delegation is main account (the owner of the vesting account), so it is delegation with unlocked tokens
        return claimDelegatorRewards(identity);
      } else {
        // the delegation is with locked tokens, so use the vesting contract
        return vestingClaimDelegatorRewards(identity);
      }
    } catch (e) {
      throw new Error(e as string);
    }
  };

  const compoundRewards = async (identity: string, proxy: string | null) => {
    try {
      if ((proxy || '').trim().length === 0) {
        // the owner of the delegation is main account (the owner of the vesting account), so it is delegation with unlocked tokens
        return compoundDelegatorRewards(identity);
      } else {
        // the delegation is with locked tokens, so use the vesting contract
        return vestingCompoundDelegatorRewards(identity);
      }
    } catch (e) {
      throw new Error(e as string);
    }
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
      undelegate,
      redeemRewards,
      compoundRewards,
    }),
    [isLoading, error, delegations, pendingDelegations, totalDelegations],
  );

  return <DelegationContext.Provider value={memoizedValue}>{children}</DelegationContext.Provider>;
};

export const useDelegationContext = () => useContext<TDelegationContext>(DelegationContext);
