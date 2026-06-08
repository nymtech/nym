import { orderBy as _orderBy } from 'lodash';
import { Order, SortingKeys } from 'src/components/Delegation/DelegationList';
import { TDelegations, isDelegation } from 'src/context/delegations';
import { isFullyUnbondedDelegation } from 'src/utils';

type MappedTypes = 'delegationValue' | 'operatorReward' | 'profitMarginValue' | 'operatorCostValue';

export const useSortDelegations = (delegationItems: TDelegations, order: Order, orderBy: SortingKeys) => {
  const unbondedDelegations = delegationItems.filter(
    (delegation) => isDelegation(delegation) && isFullyUnbondedDelegation(delegation),
  );
  const delegations = delegationItems.filter(
    (delegation) => !(isDelegation(delegation) && isFullyUnbondedDelegation(delegation)),
  );

  // example of a mapped type in typescript

  const mapOrderBy = (key: SortingKeys): MappedTypes | SortingKeys => {
    switch (key) {
      case 'amount':
        return 'delegationValue';
      case 'unclaimed_rewards':
        return 'operatorReward';
      case 'profit_margin_percent':
        return 'profitMarginValue';
      case 'operating_cost':
        return 'operatorCostValue';
      default:
        return key;
    }
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
