import type { DelegationWithEverything } from '@nymproject/types';

export const UNBONDED_NODE_IDENTITY_PREFIX = 'unbonded:';

export function isUnbondedNodeIdentity(nodeIdentity: string | undefined | null): boolean {
  if (!nodeIdentity) {
    return true;
  }
  return nodeIdentity.startsWith(UNBONDED_NODE_IDENTITY_PREFIX);
}

export function isFullyUnbondedDelegation(
  item: Pick<DelegationWithEverything, 'node_identity' | 'mixnode_is_unbonding'>,
): boolean {
  return isUnbondedNodeIdentity(item.node_identity);
}

export function formatUnbondedNodeLabel(mixId: number): string {
  return `Node unbonded (mix ${mixId})`;
}

export function formatDelegationNodeIdentityForDisplay(nodeIdentity: string, mixId: number): string {
  if (isUnbondedNodeIdentity(nodeIdentity)) {
    return formatUnbondedNodeLabel(mixId);
  }
  return nodeIdentity;
}
