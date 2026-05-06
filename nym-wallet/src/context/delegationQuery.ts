import type { DelegationWithEverything, WrappedDelegationEvent } from '@nymproject/types';
import { getAllPendingDelegations, getDelegationSummary } from 'src/requests';
import { decCoinToDisplay } from 'src/utils';

export type DelegationSummaryBundle = {
  delegations: (DelegationWithEverything | WrappedDelegationEvent)[];
  pendingDelegations: WrappedDelegationEvent[];
  totalDelegations: string;
  totalRewards: string;
  totalDelegationsAndRewards: string;
};

export async function fetchDelegationSummaryQuery(): Promise<DelegationSummaryBundle> {
  const data = await getDelegationSummary();
  const pending = await getAllPendingDelegations();

  const pendingOnNewNodes = pending.filter((event) => {
    const some = data.delegations.some(({ node_identity }) => node_identity === event.node_identity);
    return !some;
  });
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
  const delegationsAndRewards = Number.isFinite(td) && Number.isFinite(tr) ? (td + tr).toFixed(6) : '0';

  return {
    delegations: [...items, ...pendingOnNewNodes],
    pendingDelegations: pending,
    totalDelegations: `${data.total_delegations.amount} ${data.total_delegations.denom}`,
    totalRewards: `${data.total_rewards.amount} ${data.total_rewards.denom}`,
    totalDelegationsAndRewards: `${delegationsAndRewards} ${data.total_delegations.denom}`,
  };
}

export { delegationQueryKeys } from './delegationQueryKeys';
