import { invokeWrapper } from './wrapper';

export const claimDelegatorRewards = async (mixIdentity: string): Promise<void> =>
  invokeWrapper('claim_delegator_reward', { mixIdentity });

export const compoundDelegatorRewards = async (mixIdentity: String): Promise<void> =>
  invokeWrapper('compound_delegator_reward', { mixIdentity });
