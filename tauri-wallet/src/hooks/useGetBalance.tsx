import { useState } from 'react'
import { invoke } from '@tauri-apps/api'
import { Balance } from '../types'

export const useGetBalance = () => {
  const [balance, setBalance] = useState<Balance>()
  const [error, setEror] = useState<string>()
  const [isLoading, setIsLoading] = useState(false)

  const getBalance = () => {
    setIsLoading(true)
    setEror(undefined)
    invoke('get_balance')
      .then((balance) => {
        setBalance(balance as Balance)
      })
      .catch((e) => setEror(e))
    setTimeout(() => {
      setIsLoading(false)
    }, 1000)
  }

  return {
    error,
    isLoading,
    balance,
    getBalance,
  }
}
