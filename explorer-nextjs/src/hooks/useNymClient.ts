"use client";

import { COSMOS_KIT_USE_CHAIN, NYM_MIXNET_CONTRACT } from "@/config";
import { useChain } from "@cosmos-kit/react";
import { contracts } from "@nymproject/contract-clients";
import type {
  MixnetClient,
  MixnetQueryClient,
} from "@nymproject/contract-clients/Mixnet.client";
import { useEffect, useState } from "react";

export const useNymClient = () => {
  const [nymClient, setNymClient] = useState<MixnetClient>();
  const [nymQueryClient, setNymQueryClient] = useState<MixnetQueryClient>();

  const { address, getCosmWasmClient, getSigningCosmWasmClient } =
    useChain(COSMOS_KIT_USE_CHAIN);

  useEffect(() => {
    if (address) {
      const init = async () => {
        const cosmWasmSigningClient = await getSigningCosmWasmClient();
        const cosmWasmClient = await getCosmWasmClient();

        const client = new contracts.Mixnet.MixnetClient(
          // eslint-disable-next-line  @typescript-eslint/no-explicit-any
          cosmWasmSigningClient as any,
          address,
          NYM_MIXNET_CONTRACT,
        );

        const queryClient = new contracts.Mixnet.MixnetQueryClient(
          // eslint-disable-next-line  @typescript-eslint/no-explicit-any
          cosmWasmClient as any,
          NYM_MIXNET_CONTRACT,
        );

        setNymClient(client);
        setNymQueryClient(queryClient);
      };

      init();
    }
  }, [address, getCosmWasmClient, getSigningCosmWasmClient]);

  return { nymClient, nymQueryClient, address };
};
