import { FeeDetails, TransactionExecuteResult } from '@nymproject/types';
import { invokeWrapper } from './wrapper';

export const claimOperatorRewards = async () => invokeWrapper<TransactionExecuteResult[]>('claim_operator_reward');

export const compoundOperatorRewards = async () =>
  invokeWrapper<TransactionExecuteResult[]>('compound_operator_reward');

export const claimDelegatorRewards = async (mixIdentity: string, fee?: FeeDetails) =>
  invokeWrapper<TransactionExecuteResult[]>('claim_locked_and_unlocked_delegator_reward', {
    mixIdentity,
    fee: fee?.fee,
  });

export const compoundDelegatorRewards = async (mixIdentity: String, fee?: FeeDetails) =>
  invokeWrapper<TransactionExecuteResult[]>('compound_locked_and_unlocked_delegator_reward', {
    mixIdentity,
    fee: fee?.fee,
  });
