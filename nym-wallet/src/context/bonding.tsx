import { FeeDetails, MajorCurrencyAmount, MixnodeStatus, TransactionExecuteResult } from '@nymproject/types';
import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import type { Network } from 'src/types';
import { TBondGatewayArgs, TBondMixNodeArgs } from 'src/types';
import {
  bondGateway as bondGatewayRequest,
  bondMixNode as bondMixNodeRequest,
  claimOperatorRewards,
  compoundOperatorRewards,
  simulateBondGateway,
  simulateBondMixnode,
  simulateUnbondGateway,
  simulateUnbondMixnode,
  simulateVestingBondGateway,
  simulateVestingBondMixnode,
  simulateVestingUnbondGateway,
  simulateVestingUnbondMixnode,
  simulateUpdateMixnode,
  simulateVestingUpdateMixnode,
  unbondGateway as unbondGatewayRequest,
  unbondMixNode as unbondMixnodeRequest,
  vestingBondGateway,
  vestingBondMixNode,
  vestingUnbondGateway,
  vestingUnbondMixnode,
  updateMixnode as updateMixnodeRequest,
  vestingUpdateMixnode as updateMixnodeVestingRequest,
  getGatewayBondDetails,
  getMixnodeBondDetails,
  getMixnodeStatus,
} from '../requests';
import { useGetFee } from '../hooks/useGetFee';
import { useCheckOwnership } from '../hooks/useCheckOwnership';
import { AppContext } from './main';

const bounded: BondedMixnode = {
  identityKey: 'B2Xx4haarLWMajX8w259oHjtRZsC7nHwagbWrJNiA3QC',
  bond: { denom: 'NYM', amount: '1234' },
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
  identityKey: 'B2Xx4haarLWMajX8w259oHjtRZsC7nHwagbWrJNiA3QC',
  ip: '1.2.34.5',
  location: 'France',
}; */

