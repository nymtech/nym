'use client'

import React from 'react'
import { ChainProvider } from '@cosmos-kit/react'
import { wallets as keplr } from '@cosmos-kit/keplr-extension'
import { assets, chains } from 'chain-registry'
import { Chain, AssetList } from '@chain-registry/types'
import { VALIDATOR_BASE_URL } from '@/app/api/constants'

const nymSandbox: Chain = {
  chain_name: 'sandbox',
  chain_id: 'sandbox',
  bech32_prefix: 'n',
  network_type: 'devnet',
  pretty_name: 'Nym Sandbox',
  status: 'active',
  slip44: 118,
  apis: {
    rpc: [
      {
        address: 'https://rpc.sandbox.nymtech.net',
      },
    ],
  },
}

const nymSandboxAssets = {
  chain_name: 'sandbox',
  assets: [
    {
      name: 'Nym',
      base: 'unym',
      symbol: 'NYM',
      display: 'NYM',
      denom_units: [],
    },
  ],
}

const CosmosKitProvider = ({ children }: { children: React.ReactNode }) => {
  // Only use the nyx chains
  const chainsFixedUp = React.useMemo(() => {
    const nyx = chains.find((chain) => chain.chain_id === 'nyx')

    return nyx ? [nymSandbox, nyx] : [nymSandbox]
  }, [chains])

  // Only use the nyx assets
  const assetsFixedUp = React.useMemo(() => {
    const nyx = assets.find((asset) => asset.chain_name === 'nyx')

    return nyx ? [nymSandboxAssets, nyx] : [nymSandboxAssets]
  }, [assets]) as AssetList[]

  return (
    <ChainProvider
      chains={chainsFixedUp}
      assetLists={assetsFixedUp}
      wallets={[...keplr]}
      endpointOptions={{
        endpoints: {
          nyx: {
            rpc: [VALIDATOR_BASE_URL],
          },
        },
      }}
    >
      {children}
    </ChainProvider>
  )
}

export default CosmosKitProvider
