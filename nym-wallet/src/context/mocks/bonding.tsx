import { TransactionExecuteResult } from '@nymproject/types';
import React, { useCallback, useEffect, useMemo, useState } from 'react';
import type { Network } from 'src/types';
import { BondedGateway, BondedMixnode, BondingContext } from '../bonding';
import { mockSleep } from './utils';

const SLEEP_MS = 1000;

const bondedMixnodeMock = {
  key: '7mjM2fYbtN6kxMwp1TrmQ4VwPks3URR5pBgWPWhzT98F',
  ip: '112.43.234.56',
  stake: 35847.221,
  bond: 12576.32745,
  stakeSaturation: 95,
  profitMargin: 15,
  nodeRewards: 12576.32745,
  operatorRewards: 12576.32,
  delegators: 5423,
};

const bondedGatewayMock = {
  key: 'WayM2fYbtN6kxMwp1TrmQ4VwPks3URR5pBgWPWhzT98F',
  ip: '112.43.234.57',
  bond: 12576.32745,
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

export const MockBondingContextProvider = ({
  network,
  children,
}: {
  network?: Network;
  children?: React.ReactNode;
}): JSX.Element => {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [bondedData, setBondedData] = useState<BondedMixnode | BondedGateway | null>(null);
  const [bondedMixnode, setBondedMixnode] = useState<BondedMixnode | null>(null);
  const [bondedGateway, setBondedGateway] = useState<BondedGateway | null>(null);
  const [trigger, setTrigger] = useState<Date>(new Date());

  const triggerStateUpdate = () => setTrigger(new Date());

  const resetState = () => {
    setIsLoading(true);
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
    setIsLoading(false);
  }, [network]);

  useEffect(() => {
    resetState();
    refresh();
  }, [network, bondedData]);

  const bondMixnode = async (): Promise<TransactionExecuteResult> => {
    await mockSleep(SLEEP_MS);
    setBondedData(bondedMixnodeMock);
    return TxResultMock;
  };

  const bondGateway = async (): Promise<TransactionExecuteResult> => {
    await mockSleep(SLEEP_MS);
    setBondedData(bondedGatewayMock);
    return TxResultMock;
  };

  const unbondMixnode = async (): Promise<TransactionExecuteResult> => {
    await mockSleep(SLEEP_MS);
    setBondedData(null);
    return TxResultMock;
  };

  const unbondGateway = async (): Promise<TransactionExecuteResult> => {
    await mockSleep(SLEEP_MS);
    setBondedData(null);
    return TxResultMock;
  };

  const redeemRewards = async (): Promise<TransactionExecuteResult[]> => {
    await mockSleep(SLEEP_MS);
    triggerStateUpdate();
    return [TxResultMock];
  };

  const compoundRewards = async (): Promise<TransactionExecuteResult[]> => {
    await mockSleep(SLEEP_MS);
    triggerStateUpdate();
    return [TxResultMock];
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
    [isLoading, error, bondedMixnode, bondedGateway, trigger],
  );

  return <BondingContext.Provider value={memoizedValue}>{children}</BondingContext.Provider>;
};
