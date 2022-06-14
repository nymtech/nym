import React, { FC, useCallback, useEffect, useMemo, useState } from 'react';
import { DelegationWithEverything, MajorCurrencyAmount, TransactionExecuteResult } from '@nymproject/types';
import { DelegationContext, TDelegationTransaction } from '../delegations';

import { mockSleep } from './utils';

const SLEEP_MS = 1000;

let mockDelegations: DelegationWithEverything[] = [
  {
    node_identity: 'FiojKW7oY9WQmLCiYAsCA21tpowZHS6zcUoyYm319p6Z',
    delegated_on_iso_datetime: new Date(2021, 1, 1).toDateString(),
    accumulated_rewards: { amount: '0.05', denom: 'NYM' },
    amount: { amount: '10', denom: 'NYM' },
    profit_margin_percent: 0.1122323949234,
    owner: '',
    block_height: BigInt(100),
    stake_saturation: 0.5,
    proxy: '',
    avg_uptime_percent: 0.5,
    total_delegation: { amount: '0', denom: 'NYM' },
    pledge_amount: { amount: '0', denom: 'NYM' },
    pending_events: [],
    history: [],
  },
  {
    node_identity: 'DT8S942S8AQs2zKHS9SVo1GyHmuca3pfL2uLhLksJ3D8',
    accumulated_rewards: { amount: '0.1', denom: 'NYM' },
    amount: { amount: '100', denom: 'NYM' },
    delegated_on_iso_datetime: new Date(2021, 1, 2).toDateString(),
    profit_margin_percent: 0.89,
    owner: '',
    block_height: BigInt(4000),
    stake_saturation: 0.5,
    proxy: '',
    avg_uptime_percent: 0.1,
    total_delegation: { amount: '0', denom: 'NYM' },
    pledge_amount: { amount: '0', denom: 'NYM' },
    pending_events: [],
    history: [],
  },
];

export const MockDelegationContextProvider: FC<{}> = ({ children }) => {
  const [trigger, setTrigger] = useState<Date>(new Date());
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [delegations, setDelegations] = useState<undefined | DelegationWithEverything[]>();
  const [totalDelegations, setTotalDelegations] = useState<undefined | string>();

  const triggerStateUpdate = () => setTrigger(new Date());

  const getDelegations = async (): Promise<DelegationWithEverything[]> =>
    mockDelegations.sort((a, b) => a.node_identity.localeCompare(b.node_identity));

  const recalculate = async () => {
    const newDelegations = await getDelegations();
    const newTotalDelegations = `${newDelegations.length * 100} NYM`;
    setDelegations(newDelegations);
    setTotalDelegations(newTotalDelegations);
  };

  const addDelegation = async ({
    identity,
    amount,
  }: {
    identity: string;
    amount: MajorCurrencyAmount;
  }): Promise<TransactionExecuteResult> => {
    await mockSleep(SLEEP_MS);
    // mockDelegations.push({ ...newDelegation });
    await recalculate();
    triggerStateUpdate();

    setTimeout(async () => {
      mockDelegations = mockDelegations.map((d) => {
        if (d.node_identity === identity) {
          return { ...d, isPending: undefined };
        }
        return d;
      });
      await recalculate();
      triggerStateUpdate();
    }, 3000);

    return {
      logs_json: '',
      data_json: '',
      gas_info: {
        gas_wanted: BigInt(1),
        gas_used: BigInt(1),
        fee: { amount: '1', denom: 'NYM' },
      },
      transaction_hash: '55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
      fee: { amount: '1', denom: 'NYM' },
    };
  };

  const updateDelegation = async (
    newDelegation: DelegationWithEverything,
    ignorePendingForStorybook?: boolean,
  ): Promise<TDelegationTransaction> => {
    if (ignorePendingForStorybook) {
      mockDelegations = mockDelegations.map((d) => {
        if (d.node_identity === newDelegation.node_identity) {
          return { ...newDelegation };
        }
        return d;
      });
      await recalculate();
      triggerStateUpdate();
      return {
        transactionUrl:
          'https://sandbox-blocks.nymtech.net/transactions/55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
      };
    }

    await mockSleep(SLEEP_MS);
    mockDelegations = mockDelegations.map((d) => {
      if (d.node_identity === newDelegation.node_identity) {
        return { ...newDelegation, isPending: { blockHeight: 1234, actionType: 'delegate' } };
      }
      return d;
    });
    await recalculate();
    triggerStateUpdate();

    setTimeout(async () => {
      mockDelegations = mockDelegations.map((d) => {
        if (d.node_identity === newDelegation.node_identity) {
          return { ...d, isPending: undefined };
        }
        return d;
      });
      await recalculate();
      triggerStateUpdate();
    }, 3000);

    return {
      transactionUrl:
        'https://sandbox-blocks.nymtech.net/transactions/55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
    };
  };

  const undelegate = async (mixnodeAddress: string): Promise<TransactionExecuteResult> => {
    await mockSleep(SLEEP_MS);
    mockDelegations = mockDelegations.map((d) => {
      if (d.node_identity === mixnodeAddress) {
        return { ...d, isPending: { blockHeight: 5678, actionType: 'undelegate' } };
      }
      return d;
    });
    await recalculate();
    triggerStateUpdate();

    setTimeout(async () => {
      mockDelegations = mockDelegations.filter((d) => d.node_identity !== mixnodeAddress);
      await recalculate();
      triggerStateUpdate();
    }, 3000);

    return {
      logs_json: '',
      data_json: '',
      transaction_hash: '',
      gas_info: {
        gas_wanted: BigInt(1),
        gas_used: BigInt(1),
        fee: { amount: '1', denom: 'NYM' },
      },
      fee: { amount: '1', denom: 'NYM' },
    };
  };

  const resetState = () => {
    setIsLoading(true);
    setError(undefined);
    setTotalDelegations(undefined);
    setDelegations([]);
  };

  const refresh = useCallback(async () => {
    resetState();
    setTimeout(async () => {
      try {
        await mockSleep(SLEEP_MS);
        await recalculate();
      } catch (e) {
        setError((e as Error).message);
      }
      setIsLoading(false);
    }, 2000);
  }, []);

  useEffect(() => {
    // reset state and refresh
    resetState();
    refresh();
  }, []);

  const memoizedValue = useMemo(
    () => ({
      isLoading,
      error,
      delegations,
      totalDelegations,
      refresh,
      getDelegations,
      addDelegation,
      updateDelegation,
      undelegate,
    }),
    [isLoading, error, delegations, totalDelegations, trigger],
  );

  return <DelegationContext.Provider value={memoizedValue}>{children}</DelegationContext.Provider>;
};
