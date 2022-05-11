import type { DelegationWithEverything } from './DelegationWithEverything';
import type { MajorCurrencyAmount } from './Currency';

export interface DelegationsSummaryResponse {
  delegations: Array<DelegationWithEverything>;
  total_delegations: MajorCurrencyAmount;
  total_rewards: MajorCurrencyAmount;
}
