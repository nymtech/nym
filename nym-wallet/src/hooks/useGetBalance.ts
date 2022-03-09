import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api'
import { Balance, Coin, OriginalVestingResponse, Period } from '../types'
import {
  getVestingCoins,
  getVestedCoins,
  getLockedCoins,
  getSpendableCoins,
  getOriginalVesting,
  getCurrentVestingPeriod,
  getVestingAccountInfo,
} from '../requests'
import { VestingAccountInfo } from 'src/types/rust/vestingaccountinfo'

type TTokenAllocation = {
  [key in 'vesting' | 'vested' | 'locked' | 'spendable']: Coin['amount']
}

export type TUseuserBalance = {
  error?: string
  balance?: Balance
  tokenAllocation?: TTokenAllocation
  originalVesting?: OriginalVestingResponse
  currentVestingPeriod?: Period
  vestingAccountInfo?: VestingAccountInfo
  isLoading: boolean
  fetchBalance: () => void
  clearBalance: () => void
  clearAll: () => void
  fetchTokenAllocation: () => void
}

export const useGetBalance = (address?: string): TUseuserBalance => {
  const [balance, setBalance] = useState<Balance>()
  const [error, setError] = useState<string>()
  const [tokenAllocation, setTokenAllocation] = useState<TTokenAllocation>()
  const [originalVesting, setOriginalVesting] = useState<OriginalVestingResponse>()
  const [currentVestingPeriod, setCurrentVestingPeriod] = useState<Period>()
  const [vestingAccountInfo, setVestingAccountInfo] = useState<VestingAccountInfo>()
  const [isLoading, setIsLoading] = useState(false)

  const fetchBalance = useCallback(async () => {
    setIsLoading(true)
    setError(undefined)
    try {
      const balance = await invoke('get_balance')
      setBalance(balance as Balance)
    } catch (error) {
      setError(error as string)
    } finally {
      setTimeout(() => {
        setIsLoading(false)
      }, 1000)
    }
  }, [])

  const fetchTokenAllocation = async () => {
    setIsLoading(true)
    if (address) {
      try {
        const [
          originalVestingValue,
          vestingCoins,
          vestedCoins,
          lockedCoins,
          spendableCoins,
          currentVestingPeriod,
          vestingAccountInfo,
        ] = await Promise.all([
          getOriginalVesting(address),
          getVestingCoins(address),
          getVestedCoins(address),
          getLockedCoins(address),
          getSpendableCoins(address),
          getCurrentVestingPeriod(address),
          getVestingAccountInfo(address),
        ])
        setOriginalVesting(originalVestingValue)
        setCurrentVestingPeriod(currentVestingPeriod)
        setTokenAllocation({
          vesting: vestingCoins.amount,
          vested: vestedCoins.amount,
          locked: lockedCoins.amount,
          spendable: spendableCoins.amount,
        })
        setVestingAccountInfo(vestingAccountInfo)
      } catch (e) {
        clearTokenAllocation()
        clearOriginalVesting()
        console.error(e)
      }
    }
    setIsLoading(false)
  }

  const clearBalance = () => setBalance(undefined)
  const clearTokenAllocation = () => setTokenAllocation(undefined)
  const clearOriginalVesting = () => setOriginalVesting(undefined)

  const clearAll = () => {
    clearBalance()
    clearTokenAllocation()
    clearOriginalVesting()
  }

  useEffect(() => {
    handleRefresh(address)
  }, [address])

  const handleRefresh = (address?: string) => {
    if (address) {
      fetchBalance()
      fetchTokenAllocation()
    } else {
      clearAll()
    }
  }

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
  }
}
