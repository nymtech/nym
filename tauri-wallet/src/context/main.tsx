import { invoke } from '@tauri-apps/api/tauri'
import React, { createContext, useEffect, useState } from 'react'
import { useHistory } from 'react-router-dom'

type TClientContext = {
  isLoggedIn: boolean
  address: string
  balance?: string
  balanceLoading: boolean
  balanceError?: string
  logIn: () => void
  logOut: () => void
  getBalance: () => void
}

export const ClientContext = createContext({} as TClientContext)

export const ClientContextProvider = ({
  children,
}: {
  children: React.ReactNode
}) => {
  const [isLoggedIn, setIsLoggedIn] = useState(false)
  const [balance, setBalance] = useState<string>()
  const [balanceError, setBalanceError] = useState<string>()
  const [balanceLoading, setBalanceLoading] = useState(false)

  const history = useHistory()

  const getBalance = () => {
    setBalanceLoading(true)
    setBalanceError(undefined)
    invoke('get_balance')
      .then((balance) => {
        setBalance(balance as string)
      })
      .catch((e) => setBalanceError(e))
    setBalanceLoading(false)
  }

  const logIn = () => setIsLoggedIn(true)
  const logOut = () => setIsLoggedIn(false)

  useEffect(() => {
    !isLoggedIn ? history.push('/signin') : history.push('/bond')
  }, [isLoggedIn])

  return (
    <ClientContext.Provider
      value={{
        isLoggedIn,
        balance,
        balanceError,
        address: 'punk1s63y29jf8f3ft64z0vh80g3c76ty8lnyr74eur',
        balanceLoading,
        logIn,
        logOut,
        getBalance,
      }}
    >
      {children}
    </ClientContext.Provider>
  )
}
