import type { DecCoin } from './DecCoin';
import type { DelegationWithEverything } from './DelegationWithEverything';

export interface DelegationsSummaryResponse {
  delegations: Array<DelegationWithEverything>;
  total_delegations: DecCoin;
  total_rewards: DecCoin;
}
