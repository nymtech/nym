import React from 'react'
import CosmosKitProvider from '@/app/context/cosmos-kit'
import { WalletProvider } from '@/app/context/wallet'
import { NetworkExplorerThemeProvider } from '@/app/theme'
import { MainContextProvider } from '@/app/context/main'

const Providers = ({ children }: { children: React.ReactNode }) => {
  return (
    <MainContextProvider>
      <CosmosKitProvider>
        <WalletProvider>
          <NetworkExplorerThemeProvider>
            {children}
          </NetworkExplorerThemeProvider>
        </WalletProvider>
      </CosmosKitProvider>
    </MainContextProvider>
  )
}

export { Providers }
