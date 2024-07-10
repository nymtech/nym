import { FeeDetails, TransactionExecuteResult } from '@nymproject/types';
import { useCallback, useEffect, useMemo, useState } from 'react';
import type { Network } from '@src/types';
import { BondingContext, TBondedGateway, TBondedMixnode } from '../bonding';
import { mockSleep } from './utils';
import { TBondGatewaySignatureArgs, TBondMixnodeSignatureArgs } from '../../types';

const SLEEP_MS = 1000;

const bondedMixnodeMock: TBondedMixnode = {
  mixId: 1,
  name: 'Monster node',
  identityKey: '7mjM2fYbtN6kxMwp1TrmQ4VwPks3URR5pBgWPWhzT98F',
  stake: { denom: 'nym', amount: '1234' },
  bond: { denom: 'nym', amount: '1234' },
  stakeSaturation: '95',
  profitMargin: '15',
  operatorRewards: { denom: 'nym', amount: '1234' },
  delegators: 5423,
  status: 'active',
  operatorCost: { denom: 'nym', amount: '1234' },
  host: '1.2.3.4',
  routingScore: 75,
  activeSetProbability: 'High',
  standbySetProbability: 'Low',
  estimatedRewards: { denom: 'nym', amount: '2' },
  httpApiPort: 8000,
  mixPort: 1789,
  verlocPort: 1790,
  version: '1.0.2',
  isUnbonding: false,
  uptime: 1,
};

const bondedGatewayMock: TBondedGateway = {
  id: 1,
  name: 'Monster node',
  identityKey: 'WayM2fYbtN6kxMwp1TrmQ4VwPks3URR5pBgWPWhzT98F',
  ip: '112.43.234.57',
  bond: { denom: 'nym', amount: '1234' },
  host: '1.2.34.5 ',
  httpApiPort: 8000,
  mixPort: 1789,
  verlocPort: 1790,
  version: '1.0.2',
  routingScore: {
    average: 100,
    current: 100,
  },
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

  const updateMixnode = async (): Promise<TransactionExecuteResult> => {
    setIsLoading(true);
    await mockSleep(SLEEP_MS);
    triggerStateUpdate();
    setIsLoading(false);
    return TxResultMock;
  };

  const updateBondAmount = async (): Promise<TransactionExecuteResult> => {
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

  const generateMixnodeMsgPayload = async (_data: TBondMixnodeSignatureArgs) => {
    setIsLoading(true);
    await mockSleep(SLEEP_MS);
    triggerStateUpdate();
    setIsLoading(false);
    return '77dcaba7f41409984f4ebce4a386f59b10f1e65ed5514d1acdccae30174bd84b';
  };

  const generateGatewayMsgPayload = async (_data: TBondGatewaySignatureArgs) => {
    setIsLoading(true);
    await mockSleep(SLEEP_MS);
    triggerStateUpdate();
    setIsLoading(false);
    return '77dcaba7f41409984f4ebce4a386f59b10f1e65ed5514d1acdccae30174bd84b';
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
      fee,
      feeLoading,
      getFee,
      resetFeeState,
      updateMixnode,
      updateBondAmount,
      checkOwnership,
      generateMixnodeMsgPayload,
      generateGatewayMsgPayload,
      isVestingAccount: false,
    }),
    [isLoading, error, bondedMixnode, bondedGateway, trigger, fee],
  );

  return <BondingContext.Provider value={memoizedValue}>{children}</BondingContext.Provider>;
};