// TODO add relevant data
export interface BondedMixnode {
  identityKey: string;
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

// TODO add relevant data
export interface BondedGateway {
  identityKey: string;
  ip: string;
  bond: MajorCurrencyAmount;
  location?: string; // TODO not yet available, only available in Network Explorer API
}

export type TokenPool = 'locked' | 'balance';

export type FeeOperation =
  | 'bondMixnode'
  | 'bondMixnodeWithVesting'
  | 'bondGateway'
  | 'bondGatewayWithVesting'
  | 'unbondMixnode'
  | 'unbondGateway'
  | 'updateMixnode'
  | 'bondMore'
  | 'compoundRewards'
  | 'redeemRewards';

export type TBondingContext = {
  loading: boolean;
  error?: string;
  bondedMixnode?: BondedMixnode | null;
  bondedGateway?: BondedGateway | null;
  refresh: () => Promise<void>;
  bondMixnode: (
    data: Omit<TBondMixNodeArgs, 'fee'>,
    tokenPool: TokenPool,
  ) => Promise<TransactionExecuteResult | undefined>;
  bondGateway: (
    data: Omit<TBondGatewayArgs, 'fee'>,
    tokenPool: TokenPool,
  ) => Promise<TransactionExecuteResult | undefined>;
  bondMore: (signature: string, additionalBond: MajorCurrencyAmount) => Promise<TransactionExecuteResult | undefined>;
  unbondMixnode: () => Promise<TransactionExecuteResult | undefined>;
  unbondGateway: () => Promise<TransactionExecuteResult | undefined>;
  redeemRewards: () => Promise<TransactionExecuteResult[] | undefined>;
  compoundRewards: () => Promise<TransactionExecuteResult[] | undefined>;
  updateMixnode: (pm: number) => Promise<TransactionExecuteResult | undefined>;
  fee?: FeeDetails;
  getFee: <T>(feeOperation: FeeOperation, args: T) => Promise<FeeDetails | undefined>;
  feeDetails?: FeeDetails;
  feeLoading: boolean;
  feeError?: string;
  resetFeeState: () => void;
};

export const BondingContext = createContext<TBondingContext>({
  loading: true,
  feeLoading: false,
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
  getFee(): Promise<FeeDetails> {
    throw new Error('Not implemented');
  },
  resetFeeState(): void {},
  updateMixnode: async () => {
    throw new Error('Not implemented');
  },
  bondMore(): Promise<TransactionExecuteResult | undefined> {
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
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [bondedMixnode, setBondedMixnode] = useState<BondedMixnode | null>(null);
  const [bondedGateway, setBondedGateway] = useState<BondedGateway | null>(null);
  const { fee, resetFeeState, feeError, isFeeLoading } = useGetFee();
  const { ownership, checkOwnership } = useCheckOwnership();
  const { userBalance } = useContext(AppContext);

  const isVesting = Boolean(ownership.vestingPledge);

  useEffect(() => {
    const init = async () => {
      await checkOwnership();
    };
    init();
  }, [checkOwnership]);

  useEffect(() => {
    if (feeError) {
      setError(`An error occurred while retrieving fee: ${feeError}`);
    }
  }, [feeError]);

  const resetState = () => {
    setLoading(true);
    setError(undefined);
    setBondedGateway(null);
    setBondedMixnode(null);
  };

  const fetchMixnodeStatus = useCallback(async () => {
    setLoading(true);
    if (bondedMixnode) {
      try {
        const { status } = await getMixnodeStatus(bondedMixnode.identityKey);
        setBondedMixnode({ ...bondedMixnode, status });
      } catch (e: any) {
        setError(`While fetching mixnode status, an error occurred: ${e}`);
      } finally {
        setLoading(false);
      }
    }
  }, [bondedMixnode]);

  const refresh = useCallback(async () => {
    setLoading(true);
    if (ownership.hasOwnership && ownership.nodeType === 'mixnode') {
      let data;
      try {
        data = await getMixnodeBondDetails();
      } catch (e: any) {
        setError(`While fetching current bond state, an error occurred: ${e}`);
      }
      // TODO convert the returned data from the request to `BondedMixnode` type
      setBondedMixnode(bounded);
    }
    if (ownership.hasOwnership && ownership.nodeType === 'gateway') {
      let data;
      try {
        data = await getGatewayBondDetails();
      } catch (e: any) {
        setError(`While fetching current bond state, an error occurred: ${e}`);
      }
      // TODO convert the returned data from the request to `BondedGateway` type
      setBondedGateway(bounded);
    }
    setLoading(false);
  }, [network, ownership]);

  useEffect(() => {
    resetState();
    refresh();
  }, [network, ownership]);

  useEffect(() => {
    if (bondedMixnode) {
      fetchMixnodeStatus();
    }
  }, [bondedMixnode]);

  const bondMixnode = async (data: Omit<TBondMixNodeArgs, 'fee'>, tokenPool: TokenPool) => {
    let tx: TransactionExecuteResult | undefined;
    const payload = {
      ...data,
      fee: fee?.fee,
    };
    setLoading(true);
    try {
      if (tokenPool === 'balance') {
        tx = await bondMixNodeRequest(payload);
        await userBalance.fetchBalance();
      }
      if (tokenPool === 'locked') {
        tx = await vestingBondMixNode(payload);
        await userBalance.fetchTokenAllocation();
      }
      return tx;
    } catch (e: any) {
      setError(`an error occurred: ${e}`);
    } finally {
      setLoading(false);
    }
    return undefined;
  };

  const bondGateway = async (data: Omit<TBondGatewayArgs, 'fee'>, tokenPool: TokenPool) => {
    let tx: TransactionExecuteResult | undefined;
    const payload = {
      ...data,
      fee: fee?.fee,
    };
    setLoading(true);
    try {
      if (tokenPool === 'balance') {
        tx = await bondGatewayRequest(payload);
        await userBalance.fetchBalance();
      }
      if (tokenPool === 'locked') {
        tx = await vestingBondGateway(payload);
        await userBalance.fetchTokenAllocation();
      }
      return tx;
    } catch (e: any) {
      setError(`an error occurred: ${e}`);
    } finally {
      setLoading(false);
    }
    return undefined;
  };

  const unbondMixnode = async () => {
    let tx;
    setLoading(true);
    try {
      if (isVesting) tx = await vestingUnbondMixnode(fee?.fee);
      if (!isVesting) tx = await unbondMixnodeRequest(fee?.fee);
    } catch (e: any) {
      setError(`an error occurred: ${e}`);
    } finally {
      await checkOwnership();
      setLoading(false);
    }
    return tx;
  };

  const unbondGateway = async () => {
    let tx;
    setLoading(true);
    try {
      if (isVesting) tx = await vestingUnbondGateway(fee?.fee);
      if (!isVesting) tx = await unbondGatewayRequest(fee?.fee);
    } catch (e: any) {
      setError(`an error occurred: ${e}`);
    } finally {
      await checkOwnership();
      setLoading(false);
    }
    return tx;
  };

  const updateMixnode = async (pm: number) => {
    let tx;
    setLoading(true);
    try {
      // TODO use estimated fee, need requests update
      if (isVesting) tx = await updateMixnodeRequest(pm);
      if (!isVesting) tx = await updateMixnodeVestingRequest(pm);
    } catch (e: any) {
      setError(`an error occurred: ${e}`);
    } finally {
      setLoading(false);
    }
    return tx;
  };

  const redeemRewards = async () => {
    let tx;
    setLoading(true);
    try {
      tx = await claimOperatorRewards(); // TODO use estimated fee, update `claimOperatorRewards`
    } catch (e: any) {
      setError(`an error occurred: ${e}`);
    } finally {
      setLoading(false);
    }
    return tx;
  };

  const compoundRewards = async () => {
    let tx;
    setLoading(true);
    try {
      tx = await compoundOperatorRewards(); // TODO use estimated fee, update `compoundOperatorRewards`
    } catch (e: any) {
      setError(`an error occurred: ${e}`);
    } finally {
      setLoading(false);
    }
    return tx;
  };

  const bondMore = async (_signature: string, _additionalBond: MajorCurrencyAmount) =>
    // TODO to implement
    undefined;

  const feeOps = useMemo(
    () => ({
      bondMixnode: simulateBondMixnode,
      bondMixnodeWithVesting: simulateVestingBondMixnode,
      bondGateway: simulateBondGateway,
      bondGatewayWithVesting: simulateVestingBondGateway,
      unbondMixnode: isVesting ? simulateVestingUnbondMixnode : simulateUnbondMixnode,
      unbondGateway: isVesting ? simulateVestingUnbondGateway : simulateUnbondGateway,
      updateMixnode: isVesting ? simulateVestingUpdateMixnode : simulateUpdateMixnode,
      bondMore: () => undefined as unknown as Promise<FeeDetails>, // TODO fee request to implement
      compoundRewards: () => undefined as unknown as Promise<FeeDetails>, // TODO fee request to implement
      redeemRewards: () => undefined as unknown as Promise<FeeDetails>, // TODO fee request to implement
    }),
    [isVesting],
  );

  const getFee = async (feeOperation: FeeOperation, args: any) => {
    let details;
    try {
      details = feeOps[feeOperation](args);
    } catch (e: any) {
      setError(`An error occurred while retrieving fee: ${e}`);
    }
    return details;
  };

  const memoizedValue = useMemo(
    () => ({
      loading,
      error,
      bondMixnode,
      bondedMixnode,
      bondedGateway,
      bondGateway,
      unbondMixnode,
      unbondGateway,
      updateMixnode,
      refresh,
      redeemRewards,
      compoundRewards,
      feeLoading: isFeeLoading,
      feeError,
      getFee,
      fee,
      resetFeeState,
      bondMore,
    }),
    [loading, error, bondedMixnode, bondedGateway, isFeeLoading, feeError, fee, resetFeeState, isVesting],
  );

  return <BondingContext.Provider value={memoizedValue}>{children}</BondingContext.Provider>;
};

export const useBondingContext = () => useContext<TBondingContext>(BondingContext);
