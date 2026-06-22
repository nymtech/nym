import { useContext, useMemo } from 'react';
import { AppContext } from 'src/context/main';
import { NodeId } from 'src/types/families';
import { isMixnode, isNymNode } from 'src/types/global';
import useGetNodeDetails from './useGetNodeDetails';

/**
 * Controlled node ids for the connected account, worked out without the
 * Bonding/Families providers. Use this where those route-scoped providers are not
 * mounted (e.g. the always-on nav). Returns [] until the bonded node resolves, so
 * callers degrade to "no nodes" cleanly, and the no-Tauri mock harness simply
 * shows nothing rather than blowing up.
 */
export const useControlledNodeIds = (): NodeId[] => {
  const { clientDetails, network } = useContext(AppContext);
  const { bondedNode } = useGetNodeDetails(clientDetails?.client_address, network);

  return useMemo<NodeId[]>(() => {
    if (!bondedNode) return [];
    if (isNymNode(bondedNode)) return [bondedNode.nodeId];
    if (isMixnode(bondedNode)) return [bondedNode.mixId];
    return [];
  }, [bondedNode]);
};
