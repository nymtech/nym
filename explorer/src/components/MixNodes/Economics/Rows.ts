import { currencyToString } from '../../../utils/currency';
import { useMixnodeContext } from '../../../context/mixnode';
import { DelegatorsInfoRowWithIndex } from './types';

export const delegatorsInfoRows = (): DelegatorsInfoRowWithIndex => {
  const { economicDynamicsStats, mixNode } = useMixnodeContext();

  const estimatedNodeRewards = economicDynamicsStats?.data?.estimated_total_node_reward || 0;
  const estimatedOperatorRewards = economicDynamicsStats?.data?.estimated_operator_reward || 0;
  const activeSetProbability = economicDynamicsStats?.data?.active_set_inclusion_probability || 0;
  const stakeSaturation = economicDynamicsStats?.data?.stake_saturation || 0;
  const profitMargin = mixNode?.data?.mix_node.profit_margin_percent || 0;

  return {
    id: 1,
    estimated_total_reward: {
      value: currencyToString(estimatedNodeRewards.toString()),
    },
    estimated_operator_reward: {
      value: currencyToString(estimatedOperatorRewards.toString()),
    },
    active_set_probability: {
      percentaje: activeSetProbability * 100,
      value: `${(activeSetProbability * 100).toFixed(2)} %`,
    },
    stake_saturation: {
      percentaje: stakeSaturation * 100,
      value: `${(stakeSaturation * 100).toFixed(2)} %`,
    },
    profit_margin: {
      value: `${profitMargin} %`,
    },
    avg_uptime: {
      value: `${economicDynamicsStats?.data?.current_interval_uptime} %`,
    },
  };
};
