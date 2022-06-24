import { MajorCurrencyAmount, MixnodeStatus, TransactionExecuteResult } from '@nymproject/types';
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

const bounded: BondedMixnode = {
  bond: { denom: 'NYM', amount: '1234' },
  key: 'B2Xx4haarLWMajX8w259oHjtRZsC7nHwagbWrJNiA3QC',
  delegators: 123,
  ip: '1.2.34.5',
  nodeRewards: { denom: 'NYM', amount: '123' },
  operatorRewards: { denom: 'NYM', amount: '12' },
  profitMargin: 10,
  stake: { denom: 'NYM', amount: '99' },
  stakeSaturation: 99,
  status: 'active',
};

/* const bounded: BondedMixnode | BondedGateway = {
  bond: { denom: 'NYM', amount: '1234' },
  key: 'B2Xx4haarLWMajX8w259oHjtRZsC7nHwagbWrJNiA3QC',
  ip: '1.2.34.5',
  location: 'France',
}; */

// TODO temporary type for bonded mixnode data
export interface BondedMixnode {
  key: string;
  ip: string;
  stake: MajorCurrencyAmount;
  bond: MajorCurrencyAmount;
  stakeSaturation: number;
  profitMargin: number;
  nodeRewards: MajorCurrencyAmount;
  operatorRewards: MajorCurrencyAmount;
  delegators: number;
  status: MixnodeStatus;
}

// TODO temporary type for bonded gateway data
export interface BondedGateway {
  key: string;
  ip: string;
  bond: MajorCurrencyAmount;
  location?: string; // TODO not yet available, only available in Network Explorer API
}

export type TBondingContext = {
  isLoading: boolean;
  error?: string;
  bondedMixnode?: BondedMixnode | null;
  bondedGateway?: BondedGateway | null;
  refresh: () => Promise<void>;
  bondMixnode: (data: TBondMixNodeArgs) => Promise<TransactionExecuteResult>;
  bondGateway: (data: TBondGatewayArgs) => Promise<TransactionExecuteResult>;
  unbondMixnode: () => Promise<TransactionExecuteResult>;
  unbondGateway: () => Promise<TransactionExecuteResult>;
  redeemRewards: () => Promise<TransactionExecuteResult[]>;
  compoundRewards: () => Promise<TransactionExecuteResult[]>;
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
  const [bondedMixnode, setBondedMixnode] = useState<BondedMixnode | null>(null);
  const [bondedGateway, setBondedGateway] = useState<BondedGateway | null>(null);

  const resetState = () => {
    setIsLoading(true);
    setError(undefined);
    setBondedGateway(null);
    setBondedMixnode(null);
  };

  const refresh = useCallback(async () => {
    // TODO fetch bondedMixnode and bondedGatway via tauri dedicated requests
    /* try {
      bounded = await fetchBondingData();
    } catch (e: any) {
      throw new Error(e);
    } */
    if (bounded && 'stake' in bounded) {
      setBondedMixnode(bounded);
    }
    if (bounded && !('stake' in bounded)) {
      setBondedGateway(bounded);
    }
    setIsLoading(false);
  }, [network]);

  useEffect(() => {
    resetState();
    refresh();
  }, [network]);

  const bondMixnode = async (data: TBondMixNodeArgs) => {
    // TODO some logic
    let tx;
    try {
      tx = await bondMixNodeRequest(data);
    } catch (e: any) {
      throw new Error(e);
    }
    return tx;
  };

  const bondGateway = async (data: TBondGatewayArgs) => {
    // TODO some logic
    let tx;
    try {
      tx = await bondGatewayRequest(data);
    } catch (e: any) {
      throw new Error(e);
    }
    return tx;
  };

  const unbondMixnode = async () => {
    // TODO some logic
    let tx;
    try {
      tx = await unbondMixNodeRequest();
    } catch (e: any) {
      throw new Error(e);
    }
    return tx;
  };

  const unbondGateway = async () => {
    // TODO some logic
    let tx;
    try {
      tx = await unbondGatewayRequest();
    } catch (e: any) {
      throw new Error(e);
    }
    return tx;
  };

  const redeemRewards = async () => {
    // TODO some logic
    let tx;
    try {
      tx = await claimOperatorRewards();
    } catch (e: any) {
      throw new Error(e);
    }
    return tx;
  };

  const compoundRewards = async () => {
    // TODO some logic
    let tx;
    try {
      tx = await compoundOperatorRewards();
    } catch (e: any) {
      throw new Error(e);
    }
    return tx;
  };

  const memoizedValue = useMemo(
    () => ({
      isLoading,
      error,
      bondMixnode,
      bondedMixnode,
      bondedGateway,
      bondGateway,
      unbondMixnode,
      unbondGateway,
      refresh,
      redeemRewards,
      compoundRewards,
    }),
    [isLoading, error, bondedMixnode, bondedGateway],
  );

  return <BondingContext.Provider value={memoizedValue}>{children}</BondingContext.Provider>;
};

export const useBondingContext = () => useContext<TBondingContext>(BondingContext);
