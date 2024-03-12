import React from 'react';
import { ChainProvider } from '@cosmos-kit/react';
import { wallets as keplr } from '@cosmos-kit/keplr';
import { wallets as ledger } from '@cosmos-kit/ledger';
import { wallets as cosmosstation } from '@cosmos-kit/cosmostation';
import { assets, chains } from 'chain-registry';

const CosmosKitProvider = ({ children }: { children: React.ReactNode }) => (
  <ChainProvider chains={chains} assetLists={assets} wallets={[...keplr, ...ledger, ...cosmosstation]}>
    {children}
  </ChainProvider>
);

export default CosmosKitProvider;
