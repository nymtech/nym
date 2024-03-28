import React from 'react';
import { ChainProvider } from '@cosmos-kit/react';
import { wallets as keplr } from '@cosmos-kit/keplr';
import { wallets as ledger } from '@cosmos-kit/ledger';
import { wallets as cosmosstation } from '@cosmos-kit/cosmostation';
import { assets, chains } from 'chain-registry';
import { Chain, AssetList } from '@chain-registry/types';

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
};

const nymSandboxAssets: AssetList = {
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
};

const CosmosKitProvider = ({ children }: { children: React.ReactNode }) => (
  <ChainProvider
    chains={[...chains, nymSandbox]}
    assetLists={[...assets, nymSandboxAssets]}
    wallets={[...keplr, ...ledger, ...cosmosstation]}
  >
    {children}
  </ChainProvider>
);

export default CosmosKitProvider;
