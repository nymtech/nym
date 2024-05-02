import React, { useCallback, useEffect, useMemo, useState } from 'react';
import {
  CurrencyDenom,
  DecCoin,
  DelegationWithEverything,
  Fee,
  FeeDetails,
  TransactionExecuteResult,
} from '@nymproject/types';
import { DelegationContext, TDelegationTransaction } from '../delegations';

import { mockSleep } from './utils';
import { TPoolOption } from '../../components';

const SLEEP_MS = 1000;

let mockDelegations: DelegationWithEverything[] = [
  {
    mix_id: 1234,
    node_identity: 'FiojKW7oY9WQmLCiYAsCA21tpowZHS6zcUoyYm319p6Z',
    delegated_on_iso_datetime: new Date(2021, 1, 1).toDateString(),
    unclaimed_rewards: { amount: '0.05', denom: 'nym' },
    amount: { amount: '10', denom: 'nym' },
    owner: '',
    block_height: BigInt(100),
    cost_params: {
      profit_margin_percent: '0.04',
      interval_operating_cost: {
        amount: '20',
        denom: 'nym',
      },
    },
    stake_saturation: '0.2',
    avg_uptime_percent: 0.5,
    accumulated_by_delegates: { amount: '0', denom: 'nym' },
    accumulated_by_operator: { amount: '0', denom: 'nym' },
    uses_vesting_contract_tokens: false,
    pending_events: [],
    mixnode_is_unbonding: false,
    errors: null,
  },
  {
    mix_id: 5678,
    node_identity: 'DT8S942S8AQs2zKHS9SVo1GyHmuca3pfL2uLhLksJ3D8',
    unclaimed_rewards: { amount: '0.1', denom: 'nym' },
    amount: { amount: '100', denom: 'nym' },
    delegated_on_iso_datetime: new Date(2021, 1, 2).toDateString(),
    owner: '',
    block_height: BigInt(4000),
    stake_saturation: '0.5',
    avg_uptime_percent: 0.1,
    cost_params: {
      profit_margin_percent: '0.04',
      interval_operating_cost: {
        amount: '60',
        denom: 'nym',
      },
    },
    accumulated_by_delegates: { amount: '0', denom: 'nym' },
    accumulated_by_operator: { amount: '0', denom: 'nym' },
    uses_vesting_contract_tokens: true,
    pending_events: [],
    mixnode_is_unbonding: false,
    errors: null,
  },
];

export const MockDelegationContextProvider: FCWithChildren = ({ children }) => {
  const [trigger, setTrigger] = useState<Date>(new Date());
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [delegations, setDelegations] = useState<undefined | DelegationWithEverything[]>();
  const [totalDelegations, setTotalDelegations] = useState<undefined | string>();
  const [delegationItemErrors, setDelegationItemErrors] = useState<{ nodeId: string; errors: string }>();

  const triggerStateUpdate = () => setTrigger(new Date());

  const getDelegations = async (): Promise<DelegationWithEverything[]> =>
    mockDelegations.sort((a, b) => a.node_identity.localeCompare(b.node_identity));

  const recalculate = async () => {
    const newDelegations = await getDelegations();
    const newTotalDelegations = `${newDelegations.length * 100} NYM`;
    setDelegations(newDelegations);
    setTotalDelegations(newTotalDelegations);
  };

  const addDelegation = async (
    data: { mix_id: number; amount: DecCoin },
    _tokenPool: TPoolOption,
    _fee?: FeeDetails,
  ): Promise<TransactionExecuteResult> => {
    await mockSleep(SLEEP_MS);
    // mockDelegations.push({ ...newDelegation });
    await recalculate();
    triggerStateUpdate();

    setTimeout(async () => {
      mockDelegations = mockDelegations.map((d) => {
        if (d.mix_id === data.mix_id) {
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
        gas_wanted: { gas_units: BigInt(1) },
        gas_used: { gas_units: BigInt(1) },
      },
      transaction_hash: '55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
      fee: { amount: '1', denom: 'nym' },
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

  const undelegate = async (mix_id: number, _fee?: Fee): Promise<TransactionExecuteResult> => {
    await mockSleep(SLEEP_MS);
    mockDelegations = mockDelegations.map((d) => {
      if (d.mix_id === mix_id) {
        return { ...d, isPending: { blockHeight: 5678, actionType: 'undelegate' } };
      }
      return d;
    });
    await recalculate();
    triggerStateUpdate();

    setTimeout(async () => {
      mockDelegations = mockDelegations.filter((d) => d.mix_id !== mix_id);
      await recalculate();
      triggerStateUpdate();
    }, 3000);

    return {
      logs_json: '',
      data_json: '',
      transaction_hash: '',
      gas_info: {
        gas_wanted: { gas_units: BigInt(1) },
        gas_used: { gas_units: BigInt(1) },
      },
      fee: { amount: '1', denom: 'nym' as CurrencyDenom },
    };
  };

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const undelegateVesting = async (mix_id: number, _fee?: FeeDetails) => ({
    logs_json: '',
    data_json: '',
    transaction_hash: '',
    gas_info: {
      gas_wanted: { gas_units: BigInt(1) },
      gas_used: { gas_units: BigInt(1) },
    },
    fee: { amount: '1', denom: 'nym' as CurrencyDenom },
  });

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
      delegationItemErrors,
      setDelegationItemErrors,
      isLoading,
      error,
      delegations,
      totalDelegations,
      refresh,
      getDelegations,
      addDelegation,
      updateDelegation,
      undelegate,
      undelegateVesting,
    }),
    [isLoading, error, delegations, totalDelegations, trigger],
  );

  return <DelegationContext.Provider value={memoizedValue}>{children}</DelegationContext.Provider>;
};
