import React, { createContext, useEffect, useState } from 'react'
import { useHistory } from 'react-router-dom'
import { TClientDetails, TSignInWithMnemonic } from '../types'
import { TUseGetBalance, useGetBalance } from '../hooks/useGetBalance'

export const ADMIN_ADDRESS = 'punk1h3w4nj7kny5dfyjw2le4vm74z03v9vd4dstpu0'

type TClientContext = {
  clientDetails?: TClientDetails
  getBalance: TUseGetBalance
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
  const [mode, setMode] = useState<'light' | 'dark'>('dark')

  const history = useHistory()
  const getBalance = useGetBalance()

  useEffect(() => {
    !clientDetails ? history.push('/signin') : history.push('/balance')
  }, [clientDetails])

  const logIn = async (clientDetails: TSignInWithMnemonic) =>
    setClientDetails(clientDetails)

  const logOut = () => setClientDetails(undefined)

  const handleShowAdmin = () => setShowAdmin((show) => !show)

  return (
    <ClientContext.Provider
      value={{
        clientDetails,
        getBalance,
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
