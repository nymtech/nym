import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { DelegationWithEverything, TransactionExecuteResult } from '@nymproject/types';
import { RewardsContext, TRewardsTransaction } from '../rewards';
import { useDelegationContext } from '../delegations';
import { mockSleep } from './utils';

export const MockRewardsContextProvider: FCWithChildren = ({ children }) => {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [totalRewards, setTotalRewards] = useState<undefined | string>();
  const { delegations } = useDelegationContext();
  const delegationsHash = delegations
    ?.map((d) => (d as DelegationWithEverything).unclaimed_rewards?.amount.toString())
    .join(',');

  const resetState = () => {
    setIsLoading(true);
    setError(undefined);
    setTotalRewards(undefined);
  };

  const recalculate = () => {
    const sum: number | undefined = delegations
      ?.map((d) => ((d as DelegationWithEverything).unclaimed_rewards?.amount ? Number(10) : Number(0)))
      .reduce((acc, cur) => acc + cur, Number(0));

    setTotalRewards(sum ? `${sum} NYM` : undefined);
  };

  const refresh = useCallback(async () => {
    resetState();
    setTimeout(() => {
      recalculate();
      setIsLoading(false);
    }, 1500);
  }, [delegationsHash]);

  useEffect(() => {
    recalculate();
  }, [delegationsHash]);

  useEffect(() => {
    // reset state and refresh
    resetState();
    refresh();
  }, []);

  const claimRewards = async (mixId: number): Promise<TransactionExecuteResult[]> => {
    if (!delegations) {
      throw new Error('No delegations');
    }

    const d = delegations.find((d1) => (d1 as DelegationWithEverything).mix_id === mixId);

    if (!d) {
      throw new Error(`Unable to find delegation for id = ${mixId}`);
    }

    await mockSleep(1000);

    return [
      {
        transaction_hash: '55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
        fee: {
          amount: '1',
          denom: 'nym',
        },
        logs_json: '[]',
        msg_responses_json: '[]',
        gas_info: {
          gas_wanted: { gas_units: BigInt(1) },
          gas_used: { gas_units: BigInt(1) },
        },
      },
      {
        transaction_hash: '55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
        fee: {
          amount: '1',
          denom: 'nym',
        },
        msg_responses_json: '[]',
        logs_json: '[]',
        gas_info: {
          gas_wanted: { gas_units: BigInt(1) },
          gas_used: { gas_units: BigInt(1) },
        },
      },
    ];
  };

  const redeemAllRewards = async (): Promise<TRewardsTransaction[]> => {
    if (!delegations) {
      throw new Error('No delegations');
    }

    await mockSleep(1000);

    return [
      {
        transactionUrl:
          'https://sandbox-blocks.nymtech.net/transactions/55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
        transactionHash: '55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
      },
      {
        transactionUrl:
          'https://sandbox-blocks.nymtech.net/transactions/55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
        transactionHash: '55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
      },
    ];
  };

  const memoizedValue = useMemo(
    () => ({
      isLoading,
      error,
      totalRewards,
      refresh,
      claimRewards,
      redeemAllRewards,
    }),
    [isLoading, error, totalRewards],
  );

  return <RewardsContext.Provider value={memoizedValue}>{children}</RewardsContext.Provider>;
};
