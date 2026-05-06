import type { DelegationWithEverything, WrappedDelegationEvent } from '@nymproject/types';
import { getAllPendingDelegations } from 'src/requests';
import { getDelegationSummary } from 'src/requests/delegation';
import { decCoinToDisplay } from 'src/utils';

export const delegationQueryKeys = {
  all: ['nym-wallet', 'delegations'] as const,
  summary: (address: string) => [...delegationQueryKeys.all, 'summary', address] as const,
};

export type DelegationSummaryQueryResult = {
  delegations: (DelegationWithEverything | WrappedDelegationEvent)[];
  pendingDelegations: WrappedDelegationEvent[];
  totalDelegations: string;
  totalRewards: string;
  totalDelegationsAndRewards: string;
};

export async function fetchDelegationSummaryQuery(): Promise<DelegationSummaryQueryResult> {
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

  const delegationsAndRewards = (+data.total_delegations.amount + +data.total_rewards.amount).toFixed(6);

  return {
    delegations: [...items, ...pendingOnNewNodes],
    pendingDelegations: pending,
    totalDelegations: `${data.total_delegations.amount} ${data.total_delegations.denom}`,
    totalRewards: `${data.total_rewards.amount} ${data.total_rewards.denom}`,
    totalDelegationsAndRewards: `${delegationsAndRewards} ${data.total_delegations.denom}`,
  };
}
