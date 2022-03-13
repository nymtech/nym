import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api';
import { VestingAccountInfo } from 'src/types/rust/vestingaccountinfo';
import { Balance, Coin, OriginalVestingResponse, Period } from '../types';
import {
  getVestingCoins,
  getVestedCoins,
  getLockedCoins,
  getSpendableCoins,
  getOriginalVesting,
  getCurrentVestingPeriod,
  getVestingAccountInfo,
} from '../requests';

type TTokenAllocation = {
  [key in 'vesting' | 'vested' | 'locked' | 'spendable']: Coin['amount'];
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

export const useGetBalance = (address?: string): TUseuserBalance => {
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
    if (address) {
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
          getOriginalVesting(address),
          getVestingCoins(address),
          getVestedCoins(address),
          getLockedCoins(),
          getSpendableCoins(),
          getCurrentVestingPeriod(address),
          getVestingAccountInfo(address),
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
        console.error(e);
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
      setTimeout(() => {
        setIsLoading(false);
      }, 1000);
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
    handleRefresh(address);
  }, [address]);

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
