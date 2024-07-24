import { useCallback, useEffect, useState } from 'react';
import { Account, Balance, DecCoin, OriginalVestingResponse, Period, VestingAccountInfo } from '@nymproject/types';
import {
  getVestingCoins,
  getVestedCoins,
  getLockedCoins,
  getSpendableCoins,
  getOriginalVesting,
  getCurrentVestingPeriod,
  getVestingAccountInfo,
  getSpendableRewardCoins,
  getSpendableVestedCoins,
  userBalance,
} from '../requests';
import { Console } from '../utils/console';

type TTokenAllocation = {
  [key in
    | 'vesting'
    | 'vested'
    | 'locked'
    | 'spendable'
    | 'spendableRewardCoins'
    | 'spendableVestedCoins']: DecCoin['amount'];
};

export type TUseuserBalance = {
  error?: string;
  balance?: Balance;
  tokenAllocation?: TTokenAllocation;
  originalVesting?: OriginalVestingResponse;
  currentVestingPeriod?: Period;
  vestingAccountInfo?: VestingAccountInfo;
  isLoading: boolean;
  fetchBalance: () => Promise<void>;
  fetchTokenAllocation: () => Promise<void>;
  clearBalance: () => void;
  clearAll: () => void;
  refreshBalances: () => Promise<void>;
};

export const useGetBalance = (clientDetails?: Account): TUseuserBalance => {
  const [balance, setBalance] = useState<Balance>();
  const [error, setError] = useState<string>();
  const [tokenAllocation, setTokenAllocation] = useState<TTokenAllocation>();
  const [originalVesting, setOriginalVesting] = useState<OriginalVestingResponse>();
  const [currentVestingPeriod, setCurrentVestingPeriod] = useState<Period>();
  const [vestingAccountInfo, setVestingAccountInfo] = useState<VestingAccountInfo>();
  const [isLoading, setIsLoading] = useState(false);

  const clearBalance = () => setBalance(undefined);
  const clearTokenAllocation = () => setTokenAllocation(undefined);
  const clearOriginalVesting = () => setOriginalVesting(undefined);

  const fetchTokenAllocation = async () => {
    setIsLoading(true);
    if (clientDetails?.client_address) {
      try {
        const [
          originalVestingValue,
          vestingCoins,
          vestedCoins,
          lockedCoins,
          spendableCoins,
          spendableVestedCoins,
          spendableRewardCoins,
          currentPeriod,
          vestingAccountDetail,
        ] = await Promise.all([
          getOriginalVesting(clientDetails?.client_address),
          getVestingCoins(clientDetails?.client_address),
          getVestedCoins(clientDetails?.client_address),
          getLockedCoins(),
          getSpendableCoins(),
          getSpendableVestedCoins(),
          getSpendableRewardCoins(),
          getCurrentVestingPeriod(clientDetails?.client_address),
          getVestingAccountInfo(clientDetails?.client_address),
        ]);
        setOriginalVesting(originalVestingValue);
        setCurrentVestingPeriod(currentPeriod);
        setTokenAllocation({
          vesting: vestingCoins.amount,
          vested: vestedCoins.amount,
          locked: lockedCoins.amount,
          spendable: spendableCoins.amount,
          spendableVestedCoins: spendableVestedCoins.amount,
          spendableRewardCoins: spendableRewardCoins.amount,
        });
        setVestingAccountInfo(vestingAccountDetail);
      } catch (e) {
        clearTokenAllocation();
        clearOriginalVesting();
        Console.error(e as string);
      }
    }
    setIsLoading(false);
  };

  const fetchBalance = useCallback(async () => {
    setIsLoading(true);
    setError(undefined);
    try {
      const bal = await userBalance();
      setBalance(bal);
    } catch (err) {
      setError(err as string);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const clearAll = () => {
    clearBalance();
    clearTokenAllocation();
    clearOriginalVesting();
  };

  const refreshBalances = async () => {
    if (clientDetails?.client_address) {
      await fetchBalance();
      await fetchTokenAllocation();
    } else {
      clearAll();
    }
  };

  useEffect(() => {
    refreshBalances();
  }, [clientDetails]);

  return {
    error,
    isLoading,
    balance,
    tokenAllocation,
    originalVesting,
    currentVestingPeriod,
    vestingAccountInfo,
    fetchBalance,
    clearBalance,
    clearAll,
    fetchTokenAllocation,
    refreshBalances,
  };
};
