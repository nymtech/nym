import { useCallback, useState } from 'react'
import { invoke } from '@tauri-apps/api'
import { Balance } from '../types'

export type TUseuserBalance = {
  error?: string
  balance?: Balance
  isLoading: boolean
  fetchBalance: () => void
  clearBalance: () => void
}

export const useGetBalance = (): TUseuserBalance => {
  const [balance, setBalance] = useState<Balance>()
  const [error, setError] = useState<string>()
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

  const clearBalance = () => setBalance(undefined)

  return {
    error,
    isLoading,
    balance,
    fetchBalance,
    clearBalance,
  }
}
