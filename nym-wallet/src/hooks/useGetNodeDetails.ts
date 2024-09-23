import React, { useEffect, useState } from 'react';
import { TBondedNode } from 'src/context';
import { getGatewayDetails } from 'src/requests/gatewayDetails';
import { getMixnodeDetails } from 'src/requests/mixnodeDetails';
import { getNymNodeDetails } from 'src/requests/nymNodeDetails';
import { fireRequests, TauriReq } from 'src/utils';

const useGetNodeDetails = (clientAddress?: string, network?: string) => {
  const [bondedNode, setBondedNode] = useState<TBondedNode | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isError, setIsError] = useState(false);

  const getNodeDetails = async (clientAddress: string) => {
    setIsError(false);
    setBondedNode(null);
    setIsLoading(true);

    // Check if the address has a Nym node bonded
    const nymnode: TauriReq<typeof getNymNodeDetails> = {
      name: 'getNymNodeBondDetails',
      request: () => getNymNodeDetails(),
      onFulfilled: (value) => {
        if (value) {
          setBondedNode(value);
        }
      },
    };

    // Check if the address has a Mix node bonded
    const mixnode: TauriReq<typeof getMixnodeDetails> = {
      name: 'getMixnodeDetails',
      request: () => getMixnodeDetails(clientAddress),
      onFulfilled: (value) => {
        if (value) {
          setBondedNode(value);
        }
      },
    };

    // Check if the address has a Gateway bonded
    const gateway: TauriReq<typeof getGatewayDetails> = {
      name: 'getGatewayDetails',
      request: () => getGatewayDetails(),
      onFulfilled: (value) => {
        if (value) {
          setBondedNode(value);
        }
      },
    };

    await fireRequests([nymnode, mixnode, gateway]);

    setIsLoading(false);
  };

  useEffect(() => {
    if (clientAddress) {
      getNodeDetails(clientAddress);
    }
  }, [clientAddress, network]);

  return {
    bondedNode,
    isLoading,
    isError,
  };
};

export default useGetNodeDetails;
