import {
  DelegationWithEverything,
  DelegationsSummaryResponse,
  TransactionExecuteResult,
  MajorCurrencyAmount,
} from '@nymproject/types';
import { invokeWrapper } from './wrapper';

export const getMixNodeDelegationsForCurrentAccount = async () =>
  invokeWrapper<DelegationWithEverything[]>('get_all_mix_delegations');

export const getDelegationSummary = async () => invokeWrapper<DelegationsSummaryResponse>('get_delegation_summary');

export const undelegateFromMixnode = async (identity: string) =>
  invokeWrapper<TransactionExecuteResult>('undelegate_from_mixnode', { identity });

export const delegateToMixnode = async ({ identity, amount }: { identity: string; amount: MajorCurrencyAmount }) =>
  invokeWrapper<TransactionExecuteResult>('delegate_to_mixnode', { identity, amount });
