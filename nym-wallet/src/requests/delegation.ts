import {
  DelegationWithEverything,
  DelegationsSummaryResponse,
  TransactionExecuteResult,
  DecCoin,
  FeeDetails,
  Fee,
} from '@nymproject/types';
import { invokeWrapper } from './wrapper';

export const getMixNodeDelegationsForCurrentAccount = async () =>
  invokeWrapper<DelegationWithEverything[]>('get_all_mix_delegations');

export const getDelegationSummary = async () => invokeWrapper<DelegationsSummaryResponse>('get_delegation_summary');

export const undelegateFromMixnode = async (mix_id: number, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('undelegate_from_mixnode', { mix_id, fee });

export const undelegateAllFromMixnode = async (
  mix_id: number,
  usesVestingContractTokens: boolean,
  fee_liquid?: FeeDetails,
  fee_vesting?: FeeDetails,
) =>
  invokeWrapper<TransactionExecuteResult[]>('undelegate_all_from_mixnode', {
    mix_id,
    usesVestingContractTokens,
    fee_liquid,
    fee_vesting,
  });

export const delegateToMixnode = async (mix_id: number, amount: DecCoin, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('delegate_to_mixnode', { mix_id, amount, fee });
