'use client'

import React, {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react'
import { useChain } from '@cosmos-kit/react'
import { Wallet } from '@cosmos-kit/core'
import { unymToNym } from '@/app/utils/currency'
import { useNymClient } from '@/app/hooks'
import {
  MixnetClient,
  MixnetQueryClient,
} from '@nymproject/contract-clients/Mixnet.client'
import { COSMOS_KIT_USE_CHAIN } from '@/app/api/constants'

interface WalletState {
  balance: { status: 'loading' | 'success'; data?: string }
  address?: string
  isWalletConnected: boolean
  isWalletConnecting: boolean
  wallet?: Wallet
  nymClient?: MixnetClient
  nymQueryClient?: MixnetQueryClient
  connectWallet: () => Promise<void>
  disconnectWallet: () => Promise<void>
}

export const WalletContext = createContext<WalletState>({
  address: undefined,
  balance: { status: 'loading', data: undefined },
  isWalletConnected: false,
  isWalletConnecting: false,
  nymClient: undefined,
  nymQueryClient: undefined,
  connectWallet: async () => {
    throw new Error('Please connect your wallet')
  },
  disconnectWallet: async () => {
    throw new Error('Please connect your wallet')
  },
})

export const WalletProvider = ({ children }: { children: React.ReactNode }) => {
  const [balance, setBalance] = useState<WalletState['balance']>({
    status: 'loading',
    data: undefined,
  })

  const {
    connect,
    disconnect,
    wallet,
    address,
    isWalletConnected,
    isWalletConnecting,
    getCosmWasmClient,
  } = useChain(COSMOS_KIT_USE_CHAIN)

  const { nymClient, nymQueryClient } = useNymClient(address)

  const getBalance = async (walletAddress: string) => {
    const account = await getCosmWasmClient()
    const uNYMBalance = await account.getBalance(walletAddress, 'unym')
    const NYMBalance = unymToNym(uNYMBalance.amount)

    return NYMBalance
  }

  const init = async (walletAddress: string) => {
    const walletBalance = await getBalance(walletAddress)
    setBalance({ status: 'success', data: walletBalance })
  }

  useEffect(() => {
    if (isWalletConnected && address) {
      init(address)
    }
  }, [address, isWalletConnected])

  const handleConnectWallet = async () => {
    await connect()
  }

  const handleDisconnectWallet = async () => {
    await disconnect()
    setBalance({ status: 'loading', data: undefined })
  }

  const contextValue: WalletState = useMemo(
    () => ({
      address,
      balance,
      wallet,
      isWalletConnected,
      isWalletConnecting,
      nymClient,
      nymQueryClient,
      connectWallet: handleConnectWallet,
      disconnectWallet: handleDisconnectWallet,
    }),
    [
      address,
      balance,
      wallet,
      isWalletConnected,
      isWalletConnecting,
      nymClient,
      nymQueryClient,
    ]
  )

  return (
    <WalletContext.Provider value={contextValue}>
      {children}
    </WalletContext.Provider>
  )
}

export const useWalletContext = () => useContext(WalletContext)
