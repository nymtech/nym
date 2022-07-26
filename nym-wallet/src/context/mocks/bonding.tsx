import { FeeDetails, DecCoin, TransactionExecuteResult } from '@nymproject/types';
import React, { useCallback, useEffect, useMemo, useState } from 'react';
import type { Network } from 'src/types';
import { TBondedGateway, TBondedMixnode, BondingContext } from '../bonding';
import { mockSleep } from './utils';

const SLEEP_MS = 1000;

const bondedMixnodeMock: TBondedMixnode = {
  identityKey: '7mjM2fYbtN6kxMwp1TrmQ4VwPks3URR5pBgWPWhzT98F',
  stake: { denom: 'nym', amount: '1234' },
  bond: { denom: 'nym', amount: '1234' },
  stakeSaturation: 95,
  profitMargin: 15,
  nodeRewards: { denom: 'nym', amount: '1234' },
  operatorRewards: { denom: 'nym', amount: '1234' },
  delegators: 5423,
  status: 'active',
};

const bondedGatewayMock: TBondedGateway = {
  identityKey: 'WayM2fYbtN6kxMwp1TrmQ4VwPks3URR5pBgWPWhzT98F',
  ip: '112.43.234.57',
  bond: { denom: 'nym', amount: '1234' },
};

const TxResultMock: TransactionExecuteResult = {
  logs_json: '',
  data_json: '',
  transaction_hash: '55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
  gas_info: {
    gas_wanted: { gas_units: BigInt(1) },
    gas_used: { gas_units: BigInt(1) },
  },
  fee: { amount: '1', denom: 'nym' },
};

const feeMock: FeeDetails = {
  amount: { denom: 'nym', amount: '1' },
  fee: { Auto: 1 },
};

export const MockBondingContextProvider = ({
  network,
  children,
}: {
  network?: Network;
  children?: React.ReactNode;
}): JSX.Element => {
  const [isLoading, setIsLoading] = useState(true);
  const [feeLoading, setFeeLoading] = useState(false);
  const [fee, setFee] = useState<FeeDetails | undefined>();
  const [error, setError] = useState<string>();
  const [bondedData, setBondedData] = useState<TBondedMixnode | TBondedGateway | null>(null);
  const [bondedMixnode, setBondedMixnode] = useState<TBondedMixnode | null>(null);
  const [bondedGateway, setBondedGateway] = useState<TBondedGateway | null>(null);
  const [trigger, setTrigger] = useState<Date>(new Date());

  const triggerStateUpdate = () => setTrigger(new Date());

  const resetState = () => {
    setIsLoading(true);
    setError(undefined);
    setBondedGateway(null);
    setBondedMixnode(null);
  };

  // fake tauri request
  const fetchBondingData: () => Promise<TBondedMixnode | TBondedGateway | null> = async () => {
    await mockSleep(SLEEP_MS);
    return bondedData;
  };

  const checkOwnership = async () => {};

  const refresh = useCallback(async () => {
    const bounded = await fetchBondingData();
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
  }, [network, bondedData]);

  const bondMixnode = async (): Promise<TransactionExecuteResult> => {
    setIsLoading(true);
    await mockSleep(SLEEP_MS);
    setBondedData(bondedMixnodeMock);
    setIsLoading(false);
    return TxResultMock;
  };

  const bondGateway = async (): Promise<TransactionExecuteResult> => {
    setIsLoading(true);
    await mockSleep(SLEEP_MS);
    setBondedData(bondedGatewayMock);
    setIsLoading(false);
    return TxResultMock;
  };

  const unbond = async (): Promise<TransactionExecuteResult> => {
    setIsLoading(true);
    await mockSleep(SLEEP_MS);
    setBondedData(null);
    setIsLoading(false);
    return TxResultMock;
  };

  const redeemRewards = async (): Promise<TransactionExecuteResult | undefined> => {
    setIsLoading(true);
    await mockSleep(SLEEP_MS);
    triggerStateUpdate();
    setIsLoading(false);
    return TxResultMock;
  };

  const compoundRewards = async (): Promise<TransactionExecuteResult | undefined> => {
    setIsLoading(true);
    await mockSleep(SLEEP_MS);
    triggerStateUpdate();
    setIsLoading(false);
    return TxResultMock;
  };

  const updateMixnode = async (): Promise<TransactionExecuteResult> => {
    setIsLoading(true);
    await mockSleep(SLEEP_MS);
    triggerStateUpdate();
    setIsLoading(false);
    return TxResultMock;
  };

  const bondMore = async (_signature: string, _additionalBond: DecCoin) => {
    setIsLoading(true);
    await mockSleep(SLEEP_MS);
    triggerStateUpdate();
    setIsLoading(false);
    return TxResultMock;
  };

  const getFee = async (_feeOperation: any, _args: any) => {
    setFeeLoading(true);
    await mockSleep(SLEEP_MS);
    setFeeLoading(false);
    setFee(feeMock);
    return feeMock;
  };

  const resetFeeState = () => {};

  const memoizedValue = useMemo(
    () => ({
      isLoading,
      error,
      bondMixnode,
      bondGateway,
      unbond,
      refresh,
      redeemRewards,
      compoundRewards,
      fee,
      feeLoading,
      getFee,
      resetFeeState,
      updateMixnode,
      bondMore,
      checkOwnership,
    }),
    [isLoading, error, bondedMixnode, bondedGateway, trigger, fee],
  );

  return <BondingContext.Provider value={memoizedValue}>{children}</BondingContext.Provider>;
};
