import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api'
import { Balance, Coin, OriginalVestingResponse } from '../types'
import { getVestingCoins, getVestedCoins, getLockedCoins, getSpendableCoins, getOriginalVesting } from '../requests'

type TTokenAllocation = {
  [key in 'vesting' | 'vested' | 'locked' | 'spendable' ]: Coin['amount']
}

export type TUseuserBalance = {
  error?: string
  balance?: Balance
  tokenAllocation?: TTokenAllocation
  originalVesting?: OriginalVestingResponse
  isLoading: boolean
  fetchBalance: () => void
  clearBalance: () => void
  fetchTokenAllocation: () => void
}

export const useGetBalance = (address?: string): TUseuserBalance => {
  const [balance, setBalance] = useState<Balance>()
  const [error, setError] = useState<string>()
  const [tokenAllocation, setTokenAllocation] = useState<TTokenAllocation>()
  const [originalVesting, setOriginalVesting] = useState<OriginalVestingResponse>()
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
        const [originalVestingValue, vestingCoins, vestedCoins, lockedCoins, spendableCoins] = await Promise.all([
          getOriginalVesting(address),
          getVestingCoins(address),
          getVestedCoins(address),
          getLockedCoins(address),
          getSpendableCoins(address),
        ])

        setOriginalVesting(originalVestingValue)

        setTokenAllocation({
          vesting: vestingCoins.amount,
          vested: vestedCoins.amount,
          locked: lockedCoins.amount,
          spendable: spendableCoins.amount,
        })
      } catch (e) {
        clearTokenAllocation()
        clearOriginalVesting()
        console.log(e)
      }
    }
    setIsLoading(false)
  }

  const clearBalance = () => setBalance(undefined)
  const clearTokenAllocation = () => setTokenAllocation(undefined)
  const clearOriginalVesting = () => setOriginalVesting(undefined)

  useEffect(() => {
    handleRefresh(address)
  }, [address])

  const handleRefresh = (address?: string) => {
    if (address) {
      fetchBalance()
      fetchTokenAllocation()
    } else {
      clearBalance()
      clearTokenAllocation()
    }
  }

  return {
    error,
    isLoading,
    balance,
    tokenAllocation,
    originalVesting,
    fetchBalance,
    clearBalance,
    fetchTokenAllocation,
  }
}
