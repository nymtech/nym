import { currencyToString } from '../../../utils/currency';
import { useMixnodeContext } from '../../../context/mixnode';
import { EconomicsInfoRowWithIndex } from './types';

export const EconomicsInfoRows = (): EconomicsInfoRowWithIndex => {
  const { economicDynamicsStats, mixNode } = useMixnodeContext();

  const estimatedNodeRewards =
    currencyToString((economicDynamicsStats?.data?.estimated_total_node_reward || '').toString()) || '-';
  const estimatedOperatorRewards =
    currencyToString((economicDynamicsStats?.data?.estimated_operator_reward || '').toString()) || '-';
  // const activeSetProbability = economicDynamicsStats?.data?.active_set_inclusion_probability || '-';
  const stakeSaturation = economicDynamicsStats?.data?.stake_saturation || '-';
  const profitMargin = mixNode?.data?.mix_node.profit_margin_percent || '-';
  const avgUptime = economicDynamicsStats?.data?.current_interval_uptime;
  const selectionChance = 'High';

  return {
    id: 1,
    estimatedTotalReward: {
      value: estimatedNodeRewards,
    },
    estimatedOperatorReward: {
      value: estimatedOperatorRewards,
    },
    selectionChance: {
      value: selectionChance,
    },
    stakeSaturation: {
      progressBarValue: typeof stakeSaturation === 'number' ? stakeSaturation * 100 : 0,
      value: typeof stakeSaturation === 'number' ? `${(stakeSaturation * 100).toFixed(2)} %` : '-',
    },
    profitMargin: {
      value: profitMargin ? `${profitMargin} %` : '-',
    },
    avgUptime: {
      value: avgUptime ? `${avgUptime} %` : '-',
    },
  };
};
