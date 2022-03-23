import type { DelegationResult } from './delegationresult';
import type { PendingUndelegate } from './pendingundelegate';

export type DelegationEvent = { Delegate: DelegationResult } | { Undelegate: PendingUndelegate };
