import { useState } from 'react'
import { invoke } from '@tauri-apps/api'
import { Balance } from '../types'

export type TUseGetBalance = {
  error?: string
  balance?: Balance
  isLoading: boolean
  fetchBalance: () => void
}

export const useGetBalance = (): TUseGetBalance => {
  const [balance, setBalance] = useState<Balance>()
  const [error, setError] = useState<string>()
  const [isLoading, setIsLoading] = useState(false)

  const fetchBalance = () => {
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
  }

  return {
    error,
    isLoading,
    balance,
    fetchBalance,
  }
}
