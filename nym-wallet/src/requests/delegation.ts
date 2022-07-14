import {
  DelegationWithEverything,
  DelegationsSummaryResponse,
  TransactionExecuteResult,
  DecCoin,
  FeeDetails,
} from '@nymproject/types';
import { invokeWrapper } from './wrapper';

export const getMixNodeDelegationsForCurrentAccount = async () =>
  invokeWrapper<DelegationWithEverything[]>('get_all_mix_delegations');

export const getDelegationSummary = async () => invokeWrapper<DelegationsSummaryResponse>('get_delegation_summary');

export const undelegateFromMixnode = async (identity: string) =>
  invokeWrapper<TransactionExecuteResult>('undelegate_from_mixnode', { identity });

export const undelegateAllFromMixnode = async (
  identity: string,
  usesVestingContractTokens: boolean,
  fee?: FeeDetails,
) =>
  invokeWrapper<TransactionExecuteResult[]>('undelegate_all_from_mixnode', {
    identity,
    usesVestingContractTokens,
    fee: fee?.fee,
  });

export const delegateToMixnode = async ({ identity, amount }: { identity: string; amount: DecCoin }) =>
  invokeWrapper<TransactionExecuteResult>('delegate_to_mixnode', { identity, amount });
