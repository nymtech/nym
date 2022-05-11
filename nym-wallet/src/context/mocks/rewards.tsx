import React, { FC, useCallback, useEffect, useMemo, useState } from 'react';
import { RewardsContext, TRewardsTransaction } from '../rewards';
import { useDelegationContext } from '../delegations';
import { mockSleep } from './utils';

export const MockRewardsContextProvider: FC = ({ children }) => {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [totalRewards, setTotalRewards] = useState<undefined | string>();
  const { delegations, updateDelegation } = useDelegationContext();
  const delegationsHash = delegations?.map((d) => d.accumulated_rewards).join(',');

  const resetState = () => {
    setIsLoading(true);
    setError(undefined);
    setTotalRewards(undefined);
  };

  const recalculate = () => {
    const sum: number | undefined = delegations
      ?.map((d) => (d.accumulated_rewards ? Number(10) : Number(0)))
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

  const redeemRewards = async (mixnodeAddress: string): Promise<TRewardsTransaction> => {
    if (!delegations) {
      throw new Error('No delegations');
    }

    const d = delegations.find((d1) => d1.node_identity === mixnodeAddress);

    if (!d) {
      throw new Error(`Unable to find delegation for id = ${mixnodeAddress}`);
    }

    await mockSleep(1000);

    // the `updateDelegation as any` is hacking typescript with type erasure so that
    // the hook interface can stay the same and the mock implementation is easy
    await (updateDelegation as any)({ ...d, reward: undefined }, true);

    return {
      transactionUrl:
        'https://sandbox-blocks.nymtech.net/transactions/55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
    };
  };

  const redeemAllRewards = async (): Promise<TRewardsTransaction> => {
    if (!delegations) {
      throw new Error('No delegations');
    }

    await mockSleep(1000);

    // eslint-disable-next-line no-restricted-syntax
    for (const d of delegations) {
      // the `updateDelegation as any` is hacking typescript with type erasure so that
      // the hook interface can stay the same and the mock implementation is easy
      // eslint-disable-next-line no-await-in-loop
      await (updateDelegation as any)({ ...d, reward: undefined }, true);
    }

    return {
      transactionUrl:
        'https://sandbox-blocks.nymtech.net/transactions/55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
    };
  };

  const memoizedValue = useMemo(
    () => ({
      isLoading,
      error,
      totalRewards,
      refresh,
      redeemRewards,
      redeemAllRewards,
    }),
    [isLoading, error, totalRewards],
  );

  return <RewardsContext.Provider value={memoizedValue}>{children}</RewardsContext.Provider>;
};
