import { decimalToPercentage, percentToDecimal } from '@nymproject/types';
import { computeMixnodeRewardEstimation } from '@src/requests';

const SCALE_FACTOR = 1_000_000;

export const computeStakeSaturation = (bond: string, delegations: string, stakeSaturationPoint: string) => {
  const res = ((+bond + +delegations) * SCALE_FACTOR) / +stakeSaturationPoint;
  return decimalToPercentage(res.toFixed(18).toString());
};

export const computeEstimate = async ({
  mixId,
  uptime,
  pledgeAmount,
  totalDelegation,
  profitMargin,
  operatorCost,
}: {
  mixId: number;
  uptime: string;
  pledgeAmount: string;
  totalDelegation: string;
  profitMargin: string;
  operatorCost: string;
}) => {
  const computedEstimate = await computeMixnodeRewardEstimation({
    mixId,
    performance: percentToDecimal(uptime),
    pledgeAmount: Math.round(+pledgeAmount * SCALE_FACTOR),
    totalDelegation: Math.round(+totalDelegation * SCALE_FACTOR),
    profitMarginPercent: percentToDecimal(profitMargin),
    intervalOperatingCost: { denom: 'unym', amount: Math.round(+operatorCost * SCALE_FACTOR).toString() },
  });

  return computedEstimate;
};

export const handleCalculatePeriodRewards = ({
  estimatedOperatorReward,
  estimatedDelegatorsReward,
}: {
  estimatedOperatorReward: string;
  estimatedDelegatorsReward: string;
}) => {
  const dailyOperatorReward = (+estimatedOperatorReward / SCALE_FACTOR) * 24; // epoch_reward * 1 epoch_per_hour * 24 hours
  const dailyDelegatorReward = (+estimatedDelegatorsReward / SCALE_FACTOR) * 24;
  const dailyTotal = dailyOperatorReward + dailyDelegatorReward;

  return {
    total: {
      daily: dailyTotal.toFixed(3).toString(),
      monthly: (dailyTotal * 30).toFixed(3).toString(),
      yearly: (dailyTotal * 365).toFixed(3).toString(),
    },
    operator: {
      daily: dailyOperatorReward.toFixed(3).toString(),
      monthly: (dailyOperatorReward * 30).toFixed(3).toString(),
      yearly: (dailyOperatorReward * 365).toFixed(3).toString(),
    },
    delegator: {
      daily: dailyDelegatorReward.toFixed(3).toString(),
      monthly: (dailyDelegatorReward * 30).toFixed(3).toString(),
      yearly: (dailyDelegatorReward * 365).toFixed(3).toString(),
    },
  };
};
