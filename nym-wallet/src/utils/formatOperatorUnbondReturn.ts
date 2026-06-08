import Big from 'big.js';
import { DecCoin } from '@nymproject/types';
import { TBondedNode } from 'src/context/bonding';
import { isMixnode, isNymNode } from 'src/types';

export type OperatorUnbondReturn = {
  pledge: DecCoin;
  operatorRewards: DecCoin | null;
  total: DecCoin;
  hasCompoundedRewards: boolean;
  parseError?: string;
};

const toDisplayAmount = (amount: string): { value: string; error?: string } => {
  try {
    return { value: Big(amount).toFixed() };
  } catch {
    return { value: amount, error: `Could not parse amount: "${amount}"` };
  }
};

const sumCoinAmounts = (a: string, b: string): { value: string; error?: string } => {
  try {
    return { value: Big(a).plus(b).toFixed() };
  } catch {
    return { value: a, error: `Could not sum amounts: "${a}" + "${b}"` };
  }
};

export const formatOperatorUnbondReturn = (node: TBondedNode): OperatorUnbondReturn => {
  const errors: string[] = [];

  const pledgeParsed = toDisplayAmount(node.bond.amount);
  if (pledgeParsed.error) errors.push(pledgeParsed.error);
  const pledge: DecCoin = { amount: pledgeParsed.value, denom: node.bond.denom };

  const rawRewards = (isMixnode(node) || isNymNode(node)) && node.operatorRewards ? node.operatorRewards : null;
  let operatorRewards: DecCoin | null = null;
  if (rawRewards) {
    const rewardsParsed = toDisplayAmount(rawRewards.amount);
    if (rewardsParsed.error) errors.push(rewardsParsed.error);
    operatorRewards = { amount: rewardsParsed.value, denom: rawRewards.denom };
  }

  let hasCompoundedRewards = false;
  try {
    hasCompoundedRewards = Boolean(operatorRewards) && Big(operatorRewards!.amount).gt(0);
  } catch {
    errors.push(`Could not evaluate rewards amount: "${operatorRewards?.amount}"`);
  }

  const rewardsAmount = hasCompoundedRewards ? operatorRewards!.amount : '0';
  const totalParsed = sumCoinAmounts(pledge.amount, rewardsAmount);
  if (totalParsed.error) errors.push(totalParsed.error);
  const total: DecCoin = { amount: totalParsed.value, denom: pledge.denom };

  return {
    pledge,
    operatorRewards: hasCompoundedRewards ? operatorRewards : null,
    total,
    hasCompoundedRewards,
    parseError: errors.length > 0 ? errors.join('; ') : undefined,
  };
};

export const formatCoinDisplay = (coin: DecCoin): string => `${coin.amount} ${coin.denom.toUpperCase()}`;
