import { Fee, FeeDetails, TransactionExecuteResult } from '@nymproject/types';
import { invokeWrapper } from './wrapper';

export const claimOperatorReward = async (fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('claim_operator_reward', { fee });

export const compoundOperatorReward = async (fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('compound_operator_reward', { fee });

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
