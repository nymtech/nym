import { currencyToString } from '../../../utils/currency';
import { useMixnodeContext } from '../../../context/mixnode';
import { ApiState, MixNodeEconomicDynamicsStatsResponse } from '../../../typeDefs/explorer-api';
import { EconomicsInfoRowWithIndex } from './types';

const selectionChance = (economicDynamicsStats: ApiState<MixNodeEconomicDynamicsStatsResponse> | undefined) => {
  const inclusionProbability = economicDynamicsStats?.data?.active_set_inclusion_probability;
  // TODO: when v2 will be deployed, remove cases: VeryHigh, Moderate and VeryLow
  switch (inclusionProbability) {
    case 'VeryLow':
      return 'Very Low';
    case 'VeryHigh':
      return 'Very High';
    case 'High':
    case 'Good':
    case 'Low':
    case 'Moderate':
      return inclusionProbability;
    default:
      return '-';
  }
};

export const EconomicsInfoRows = (): EconomicsInfoRowWithIndex => {
  const { economicDynamicsStats, mixNode } = useMixnodeContext();

  const estimatedNodeRewards =
    currencyToString((economicDynamicsStats?.data?.estimated_total_node_reward || '').toString()) || '-';
  const estimatedOperatorRewards =
    currencyToString((economicDynamicsStats?.data?.estimated_operator_reward || '').toString()) || '-';
  const stakeSaturation = economicDynamicsStats?.data?.stake_saturation || '-';
  const profitMargin = mixNode?.data?.mix_node.profit_margin_percent || '-';
  const avgUptime = economicDynamicsStats?.data?.current_interval_uptime;

  return {
    id: 1,
    estimatedTotalReward: {
      value: estimatedNodeRewards,
    },
    estimatedOperatorReward: {
      value: estimatedOperatorRewards,
    },
    selectionChance: {
      value: selectionChance(economicDynamicsStats),
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
