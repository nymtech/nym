import React, { FC } from 'react';

import { ChainProvider, useChainWallet } from '@cosmos-kit/react';
import { chains, assets } from 'chain-registry';
import { wallets } from '@cosmos-kit/keplr';

// Import this in your top-level route/layout
import '@interchain-ui/react/styles';

const WalletInner: FC = () => {
  const chainContext = useChainWallet('nyx', 'keplr');

  return <pre>{JSON.stringify(chainContext, null, 2)}</pre>;
};

export const Wallet: FC = () => (
  // TODO

  <ChainProvider
    chains={chains} // supported chains
    assetLists={assets} // supported asset lists
    wallets={wallets} // supported wallets
  >
    <div>This is a wallet</div>
    <WalletInner />
  </ChainProvider>
);
