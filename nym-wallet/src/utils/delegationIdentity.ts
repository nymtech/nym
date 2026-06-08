import type { DelegationWithEverything, WrappedDelegationEvent } from '@nymproject/types';

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

export function isPendingUndelegateWithRegistryMiss(
  item: Pick<WrappedDelegationEvent, 'node_identity' | 'event'>,
): boolean {
  return item.event.kind === 'Undelegate' && isUnbondedNodeIdentity(item.node_identity);
}

export function formatPendingDelegationLinkLabel(nodeIdentity: string, mixId: number): string {
  if (!nodeIdentity || isUnbondedNodeIdentity(nodeIdentity)) {
    return `Mix ${mixId}`;
  }
  return `${nodeIdentity.slice(0, 6)}...${nodeIdentity.slice(-6)}`;
}
