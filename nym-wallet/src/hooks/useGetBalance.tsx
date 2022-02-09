import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api'
import { Balance, Coin } from '../types'
import { getVestingCoins, getVestedCoins, getLockedCoins, minorToMajor, getSpendableCoins } from '../requests'

type TTokenAllocation = {
  [key in 'vesting' | 'vested' | 'locked' | 'spendable']: Coin['amount']
}

export type TUseuserBalance = {
  error?: string
  balance?: Balance
  tokenAllocation?: TTokenAllocation
  isLoading: boolean
  fetchBalance: () => void
  clearBalance: () => void
  fetchTokenAllocation: (address: string) => Promise<void>
}

export const useGetBalance = (address?: string): TUseuserBalance => {
  const [balance, setBalance] = useState<Balance>()
  const [error, setError] = useState<string>()
  const [tokenAllocation, setTokenAllocation] = useState<TTokenAllocation>()
  const [isLoading, setIsLoading] = useState(false)

  const fetchBalance = useCallback(async () => {
    setIsLoading(true)
    setError(undefined)
    invoke('get_balance')
      .then((balance) => {
        setBalance(balance as Balance)
      })
      .catch(setError)
    setTimeout(() => {
      setIsLoading(false)
    }, 1000)
  }, [])

  const fetchTokenAllocation = async (address: string) => {
    console.log(address)
    try {
      const [vestingCoins, vestedCoins, lockedCoins, spendableCoins] = await Promise.all([
        getVestingCoins(address),
        getVestedCoins(address),
        getLockedCoins(address),
        getSpendableCoins(address),
      ])

      const [vestingCoinsMajor, vestedCoinsMajor, lockedCoinsMajor, spendableCoinsMajor] = await Promise.all([
        minorToMajor(vestingCoins.amount),
        minorToMajor(vestedCoins.amount),
        minorToMajor(lockedCoins.amount),
        minorToMajor(spendableCoins.amount),
      ])

      setTokenAllocation({
        vesting: vestingCoinsMajor.amount,
        vested: vestedCoinsMajor.amount,
        locked: lockedCoinsMajor.amount,
        spendable: spendableCoinsMajor.amount,
      })
    } catch (e) {
      console.log(e)
      clearTokenAllocation()
    }
  }

  const clearBalance = () => setBalance(undefined)
  const clearTokenAllocation = () => setTokenAllocation(undefined)

  useEffect(() => {
    if (address) {
      fetchBalance()
      fetchTokenAllocation(address)
    } else {
      clearBalance()
      clearTokenAllocation()
    }
  }, [address])

  return {
    error,
    isLoading,
    balance,
    tokenAllocation,
    fetchBalance,
    fetchTokenAllocation,
    clearBalance,
  }
}
