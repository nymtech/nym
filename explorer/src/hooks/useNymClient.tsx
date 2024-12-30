import { useEffect, useState } from 'react';
import { useChain } from '@cosmos-kit/react';
import { contracts } from '@nymproject/contract-clients';
import type { MixnetClient, MixnetQueryClient } from '@nymproject/contract-clients/Mixnet.client';
import { COSMOS_KIT_USE_CHAIN, NYM_MIXNET_CONTRACT } from '@src/api/constants';

export const useNymClient = (address?: string) => {
  const [nymClient, setNymClient] = useState<MixnetClient>();
  const [nymQueryClient, setNymQueryClient] = useState<MixnetQueryClient>();

  const { getCosmWasmClient, getSigningCosmWasmClient } = useChain(COSMOS_KIT_USE_CHAIN);

  useEffect(() => {
    if (address) {
      const init = async () => {
        const cosmWasmSigningClient = await getSigningCosmWasmClient();
        const cosmWasmClient = await getCosmWasmClient();
        // eslint-disable-next-line  @typescript-eslint/no-explicit-any
        const client = new contracts.Mixnet.MixnetClient(cosmWasmSigningClient as any, address, NYM_MIXNET_CONTRACT);
        // eslint-disable-next-line  @typescript-eslint/no-explicit-any
        const queryClient = new contracts.Mixnet.MixnetQueryClient(cosmWasmClient as any, NYM_MIXNET_CONTRACT);

        setNymClient(client);
        setNymQueryClient(queryClient);
      };

      init();
    }
  }, [address, getCosmWasmClient, getSigningCosmWasmClient]);

  return { nymClient, nymQueryClient };
};
