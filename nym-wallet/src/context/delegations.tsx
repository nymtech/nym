import React, { createContext, FC, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { getDelegationSummary, undelegateFromMixnode } from 'src/requests/delegation';
import {
  DecCoin,
  DelegationWithEverything,
  Fee,
  FeeDetails,
  TransactionExecuteResult,
  WrappedDelegationEvent,
} from '@nymproject/types';
import type { Network } from 'src/types';
import {
  delegateToMixnode,
  getAllPendingDelegations,
  vestingDelegateToMixnode,
  vestingUndelegateFromMixnode,
} from 'src/requests';
import { TPoolOption } from 'src/components';
import { decCoinToDisplay } from 'src/utils';
import { Console } from 'src/utils/console';
import { AppContext } from 'src/context/main';

export type TDelegationRefreshOptions = {
  /** When true, do not flip the global loading state (keeps cached list visible during refetch). */
  background?: boolean;
};

export type TDelegationContext = {
  delegationItemErrors?: { nodeId: string; errors: string };
  isLoading: boolean;
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

export const DelegationContextProvider: FC<{
  network?: Network;
  children: React.ReactNode;
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
}> = ({ network, children }) => {
  const location = useLocation();
  const { clientDetails } = useContext(AppContext);
  const clientAddress = clientDetails?.client_address;
  const [isLoading, setIsLoading] = useState(false);
  const [delegationItemErrors, setDelegationItemErrors] = useState<{ nodeId: string; errors: string }>();
  const [delegations, setDelegations] = useState<undefined | TDelegations>();
  const [totalDelegations, setTotalDelegations] = useState<undefined | string>();
  const [totalRewards, setTotalRewards] = useState<undefined | string>();
  const [totalDelegationsAndRewards, setTotalDelegationsAndRewards] = useState<undefined | string>();
  const [pendingDelegations, setPendingDelegations] = useState<WrappedDelegationEvent[]>();
  const delegationsRef = useRef<TDelegations | undefined>(undefined);
  delegationsRef.current = delegations;

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

  const refresh = useCallback(async (opts?: TDelegationRefreshOptions) => {
    if (!opts?.background) {
      setIsLoading(true);
    }
    try {
      const data = await getDelegationSummary();
      const pending = await getAllPendingDelegations();

      const pendingOnNewNodes = pending.filter((event) => {
        const some = data.delegations.some(({ node_identity }) => node_identity === event.node_identity);
        return !some;
      });
      const items = data.delegations.map((delegation) => ({
        ...delegation,
        amount: decCoinToDisplay(delegation.amount),
        unclaimed_rewards: delegation.unclaimed_rewards && decCoinToDisplay(delegation.unclaimed_rewards),
        cost_params: delegation.cost_params && {
          ...delegation.cost_params,
          interval_operating_cost: decCoinToDisplay(delegation.cost_params.interval_operating_cost),
        },
      }));

      const delegationsAndRewards = (+data.total_delegations.amount + +data.total_rewards.amount).toFixed(6);

      setPendingDelegations(pending);
      setDelegations([...items, ...pendingOnNewNodes]);
      setTotalDelegations(`${data.total_delegations.amount} ${data.total_delegations.denom}`);
      setTotalRewards(`${data.total_rewards.amount} ${data.total_rewards.denom}`);
      setTotalDelegationsAndRewards(`${delegationsAndRewards} ${data.total_delegations.denom}`);
    } catch (e) {
      Console.error(e);
    }
    setIsLoading(false);
  }, []);

  useEffect(() => {
    if (!clientAddress) {
      setDelegations(undefined);
      setPendingDelegations(undefined);
      setTotalDelegations(undefined);
      setTotalRewards(undefined);
      setTotalDelegationsAndRewards(undefined);
      setIsLoading(false);
    }
  }, [clientAddress]);

  useEffect(() => {
    if (!clientAddress) {
      return;
    }
    const onDelegationRoute = location.pathname === '/delegation' || location.pathname.endsWith('/delegation');
    if (!onDelegationRoute) {
      return;
    }
    const hasCache = delegationsRef.current !== undefined;
    refresh(hasCache ? { background: true } : undefined).catch((err) => {
      Console.error(err);
    });
  }, [clientAddress, location.pathname, refresh]);

  const memoizedValue = useMemo(
    () => ({
      delegationItemErrors,
      isLoading,
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
      isLoading,
      delegations,
      delegationItemErrors,
      pendingDelegations,
      totalDelegations,
      totalRewards,
      totalDelegationsAndRewards,
      refresh,
    ],
  );

  return <DelegationContext.Provider value={memoizedValue}>{children}</DelegationContext.Provider>;
};

export const useDelegationContext = () => useContext<TDelegationContext>(DelegationContext);
