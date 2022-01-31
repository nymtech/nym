import React, { createContext, useEffect, useState } from 'react'
import { Account, Network, TCurrency, TMixnodeBondDetails } from '../types'
import { TUseuserBalance, useGetBalance } from '../hooks/useGetBalance'
import { config } from '../../config'
import { getMixnodeBondDetails, selectNetwork } from '../requests'
import { currencyMap } from '../utils'

export const { ADMIN_ADDRESS } = config

export const urls = (network: Network) => ({
  blockExplorer: `https://${network}-blocks.nymtech.net`,
  networkExplorer: `https://${network}-explorer.nymtech.net`,
})

type TClientContext = {
  mode: 'light' | 'dark'
  clientDetails?: Account
  mixnodeDetails?: TMixnodeBondDetails | null
  userBalance: TUseuserBalance
  showAdmin: boolean
  showSettings: boolean
  network: Network
  currency?: TCurrency
  switchNetwork: (network: Network) => void
  getBondDetails: () => Promise<void>
  handleShowSettings: () => void
  handleShowAdmin: () => void
  logIn: () => void
  logOut: () => void
}

export const ClientContext = createContext({} as TClientContext)

export const ClientContextProvider = ({ children }: { children: React.ReactNode }) => {
  const [clientDetails, setClientDetails] = useState<Account>()
  const [mixnodeDetails, setMixnodeDetails] = useState<TMixnodeBondDetails | null>()
  const [network, setNetwork] = useState<Network>('SANDBOX')
  const [currency, setCurrency] = useState<TCurrency>()
  const [showAdmin, setShowAdmin] = useState(false)
  const [showSettings, setShowSettings] = useState(false)
  const [mode, setMode] = useState<'light' | 'dark'>('light')

  const userBalance = useGetBalance()

  useEffect(() => {
    if (clientDetails) {
      userBalance.fetchBalance()
    }
  }, [clientDetails, userBalance.fetchBalance])

  useEffect(() => {
    const refreshAccount = async () => {
      await logIn()
      userBalance.fetchBalance()
    }
    refreshAccount()
  }, [network])

  const logIn = async () => {
    const clientDetails = await selectNetwork(network)
    await getBondDetails()
    setClientDetails(clientDetails)
    setCurrency(currencyMap(network))
  }

  console.log({ clientDetails, mixnodeDetails })
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

  const switchNetwork = (network: Network) => setNetwork(network)

  return (
    <ClientContext.Provider
      value={{
        mode,
        clientDetails,
        mixnodeDetails,
        userBalance,
        showAdmin,
        showSettings,
        network,
        currency,
        switchNetwork,
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
