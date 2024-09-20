import React, { useEffect, useState } from 'react';
import { TBondedGateway, TBondedMixnode, TNymNode } from 'src/context';
import { getNymNodeBondDetails } from 'src/requests';
import { getGatewayDetails } from 'src/requests/gatewayDetails';
import { getMixnodeDetails } from 'src/requests/mixnodeDetails';

type TNode = TBondedMixnode | TBondedGateway | TNymNode | null;

const useGetNodeDetails = (clientAddress: string) => {
  const [bondedNode, setBondedNode] = useState<TNode>(null);

  const getNodeDetails = async () => {
    // Check if the address has a NymNode bonded
    const nymNode = await getNymNodeBondDetails();
    if (nymNode) {
      setBondedNode({ nodeId: nymNode.bond_information.node_id });
      return;
    }

    // Check if the address has a Mixnode bonded
    const mixnode = await getMixnodeDetails(clientAddress);
    if (mixnode) {
      setBondedNode(mixnode);
    }

    // Check if the address has a Gateway bonded
    const gateway = await getGatewayDetails();
    if (gateway) {
      setBondedNode(gateway);
    }
  };

  useEffect(() => {
    getNodeDetails();
  }, [clientAddress]);

  return {
    bondedNode,
  };
};

export default useGetNodeDetails;
