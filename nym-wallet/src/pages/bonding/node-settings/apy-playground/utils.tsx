export const handleCalculate = (operatorReward: number, delegatorsReward: number, totalReward: number) => {
  const dailyOperatorReward = (operatorReward / 1_000_000) * 24; // epoch_reward * 1 epoch_per_hour * 24 hours
  const dailyDelegatorReward = (delegatorsReward / 1_000_000) * 24;
  const dailyTotal = (totalReward / 1_000_000) * 24;
  console.log({ dailyOperatorReward });
  return {
    total: {
      daily: Math.round(dailyTotal).toString(),
      monthly: Math.round(dailyTotal * 30).toString(),
      yearly: Math.round(dailyTotal * 365).toString(),
    },
    operator: {
      daily: Math.round(dailyOperatorReward).toString(),
      monthly: Math.round(dailyOperatorReward * 30).toString(),
      yearly: Math.round(dailyOperatorReward * 365).toString(),
    },
    delegator: {
      daily: Math.round(dailyDelegatorReward).toString(),
      monthly: Math.round(dailyDelegatorReward * 30).toString(),
      yearly: Math.round(dailyDelegatorReward * 365).toString(),
    },
  };
};
