import Big from 'big.js';
import { DecCoin } from '@nymproject/types';
import { TBondedNode } from 'src/context/bonding';
import { isMixnode, isNymNode } from 'src/types';

export type OperatorUnbondReturn = {
  pledge: DecCoin;
  operatorRewards: DecCoin | null;
  total: DecCoin;
  hasCompoundedRewards: boolean;
};

const toDisplayAmount = (amount: string): string => {
  try {
    return Big(amount).toFixed();
  } catch {
    return '0';
  }
};

const sumCoinAmounts = (a: string, b: string): string => {
  try {
    return Big(a).plus(b).toFixed();
  } catch {
    return toDisplayAmount(a);
  }
};

export const formatOperatorUnbondReturn = (node: TBondedNode): OperatorUnbondReturn => {
  const pledge: DecCoin = {
    amount: toDisplayAmount(node.bond.amount),
    denom: node.bond.denom,
  };

  const operatorRewards =
    (isMixnode(node) || isNymNode(node)) && node.operatorRewards
      ? {
          amount: toDisplayAmount(node.operatorRewards.amount),
          denom: node.operatorRewards.denom,
        }
      : null;

  const rewardsAmount = operatorRewards && Big(operatorRewards.amount).gt(0) ? operatorRewards.amount : '0';

  const total: DecCoin = {
    amount: sumCoinAmounts(pledge.amount, rewardsAmount),
    denom: pledge.denom,
  };

  return {
    pledge,
    operatorRewards: Big(rewardsAmount).gt(0) ? operatorRewards : null,
    total,
    hasCompoundedRewards: Big(rewardsAmount).gt(0),
  };
};

export const formatCoinDisplay = (coin: DecCoin): string => `${coin.amount} ${coin.denom.toUpperCase()}`;
