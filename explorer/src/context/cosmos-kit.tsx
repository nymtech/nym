import React from 'react';
import { ChainProvider } from '@cosmos-kit/react';
import { wallets as keplr } from '@cosmos-kit/keplr';
import { wallets as ledger } from '@cosmos-kit/ledger';
import { assets, chains } from 'chain-registry';

const CosmosKitProvider = ({ children }: { children: React.ReactNode }) => (
  <ChainProvider chains={chains} assetLists={assets} wallets={[...keplr, ...ledger]}>
    {children}
  </ChainProvider>
);

export default CosmosKitProvider;
