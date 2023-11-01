import { useCallback, useContext, useEffect, useState } from 'react';
import { Console } from '../utils/console';
import { AppContext } from '../context/main';
import { checkGatewayOwnership, checkMixnodeOwnership, getVestingPledgeInfo } from '../requests';
import { EnumNodeType, TNodeOwnership } from '../types';

const initial: TNodeOwnership = {
  hasOwnership: false,
  nodeType: undefined,
  vestingPledge: undefined,
};

export const useCheckOwnership = () => {
  const { clientDetails } = useContext(AppContext);

  const [ownership, setOwnership] = useState<TNodeOwnership>(initial);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>();

  const checkOwnership = useCallback(async () => {
    const status = { ...initial };
    try {
      const [ownsMixnode, ownsGateway] = await Promise.all([checkMixnodeOwnership(), checkGatewayOwnership()]);

      if (ownsMixnode) {
        status.hasOwnership = true;
        status.nodeType = EnumNodeType.mixnode;
        status.vestingPledge = await getVestingPledgeInfo({
          address: clientDetails?.client_address!,
          type: EnumNodeType.mixnode,
        });
      }

      if (ownsGateway) {
        status.hasOwnership = true;
        status.nodeType = EnumNodeType.gateway;
        status.vestingPledge = await getVestingPledgeInfo({
          address: clientDetails?.client_address!,
          type: EnumNodeType.gateway,
        });
      }

      setOwnership(status);
    } catch (e) {
      Console.error(e as string);
      setError(e as string);
      setOwnership(initial);
    } finally {
      setIsLoading(false);
    }
  }, [clientDetails]);

  useEffect(() => {
    checkOwnership();
  }, [clientDetails]);

  return { isLoading, error, ownership, checkOwnership };
};
