import React from 'react';
import { ChainProvider } from '@cosmos-kit/react';
import { wallets as keplr } from '@cosmos-kit/keplr';
import { assets, chains } from 'chain-registry';

const CosmosKitProvider = ({ children }: { children: React.ReactNode }) => (
  <ChainProvider chains={chains} assetLists={assets} wallets={[...keplr]}>
    {children}
  </ChainProvider>
);

export default CosmosKitProvider;
