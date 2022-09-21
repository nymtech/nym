import type { DelegationEvent } from './DelegationEvent';

export interface WrappedDelegationEvent {
  event: DelegationEvent;
  node_identity: string;
}
