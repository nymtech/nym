import React, { createContext, FC, useCallback, useContext, useEffect, useMemo } from 'react';
import { useLocation } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { undelegateFromMixnode } from 'src/requests/delegation';
import {
  DecCoin,
  DelegationWithEverything,
  Fee,
  FeeDetails,
  TransactionExecuteResult,
  WrappedDelegationEvent,
} from '@nymproject/types';
import type { Network } from 'src/types';
import { delegateToMixnode, vestingDelegateToMixnode, vestingUndelegateFromMixnode } from 'src/requests';
import { TPoolOption } from 'src/components';
import { Console } from 'src/utils/console';
import { AppContext } from 'src/context/main';
import { delegationQueryKeys, fetchDelegationSummaryQuery } from './delegationQuery';

export type TDelegationRefreshOptions = {
  /** When true, do not flip the global loading state (keeps cached list visible during refetch). */
  background?: boolean;
};

export type TDelegationContext = {
  delegationItemErrors?: { nodeId: string; errors: string };
  isLoading: boolean;
  isFetching: boolean;
  isError: boolean;
  lastUpdatedAtMs: number;
  delegations?: TDelegations;
  pendingDelegations?: WrappedDelegationEvent[];
  totalDelegations?: string;
  totalRewards?: string;
  totalDelegationsAndRewards?: string;
  refresh: (opts?: TDelegationRefreshOptions) => Promise<void>;
  addDelegation: (
    data: { mix_id: number; amount: DecCoin },
    tokenPool: TPoolOption,
    fee?: FeeDetails,
  ) => Promise<TransactionExecuteResult>;
  undelegate: (mix_id: number, fee?: Fee) => Promise<TransactionExecuteResult>;
  undelegateVesting: (mix_id: number) => Promise<TransactionExecuteResult>;
  setDelegationItemErrors: (data: { nodeId: string; errors: string } | undefined) => void;
};

export type TDelegationTransaction = {
  transactionUrl: string;
};

export type DelegationWithEvent = DelegationWithEverything | WrappedDelegationEvent;
export type TDelegation = DelegationWithEvent;
export type TDelegations = TDelegation[];

export const isPendingDelegation = (delegation: DelegationWithEvent): delegation is WrappedDelegationEvent =>
  'event' in delegation;
export const isDelegation = (delegation: DelegationWithEvent): delegation is DelegationWithEverything =>
  'owner' in delegation;

export const DelegationContext = createContext<TDelegationContext>({
  isLoading: false,
  isFetching: false,
  isError: false,
  lastUpdatedAtMs: 0,
  refresh: async (_opts?: TDelegationRefreshOptions) => undefined,
  addDelegation: async () => {
    throw new Error('Not implemented');
  },
  undelegate: () => {
    throw new Error('Not implemented');
  },
  undelegateVesting: () => {
    throw new Error('Not implemented');
  },
  setDelegationItemErrors: () => undefined,
});

function isDelegationRoutePath(pathname: string): boolean {
  return pathname === '/delegation' || pathname.endsWith('/delegation');
}

export const DelegationContextProvider: FC<{
  network?: Network;
  children: React.ReactNode;
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
}> = ({ network, children }) => {
  const location = useLocation();
  const queryClient = useQueryClient();
  const { clientDetails } = useContext(AppContext);
  const clientAddress = clientDetails?.client_address;
  const onDelegationRoute = isDelegationRoutePath(location.pathname);

  const [delegationItemErrors, setDelegationItemErrors] = React.useState<{ nodeId: string; errors: string }>();

  const query = useQuery({
    queryKey: delegationQueryKeys.summary(clientAddress ?? ''),
    queryFn: fetchDelegationSummaryQuery,
    enabled: Boolean(clientAddress) && onDelegationRoute,
    staleTime: 5 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
  });

  useEffect(() => {
    if (!clientAddress) {
      queryClient.removeQueries({ queryKey: delegationQueryKeys.all });
    }
  }, [clientAddress, queryClient]);

  const bundle = clientAddress && onDelegationRoute ? query.data : undefined;

  const refresh = useCallback(
    async (_opts?: TDelegationRefreshOptions) => {
      if (!clientAddress) {
        return;
      }
      await queryClient.invalidateQueries({
        queryKey: delegationQueryKeys.summary(clientAddress),
      });
    },
    [clientAddress, queryClient],
  );

  const addDelegation = async (data: { mix_id: number; amount: DecCoin }, tokenPool: TPoolOption, fee?: FeeDetails) => {
    try {
      let tx;

      if (tokenPool === 'locked') {
        tx = await vestingDelegateToMixnode(data.mix_id, data.amount, fee?.fee);
      } else {
        tx = await delegateToMixnode(data.mix_id, data.amount, fee?.fee);
      }

      return tx;
    } catch (e) {
      throw new Error(e as string);
    }
  };

  const delegations = bundle?.delegations;
  const pendingDelegations = bundle?.pendingDelegations;
  const totalDelegations = bundle?.totalDelegations;
  const totalRewards = bundle?.totalRewards;
  const totalDelegationsAndRewards = bundle?.totalDelegationsAndRewards;

  const isLoading = Boolean(clientAddress) && onDelegationRoute && query.isPending;
  const isFetching = Boolean(clientAddress) && onDelegationRoute && query.isFetching;
  const isError = Boolean(clientAddress) && onDelegationRoute && query.isError && !query.data;
  const lastUpdatedAtMs = bundle ? query.dataUpdatedAt : 0;

  const memoizedValue = useMemo(
    () => ({
      delegationItemErrors,
      isLoading,
      isFetching,
      isError,
      lastUpdatedAtMs,
      delegations,
      pendingDelegations,
      totalDelegations,
      totalRewards,
      totalDelegationsAndRewards,
      refresh,
      setDelegationItemErrors,
      addDelegation,
      undelegate: undelegateFromMixnode,
      undelegateVesting: vestingUndelegateFromMixnode,
    }),
    [
      delegationItemErrors,
      isLoading,
      isFetching,
      isError,
      lastUpdatedAtMs,
      delegations,
      pendingDelegations,
      totalDelegations,
      totalRewards,
      totalDelegationsAndRewards,
      refresh,
    ],
  );

  useEffect(() => {
    if (query.isError && query.error) {
      Console.error(query.error);
    }
  }, [query.isError, query.error]);

  return <DelegationContext.Provider value={memoizedValue}>{children}</DelegationContext.Provider>;
};

export const useDelegationContext = () => useContext<TDelegationContext>(DelegationContext);
