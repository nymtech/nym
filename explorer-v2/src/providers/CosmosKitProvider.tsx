"use client";

import type { AssetList, Chain } from "@chain-registry/types";
import { wallets as keplr } from "@cosmos-kit/keplr-extension";
import { ChainProvider } from "@cosmos-kit/react";
import { assets, chains } from "chain-registry";
import React from "react";

const nymSandbox: Chain = {
  chain_type: "cosmos",
  chain_name: "sandbox",
  chain_id: "sandbox",
  bech32_prefix: "n",
  network_type: "devnet",
  pretty_name: "Nym Sandbox",
  status: "live",
  slip44: 118,
  apis: {
    rpc: [
      {
        address: "https://rpc.sandbox.nymtech.net",
      },
    ],
  },
};

const nymSandboxAssets = {
  chain_name: "sandbox",
  assets: [
    {
      name: "Nym",
      base: "unym",
      symbol: "NYM",
      display: "NYM",
      denom_units: [],
    },
  ],
};

const CosmosKitProvider = ({ children }: { children: React.ReactNode }) => {
  // Only use the nyx chains
  const chainsWithNyx = React.useMemo(() => {
    const nyx = chains.find((chain) => chain.chain_id === "nyx");

    const nyxRpc = nyx
      ? {
          ...nyx,
          apis: {
            rpc: [
              {
                address: "https://rpc.nymtech.net/",
              },
            ],
          },
        }
      : nymSandbox;

    return nyx ? [nymSandbox, nyxRpc] : [nymSandbox];
  }, []);

  // Only use the nyx assets
  const assetsWithNyx = React.useMemo(() => {
    const nyx = assets.find((asset) => asset.chain_name === "nyx");

    return nyx ? [nymSandboxAssets, nyx] : [nymSandboxAssets];
  }, []) as AssetList[];

  return (
    <ChainProvider
      chains={chainsWithNyx}
      assetLists={assetsWithNyx}
      wallets={[...keplr]}
    >
      {children}
    </ChainProvider>
  );
};

export default CosmosKitProvider;
