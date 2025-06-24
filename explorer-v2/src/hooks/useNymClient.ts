"use client";

import { useChain } from "@cosmos-kit/react";
import { contracts } from "@nymproject/contract-clients";
import type {
  MixnetClient,
  MixnetQueryClient,
} from "@nymproject/contract-clients/Mixnet.client";
import { useEffect, useState } from "react";
import {
  COSMOS_KIT_USE_CHAIN,
  NYM_MIXNET_CONTRACT,
  SANDBOX_MIXNET_CONTRACT_ADDRESS,
} from "../config";
import { useEnvironment } from "@/providers/EnvironmentProvider";

export const useNymClient = () => {
  const [nymClient, setNymClient] = useState<MixnetClient>();
  const [nymQueryClient, setNymQueryClient] = useState<MixnetQueryClient>();
  const { environment } = useEnvironment();
  const chain = environment === "mainnet" ? COSMOS_KIT_USE_CHAIN : "sandbox";
  const mixnetContractAddress =
    environment === "mainnet"
      ? NYM_MIXNET_CONTRACT
      : SANDBOX_MIXNET_CONTRACT_ADDRESS;

  const { address, getCosmWasmClient, getSigningCosmWasmClient } =
    useChain(chain);

  useEffect(() => {
    if (address) {
      const init = async () => {
        const cosmWasmSigningClient = await getSigningCosmWasmClient();
        const cosmWasmClient = await getCosmWasmClient();

        const client = new contracts.Mixnet.MixnetClient(
          // eslint-disable-next-line  @typescript-eslint/no-explicit-any
          cosmWasmSigningClient as any,
          address,
          mixnetContractAddress
        );

        const queryClient = new contracts.Mixnet.MixnetQueryClient(
          // eslint-disable-next-line  @typescript-eslint/no-explicit-any
          cosmWasmClient as any,
          mixnetContractAddress
        );

        setNymClient(client);
        setNymQueryClient(queryClient);
      };

      init();
    }
  }, [
    address,
    getCosmWasmClient,
    getSigningCosmWasmClient,
    mixnetContractAddress,
    environment,
  ]);

  return { nymClient, nymQueryClient, address };
};
