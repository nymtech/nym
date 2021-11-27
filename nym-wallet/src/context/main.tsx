import React, { createContext, useEffect, useState } from 'react'
import { useHistory } from 'react-router-dom'
import { TClientDetails, TSignInWithMnemonic } from '../types'
import { TUseuserBalance, useGetBalance } from '../hooks/useGetBalance'

export const ADMIN_ADDRESS = 'punk1h3w4nj7kny5dfyjw2le4vm74z03v9vd4dstpu0'

type TClientContext = {
  clientDetails?: TClientDetails
  userBalance: TUseuserBalance
  showAdmin: boolean
  mode: 'light' | 'dark'
  handleShowAdmin: () => void
  logIn: (clientDetails: TSignInWithMnemonic) => void
  logOut: () => void
}

export const ClientContext = createContext({} as TClientContext)

export const ClientContextProvider = ({
  children,
}: {
  children: React.ReactNode
}) => {
  const [clientDetails, setClientDetails] = useState<TClientDetails>()
  const [showAdmin, setShowAdmin] = useState(false)
  const [mode, setMode] = useState<'light' | 'dark'>('light')

  const history = useHistory()
  const userBalance = useGetBalance()

  useEffect(() => {
    if (!clientDetails) {
      history.push('/signin')
    } else {
      userBalance.fetchBalance()
      history.push('/balance')
    }
  }, [clientDetails, userBalance.fetchBalance])

  const logIn = async (clientDetails: TSignInWithMnemonic) =>
    setClientDetails(clientDetails)

  const logOut = () => {
    setClientDetails(undefined)
    userBalance.clearBalance()
  }

  const handleShowAdmin = () => setShowAdmin((show) => !show)

  return (
    <ClientContext.Provider
      value={{
        clientDetails,
        userBalance,
        showAdmin,
        mode,
        handleShowAdmin,
        logIn,
        logOut,
      }}
    >
      {children}
    </ClientContext.Provider>
  )
}
