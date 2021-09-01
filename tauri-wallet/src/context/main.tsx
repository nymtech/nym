import { invoke } from '@tauri-apps/api/tauri'
import React, { createContext, useEffect, useState } from 'react'
import { useHistory } from 'react-router-dom'
import { TBalance, TClientDetails } from '../types/global'

type TClientContext = {
  clientDetails?: TClientDetails
  balance?: TBalance
  balanceLoading: boolean
  balanceError?: string
  logIn: (clientDetails: TClientDetails) => void
  logOut: () => void
  getBalance: () => void
}

export const ClientContext = createContext({} as TClientContext)

export const ClientContextProvider = ({
  children,
}: {
  children: React.ReactNode
}) => {
  const [balance, setBalance] = useState<TBalance>()
  const [balanceError, setBalanceError] = useState<string>()
  const [balanceLoading, setBalanceLoading] = useState(false)
  const [clientDetails, setClientDetails] = useState<TClientDetails>()

  const history = useHistory()

  const getBalance = () => {
    setBalanceLoading(true)
    setBalanceError(undefined)
    invoke('get_balance')
      .then((balance) => {
        setBalance(balance as TBalance)
      })
      .catch((e) => setBalanceError(e))
    setTimeout(() => {
      setBalanceLoading(false)
    }, 1000)
  }

  const logIn = (clientDetails: TClientDetails) =>
    setClientDetails(clientDetails)

  const logOut = () => setClientDetails(undefined)

  useEffect(() => {
    !clientDetails ? history.push('/signin') : history.push('/bond')
  }, [clientDetails])

  return (
    <ClientContext.Provider
      value={{
        balance,
        balanceError,
        clientDetails,
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
