import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api';
import {
  Account,
  Balance,
  MajorCurrencyAmount,
  OriginalVestingResponse,
  Period,
  VestingAccountInfo,
} from '@nymproject/types';
import {
  getVestingCoins,
  getVestedCoins,
  getLockedCoins,
  getSpendableCoins,
  getOriginalVesting,
  getCurrentVestingPeriod,
  getVestingAccountInfo,
} from '../requests';
import { Console } from '../utils/console';

type TTokenAllocation = {
  [key in 'vesting' | 'vested' | 'locked' | 'spendable']: MajorCurrencyAmount['amount'];
};

export type TUseuserBalance = {
  error?: string;
  balance?: Balance;
  tokenAllocation?: TTokenAllocation;
  originalVesting?: OriginalVestingResponse;
  currentVestingPeriod?: Period;
  vestingAccountInfo?: VestingAccountInfo;
  isLoading: boolean;
  fetchBalance: () => void;
  clearBalance: () => void;
  clearAll: () => void;
  fetchTokenAllocation: () => void;
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
          currentVestingPer,
          vestingAccountDetail,
        ] = await Promise.all([
          getOriginalVesting(clientDetails?.client_address),
          getVestingCoins(clientDetails?.client_address),
          getVestedCoins(clientDetails?.client_address),
          getLockedCoins(),
          getSpendableCoins(),
          getCurrentVestingPeriod(clientDetails?.client_address),
          getVestingAccountInfo(clientDetails?.client_address),
        ]);
        setOriginalVesting(originalVestingValue);
        setCurrentVestingPeriod(currentVestingPer);
        setTokenAllocation({
          vesting: vestingCoins.amount,
          vested: vestedCoins.amount,
          locked: lockedCoins.amount,
          spendable: spendableCoins.amount,
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
      const bal = await invoke('get_balance');
      setBalance(bal as Balance);
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

  const handleRefresh = async (addr?: string) => {
    if (addr) {
      await fetchBalance();
      await fetchTokenAllocation();
    } else {
      clearAll();
    }
  };

  useEffect(() => {
    handleRefresh(clientDetails?.client_address);
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
  };
};
