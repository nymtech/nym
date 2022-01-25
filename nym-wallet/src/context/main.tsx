import React, { createContext, useEffect, useState } from 'react'
import { useHistory } from 'react-router-dom'
import { TClientDetails, TMixnodeBondDetails, TSignInWithMnemonic } from '../types'
import { TUseuserBalance, useGetBalance } from '../hooks/useGetBalance'
import { config } from '../../config'
import { getMixnodeBondDetails } from '../requests'

export const { MAJOR_CURRENCY, MINOR_CURRENCY, ADMIN_ADDRESS, NETWORK_NAME } = config

export const urls = {
  blockExplorer: `https://${NETWORK_NAME}-blocks.nymtech.net`,
  networkExplorer: `https://${NETWORK_NAME}-explorer.nymtech.net`,
}

type TClientContext = {
  mode: 'light' | 'dark'
  clientDetails?: TClientDetails
  mixnodeDetails?: TMixnodeBondDetails | null
  userBalance: TUseuserBalance
  showAdmin: boolean
  showSettings: boolean
  getBondDetails: () => Promise<void>
  handleShowSettings: () => void
  handleShowAdmin: () => void
  logIn: (clientDetails: TSignInWithMnemonic) => void
  logOut: () => void
}

export const ClientContext = createContext({} as TClientContext)

export const ClientContextProvider = ({ children }: { children: React.ReactNode }) => {
  const [clientDetails, setClientDetails] = useState<TClientDetails>()
  const [mixnodeDetails, setMixnodeDetails] = useState<TMixnodeBondDetails | null>()
  const [showAdmin, setShowAdmin] = useState(false)
  const [showSettings, setShowSettings] = useState(false)
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

  const logIn = async (clientDetails: TSignInWithMnemonic) => {
    await getBondDetails()
    setClientDetails(clientDetails)
  }

  const logOut = () => {
    setClientDetails(undefined)
    userBalance.clearBalance()
  }

  const handleShowAdmin = () => setShowAdmin((show) => !show)
  const handleShowSettings = () => setShowSettings((show) => !show)

  const getBondDetails = async () => {
    const mixnodeDetails = await getMixnodeBondDetails()
    setMixnodeDetails(mixnodeDetails)
  }

  return (
    <ClientContext.Provider
      value={{
        mode,
        clientDetails,
        mixnodeDetails,
        userBalance,
        showAdmin,
        showSettings,
        getBondDetails,
        handleShowSettings,
        handleShowAdmin,
        logIn,
        logOut,
      }}
    >
      {children}
    </ClientContext.Provider>
  )
}
