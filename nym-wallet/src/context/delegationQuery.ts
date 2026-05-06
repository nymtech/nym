import type { DecCoin, DelegationWithEverything, WrappedDelegationEvent } from '@nymproject/types';
import { getAllPendingDelegations, getDelegationSummary } from 'src/requests';
import { decCoinToDisplay } from 'src/utils';

export type DelegationSummaryBundle = {
  delegations: (DelegationWithEverything | WrappedDelegationEvent)[];
  pendingDelegations: WrappedDelegationEvent[];
  totalDelegations: DecCoin;
  totalRewards: DecCoin;
  totalDelegationsAndRewards: DecCoin;
};

export async function fetchDelegationSummaryQuery(): Promise<DelegationSummaryBundle> {
  const data = await getDelegationSummary();
  const pending = await getAllPendingDelegations();

  const delegatedIdentities = new Set(data.delegations.map((d) => d.node_identity));
  const pendingOnNewNodes = pending.filter((event) => !delegatedIdentities.has(event.node_identity));
  const items = data.delegations.map((delegation) => ({
    ...delegation,
    amount: decCoinToDisplay(delegation.amount),
    unclaimed_rewards: delegation.unclaimed_rewards && decCoinToDisplay(delegation.unclaimed_rewards),
    cost_params: delegation.cost_params && {
      ...delegation.cost_params,
      interval_operating_cost: decCoinToDisplay(delegation.cost_params.interval_operating_cost),
    },
  }));

  const td = parseFloat(data.total_delegations.amount);
  const tr = parseFloat(data.total_rewards.amount);
  const combinedAmount = Number.isFinite(td) && Number.isFinite(tr) ? (td + tr).toFixed(6) : '0';

  return {
    delegations: [...items, ...pendingOnNewNodes],
    pendingDelegations: pending,
    totalDelegations: data.total_delegations,
    totalRewards: data.total_rewards,
    totalDelegationsAndRewards: {
      amount: combinedAmount,
      denom: data.total_delegations.denom,
    },
  };
}

export { delegationQueryKeys } from './delegationQueryKeys';
