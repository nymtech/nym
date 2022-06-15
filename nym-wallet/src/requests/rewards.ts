import { TransactionExecuteResult } from '@nymproject/types';
import { invokeWrapper } from './wrapper';

export const claimOperatorRewards = async () => invokeWrapper<TransactionExecuteResult[]>('claim_operator_reward');

export const compoundOperatorRewards = async () =>
  invokeWrapper<TransactionExecuteResult[]>('compound_operator_reward');

export const claimDelegatorRewards = async (mixIdentity: string) =>
  invokeWrapper<TransactionExecuteResult[]>('claim_locked_and_unlocked_delegator_reward', { mixIdentity });

export const compoundDelegatorRewards = async (mixIdentity: String) =>
  invokeWrapper<TransactionExecuteResult[]>('compound_locked_and_unlocked_delegator_reward', { mixIdentity });
