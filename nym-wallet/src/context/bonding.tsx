import { TransactionExecuteResult } from '@nymproject/types';
import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import type { Network } from 'src/types';
import { TBondGatewayArgs, TBondMixNodeArgs } from 'src/types';
import {
  bondGateway as bondGatewayRequest,
  bondMixNode as bondMixNodeRequest,
  claimOperatorRewards,
  compoundOperatorRewards,
  unbondGateway as unbondGatewayRequest,
  unbondMixNode as unbondMixNodeRequest,
} from '../requests';

export type TBondingContext = {
  isLoading: boolean;
  error?: string;
  bondedMixnode?: any; // TODO fix up type
  bondedGateway?: any; // TODO fix up type
  refresh: () => Promise<void>;
  bondMixnode: (data: TBondMixNodeArgs) => Promise<TransactionExecuteResult>;
  bondGateway: (data: TBondGatewayArgs) => Promise<TransactionExecuteResult>;
  unbondMixnode: () => Promise<TransactionExecuteResult>;
  unbondGateway: () => Promise<TransactionExecuteResult>;
  redeemRewards: () => Promise<TransactionExecuteResult>;
  compoundRewards: () => Promise<TransactionExecuteResult>;
};

export const BondingContext = createContext<TBondingContext>({
  isLoading: true,
  refresh: async () => undefined,
  bondMixnode: async () => {
    throw new Error('Not implemented');
  },
  bondGateway: async () => {
    throw new Error('Not implemented');
  },
  unbondMixnode: async () => {
    throw new Error('Not implemented');
  },
  unbondGateway: async () => {
    throw new Error('Not implemented');
  },
  redeemRewards: async () => {
    throw new Error('Not implemented');
  },
  compoundRewards: async () => {
    throw new Error('Not implemented');
  },
});

export const BondingContextProvider = ({
  network,
  children,
}: {
  network?: Network;
  children?: React.ReactNode;
}): JSX.Element => {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>();

  const refresh = async () => {
    throw new Error('Not implemented');
  };

  useEffect(() => {
    refresh();
  }, [network]);

  const bondMixnode = async (data: TBondMixNodeArgs) => {
    // TODO some logic
    return bondMixNodeRequest(data);
  };
  const bondGateway = async (data: TBondGatewayArgs) => {
    // TODO some logic
    return bondGatewayRequest(data);
  };
  const unbondMixnode = async () => {
    // TODO some logic
    return unbondMixNodeRequest();
  };
  const unbondGateway = async () => {
    // TODO some logic
    return unbondGatewayRequest();
  };
  const redeemRewards = async () => {
    // TODO some logic
    return claimOperatorRewards();
  };
  const compoundRewards = async () => {
    // TODO some logic
    return compoundOperatorRewards();
  };

  const memoizedValue = useMemo(
    () => ({
      isLoading,
      error,
      bondMixnode,
      bondGateway,
      unbondMixnode,
      unbondGateway,
      refresh,
      redeemRewards,
      compoundRewards,
    }),
    [isLoading, error],
  );

  return <BondingContext.Provider value={memoizedValue}>{children}</BondingContext.Provider>;
};

export const useBondingContext = () => useContext<TBondingContext>(BondingContext);
