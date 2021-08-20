import { useCallback, useContext, useState } from 'react'
import { ClientContext } from '../context/main'

export const useGetBalance = () => {
  const { client } = useContext(ClientContext)
  const [isLoading, setIsLoading] = useState(false)
  const [balanceCheckError, setBalanceCheckError] = useState(null)
  const [accountBalance, setAccountBalance] = useState<number>()

  const getBalance = useCallback(async () => {
    if (client) {
      setIsLoading(true)

      try {
        const value = await Promise.resolve(1000)
        setAccountBalance(value)
        setIsLoading(false)
      } catch (e) {
        setBalanceCheckError(e)
      }
    }
  }, [])

  return {
    balanceCheckError,
    isBalanceLoading: isLoading,
    accountBalance,
    printedBalance: '',
    getBalance,
  }
}
