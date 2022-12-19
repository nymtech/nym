import { Fee, FeeDetails, RewardingParams, TransactionExecuteResult } from '@nymproject/types';
import { invokeWrapper } from './wrapper';

export const claimOperatorReward = async (fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('claim_operator_reward', { fee });

export const claimDelegatorRewards = async (mixId: number, fee?: FeeDetails) =>
  invokeWrapper<TransactionExecuteResult[]>('claim_locked_and_unlocked_delegator_reward', {
    mixId,
    fee: fee?.fee,
  });

export const getCurrentRewardingParameter = async () =>
  invokeWrapper<RewardingParams>('get_current_rewarding_parameters', {});
