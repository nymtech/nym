import type { DelegationWithEverything, WrappedDelegationEvent } from '@nymproject/types';
import { isFullyUnbondedDelegation } from './delegationIdentity';

export type DelegationListItem = DelegationWithEverything | WrappedDelegationEvent;

const isPendingDelegationItem = (delegation: DelegationListItem): delegation is WrappedDelegationEvent =>
  'event' in delegation;

const isDelegationItem = (delegation: DelegationListItem): delegation is DelegationWithEverything =>
  'owner' in delegation;

export function shouldHideDelegationFromList(item: DelegationListItem): boolean {
  if (isDelegationItem(item)) {
    if (!item.node_identity || item.node_identity === '-' || item.node_identity === '...') {
      return true;
    }
  }

  if (isPendingDelegationItem(item)) {
    // Pending rows carry mix_id on the event; do not hide when bonded-registry identity lookup missed.
    return false;
  }

  return false;
}

export function filterVisibleDelegations(items: DelegationListItem[]): DelegationListItem[] {
  return items.filter((item) => !shouldHideDelegationFromList(item));
}

export function searchDelegations(
  delegations: DelegationWithEverything[],
  searchNeedle: string,
): DelegationWithEverything[] {
  const needle = searchNeedle.trim().toLowerCase();
  if (!needle) {
    return delegations;
  }
  return delegations.filter((d) => {
    const identity = d.node_identity?.toLowerCase() ?? '';
    const historical = d.historical_node_identity?.toLowerCase() ?? '';
    return identity.includes(needle) || historical.includes(needle) || String(d.mix_id).includes(needle);
  });
}

export function isUndelegateOnlyDelegation(item: DelegationWithEverything): boolean {
  return isFullyUnbondedDelegation(item);
}
