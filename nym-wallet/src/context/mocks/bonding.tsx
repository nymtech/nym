import { FeeDetails, MajorCurrencyAmount, TransactionExecuteResult } from '@nymproject/types';
import React, { useCallback, useEffect, useMemo, useState } from 'react';
import type { Network } from 'src/types';
import { BondedGateway, BondedMixnode, BondingContext, FeeOperation } from '../bonding';
import { mockSleep } from './utils';

const SLEEP_MS = 1000;

const bondedMixnodeMock: BondedMixnode = {
  identityKey: '7mjM2fYbtN6kxMwp1TrmQ4VwPks3URR5pBgWPWhzT98F',
  ip: '112.43.234.56',
  stake: { denom: 'NYM', amount: '1234' },
  bond: { denom: 'NYM', amount: '1234' },
  stakeSaturation: 95,
  profitMargin: 15,
  nodeRewards: { denom: 'NYM', amount: '1234' },
  operatorRewards: { denom: 'NYM', amount: '1234' },
  delegators: 5423,
  status: 'active',
};

const bondedGatewayMock: BondedGateway = {
  identityKey: 'WayM2fYbtN6kxMwp1TrmQ4VwPks3URR5pBgWPWhzT98F',
  ip: '112.43.234.57',
  bond: { denom: 'NYM', amount: '1234' },
};

const TxResultMock: TransactionExecuteResult = {
  logs_json: '',
  data_json: '',
  transaction_hash: '55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
  gas_info: {
    gas_wanted: BigInt(1),
    gas_used: BigInt(1),
    fee: { amount: '1', denom: 'NYM' },
  },
  fee: { amount: '1', denom: 'NYM' },
};

const feeMock: FeeDetails = {
  amount: { denom: 'NYM', amount: '1' },
  fee: { Auto: 1 },
};

export const MockBondingContextProvider = ({
  network,
  children,
}: {
  network?: Network;
  children?: React.ReactNode;
}): JSX.Element => {
  const [loading, setLoading] = useState(true);
  const [feeLoading, setFeeLoading] = useState(false);
  const [fee, setFee] = useState<FeeDetails | undefined>();
  const [error, setError] = useState<string>();
  const [bondedData, setBondedData] = useState<BondedMixnode | BondedGateway | null>(null);
  const [bondedMixnode, setBondedMixnode] = useState<BondedMixnode | null>(null);
  const [bondedGateway, setBondedGateway] = useState<BondedGateway | null>(null);
  const [trigger, setTrigger] = useState<Date>(new Date());

  const triggerStateUpdate = () => setTrigger(new Date());

  const resetState = () => {
    setLoading(true);
    setError(undefined);
    setBondedGateway(null);
    setBondedMixnode(null);
  };

  // fake tauri request
  const fetchBondingData: () => Promise<BondedMixnode | BondedGateway | null> = async () => {
    await mockSleep(SLEEP_MS);
    return bondedData;
  };

  const refresh = useCallback(async () => {
    const bounded = await fetchBondingData();
    if (bounded && 'stake' in bounded) {
      setBondedMixnode(bounded);
    }
    if (bounded && !('stake' in bounded)) {
      setBondedGateway(bounded);
    }
    setLoading(false);
  }, [network]);

  useEffect(() => {
    resetState();
    refresh();
  }, [network, bondedData]);

  const bondMixnode = async (): Promise<TransactionExecuteResult> => {
    setLoading(true);
    await mockSleep(SLEEP_MS);
    setBondedData(bondedMixnodeMock);
    setLoading(false);
    return TxResultMock;
  };

  const bondGateway = async (): Promise<TransactionExecuteResult> => {
    setLoading(true);
    await mockSleep(SLEEP_MS);
    setBondedData(bondedGatewayMock);
    setLoading(false);
    return TxResultMock;
  };

  const unbondMixnode = async (): Promise<TransactionExecuteResult> => {
    setLoading(true);
    await mockSleep(SLEEP_MS);
    setBondedData(null);
    setLoading(false);
    return TxResultMock;
  };

  const unbondGateway = async (): Promise<TransactionExecuteResult> => {
    setLoading(true);
    await mockSleep(SLEEP_MS);
    setBondedData(null);
    setLoading(false);
    return TxResultMock;
  };

  const redeemRewards = async (): Promise<TransactionExecuteResult[] | undefined> => {
    setLoading(true);
    await mockSleep(SLEEP_MS);
    triggerStateUpdate();
    setLoading(false);
    return [TxResultMock];
  };

  const compoundRewards = async (): Promise<TransactionExecuteResult[] | undefined> => {
    setLoading(true);
    await mockSleep(SLEEP_MS);
    triggerStateUpdate();
    setLoading(false);
    return [TxResultMock];
  };

  const updateMixnode = async (): Promise<TransactionExecuteResult> => {
    setLoading(true);
    await mockSleep(SLEEP_MS);
    triggerStateUpdate();
    setLoading(false);
    return TxResultMock;
  };

  const bondMore = async (_signature: string, _additionalBond: MajorCurrencyAmount) => {
    setLoading(true);
    await mockSleep(SLEEP_MS);
    triggerStateUpdate();
    setLoading(false);
    return TxResultMock;
  };

  const getFee = async (_feeOperation: FeeOperation, _args: any) => {
    setFeeLoading(true);
    await mockSleep(SLEEP_MS);
    setFeeLoading(false);
    setFee(feeMock);
    return feeMock;
  };

  const resetFeeState = () => {};

  const memoizedValue = useMemo(
    () => ({
      loading,
      error,
      bondMixnode,
      bondGateway,
      unbondMixnode,
      unbondGateway,
      refresh,
      redeemRewards,
      compoundRewards,
      fee,
      feeLoading,
      getFee,
      resetFeeState,
      updateMixnode,
      bondMore,
    }),
    [loading, error, bondedMixnode, bondedGateway, trigger, fee],
  );

  return <BondingContext.Provider value={memoizedValue}>{children}</BondingContext.Provider>;
};
