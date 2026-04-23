import { useCallback, useEffect, useRef, useState } from 'react';
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
  fetchTokenAllocation: (isBackgroundPoll?: boolean) => Promise<void>;
  clearBalance: () => void;
  clearAll: () => void;
  refreshBalances: () => Promise<void>;
};

/** `BackendError::VestingContractAccountNotFound`, or legacy vesting ABCI wording. */
const isNoVestingAccountError = (e: unknown) => {
  const s = String(e);
  return (
    s === 'Vesting contract has no account for this address' ||
    (s.includes('VESTING') && s.includes('Account does not exist'))
  );
};

export const useGetBalance = (clientDetails?: Account): TUseuserBalance => {
  const [balance, setBalance] = useState<Balance>();
  const [error, setError] = useState<string>();
  const [tokenAllocation, setTokenAllocation] = useState<TTokenAllocation>();
  const [originalVesting, setOriginalVesting] = useState<OriginalVestingResponse>();
  const [currentVestingPeriod, setCurrentVestingPeriod] = useState<Period>();
  const [vestingAccountInfo, setVestingAccountInfo] = useState<VestingAccountInfo>();
  const [isLoading, setIsLoading] = useState(false);

  const vestingAccountStatusRef = useRef<'unknown' | 'absent' | 'present'>('unknown');

  const clearBalance = () => setBalance(undefined);
  const clearTokenAllocation = () => setTokenAllocation(undefined);
  const clearOriginalVesting = () => setOriginalVesting(undefined);

  const clearVestingUiState = () => {
    clearTokenAllocation();
    clearOriginalVesting();
    setCurrentVestingPeriod(undefined);
    setVestingAccountInfo(undefined);
  };

  useEffect(() => {
    vestingAccountStatusRef.current = 'unknown';
  }, [clientDetails?.client_address]);

  const applyAllocationFromResponses = (
    originalVestingValue: OriginalVestingResponse,
    vestingCoins: Awaited<ReturnType<typeof getVestingCoins>>,
    vestedCoins: Awaited<ReturnType<typeof getVestedCoins>>,
    lockedCoins: Awaited<ReturnType<typeof getLockedCoins>>,
    spendableCoins: Awaited<ReturnType<typeof getSpendableCoins>>,
    spendableVestedCoins: Awaited<ReturnType<typeof getSpendableVestedCoins>>,
    spendableRewardCoins: Awaited<ReturnType<typeof getSpendableRewardCoins>>,
    currentPeriod: Awaited<ReturnType<typeof getCurrentVestingPeriod>>,
    vestingAccountDetail: Awaited<ReturnType<typeof getVestingAccountInfo>>,
  ) => {
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
  };

  const fetchTokenAllocation = async (isBackgroundPoll = false) => {
    setIsLoading(true);
    if (!clientDetails?.client_address) {
      setIsLoading(false);
      return;
    }

    if (vestingAccountStatusRef.current === 'absent') {
      if (isBackgroundPoll) {
        setIsLoading(false);
        return;
      }
      vestingAccountStatusRef.current = 'unknown';
    }

    const address = clientDetails.client_address;

    try {
      if (vestingAccountStatusRef.current === 'unknown') {
        let originalVestingValue: OriginalVestingResponse;
        try {
          originalVestingValue = await getOriginalVesting(address);
        } catch (e) {
          if (isNoVestingAccountError(e)) {
            vestingAccountStatusRef.current = 'absent';
            clearVestingUiState();
            return;
          }
          clearVestingUiState();
          Console.error(e as string);
          return;
        }

        vestingAccountStatusRef.current = 'present';
        const [
          vestingCoins,
          vestedCoins,
          lockedCoins,
          spendableCoins,
          spendableVestedCoins,
          spendableRewardCoins,
          currentPeriod,
          vestingAccountDetail,
        ] = await Promise.all([
          getVestingCoins(address),
          getVestedCoins(address),
          getLockedCoins(),
          getSpendableCoins(),
          getSpendableVestedCoins(),
          getSpendableRewardCoins(),
          getCurrentVestingPeriod(address),
          getVestingAccountInfo(address),
        ]);
        applyAllocationFromResponses(
          originalVestingValue,
          vestingCoins,
          vestedCoins,
          lockedCoins,
          spendableCoins,
          spendableVestedCoins,
          spendableRewardCoins,
          currentPeriod,
          vestingAccountDetail,
        );
        return;
      }

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
        getOriginalVesting(address),
        getVestingCoins(address),
        getVestedCoins(address),
        getLockedCoins(),
        getSpendableCoins(),
        getSpendableVestedCoins(),
        getSpendableRewardCoins(),
        getCurrentVestingPeriod(address),
        getVestingAccountInfo(address),
      ]);
      applyAllocationFromResponses(
        originalVestingValue,
        vestingCoins,
        vestedCoins,
        lockedCoins,
        spendableCoins,
        spendableVestedCoins,
        spendableRewardCoins,
        currentPeriod,
        vestingAccountDetail,
      );
    } catch (e) {
      if (isNoVestingAccountError(e)) {
        vestingAccountStatusRef.current = 'absent';
      }
      clearVestingUiState();
      Console.error(e as string);
    } finally {
      setIsLoading(false);
    }
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
    setCurrentVestingPeriod(undefined);
    setVestingAccountInfo(undefined);
    vestingAccountStatusRef.current = 'unknown';
  };

  const refreshBalances = async () => {
    vestingAccountStatusRef.current = 'unknown';
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
