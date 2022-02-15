import React, { createContext, useEffect, useState } from 'react'
import { Account, Network, TCurrency, TMixnodeBondDetails } from '../types'
import { TUseuserBalance, useGetBalance } from '../hooks/useGetBalance'
import { config } from '../../config'
import { getMixnodeBondDetails, selectNetwork, signOut } from '../requests'
import { currencyMap } from '../utils'
import { useHistory } from 'react-router-dom'

export const { ADMIN_ADDRESS } = config

export const urls = (network: Network) =>
  network === 'MAINNET'
    ? {
        blockExplorer: 'https://blocks.nymtech.net',
        networkExplorer: 'https://explorer.nymtech.net',
      }
    : {
        blockExplorer: `https://${network}-blocks.nymtech.net`,
        networkExplorer: `https://${network}-explorer.nymtech.net`,
      }

type TClientContext = {
  mode: 'light' | 'dark'
  clientDetails?: Account
  mixnodeDetails?: TMixnodeBondDetails | null
  userBalance: TUseuserBalance
  showAdmin: boolean
  showSettings: boolean
  network?: Network
  currency?: TCurrency
  switchNetwork: (network: Network) => void
  getBondDetails: () => Promise<void>
  handleShowSettings: () => void
  handleShowAdmin: () => void
  logIn: (network: Network) => void
  logOut: () => void
}

export const ClientContext = createContext({} as TClientContext)

export const ClientContextProvider = ({ children }: { children: React.ReactNode }) => {
  const [clientDetails, setClientDetails] = useState<Account>()
  const [mixnodeDetails, setMixnodeDetails] = useState<TMixnodeBondDetails | null>()
  const [network, setNetwork] = useState<Network | undefined>()
  const [currency, setCurrency] = useState<TCurrency>()
  const [showAdmin, setShowAdmin] = useState(false)
  const [showSettings, setShowSettings] = useState(false)
  const [mode, setMode] = useState<'light' | 'dark'>('light')

  const userBalance = useGetBalance()
  const history = useHistory()

  useEffect(() => {
    if (clientDetails) {
      userBalance.fetchBalance()
    }
  }, [clientDetails, userBalance.fetchBalance])

  useEffect(() => {
    const refreshAccount = async () => {
      if (network) {
        await loadAccount(network)
        await getBondDetails()
        userBalance.fetchBalance()
      }
    }
    refreshAccount()
  }, [network])

  const logIn = async (network: Network) => {
    try {
      setNetwork(network)
      history.push('/balance')
    } catch (e) {
      console.log({ e })
    }
  }

  const loadAccount = async (network: Network) => {
    try {
      const clientDetails = await selectNetwork(network)
      setClientDetails(clientDetails)
    } catch (e) {
    } finally {
      setCurrency(currencyMap(network))
    }
  }

  const logOut = async () => {
    setClientDetails(undefined)
    setNetwork(undefined)
    await signOut()
  }

  const handleShowAdmin = () => setShowAdmin((show) => !show)
  const handleShowSettings = () => setShowSettings((show) => !show)

  const getBondDetails = async () => {
    setMixnodeDetails(undefined)
    try {
      const mixnodeDetails = await getMixnodeBondDetails()
      setMixnodeDetails(mixnodeDetails)
    } catch (e) {
      console.log(e)
    }
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
