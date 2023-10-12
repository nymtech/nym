import { orderBy as _orderBy } from 'lodash';
import { Order, SortingKeys } from 'src/components/Delegation/DelegationList';
import { TDelegations, isDelegation } from 'src/context/delegations';

export const useSortDelegations = (delegationItems: TDelegations, order: Order, orderBy: SortingKeys) => {
  const unbondedDelegations = delegationItems.filter((delegation) => !delegation.node_identity);
  const delegations = delegationItems.filter((delegation) => delegation.node_identity);

  const mapOrderBy = (key: SortingKeys) => {
    if (key === 'amount') return 'delegationValue';
    if (key === 'unclaimed_rewards') return 'operatorReward';
    if (key === 'profit_margin_percent') return 'profitMarginValue';
    if (key === 'operating_cost') return 'operatorCostValue';
    return key;
  };

  const mapAndSort = (_items: TDelegations) => {
    const mapToNumberType = _items.map((item) =>
      isDelegation(item)
        ? {
            ...item,
            delegationValue: Number(item.amount.amount),
            operatorReward: Number(item.unclaimed_rewards?.amount),
            profitMarginValue: Number(item.cost_params?.profit_margin_percent),
            operatorCostValue: Number(item.cost_params?.interval_operating_cost.amount),
          }
        : item,
    );
    const ordered = _orderBy(mapToNumberType, mapOrderBy(orderBy), order).sort();
    return ordered;
  };

  return [...unbondedDelegations, ...mapAndSort(delegations)];
};
