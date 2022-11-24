import { currencyToString, unymToNym } from '../../../utils/currency';
import { useMixnodeContext } from '../../../context/mixnode';
import { ApiState, MixNodeEconomicDynamicsStatsResponse } from '../../../typeDefs/explorer-api';
import { EconomicsInfoRowWithIndex } from './types';
import { toPercentIntegerString } from '../../../utils';

const selectionChance = (economicDynamicsStats: ApiState<MixNodeEconomicDynamicsStatsResponse> | undefined) =>
  economicDynamicsStats?.data?.active_set_inclusion_probability || '-';

export const EconomicsInfoRows = (): EconomicsInfoRowWithIndex => {
  const { economicDynamicsStats, mixNode } = useMixnodeContext();

  const estimatedNodeRewards =
    currencyToString((economicDynamicsStats?.data?.estimated_total_node_reward || '').toString()) || '-';
  const estimatedOperatorRewards =
    currencyToString((economicDynamicsStats?.data?.estimated_operator_reward || '').toString()) || '-';
  const stakeSaturation = economicDynamicsStats?.data?.uncapped_saturation || '-';
  const profitMargin = mixNode?.data?.profit_margin_percent
    ? toPercentIntegerString(mixNode?.data?.profit_margin_percent)
    : '-';
  const avgUptime = economicDynamicsStats?.data?.current_interval_uptime;

  const opCost = mixNode?.data?.operating_cost;

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
    operatingCost: {
      value: opCost ? `${unymToNym(opCost.amount, 6)} NYM` : '-',
    },
    avgUptime: {
      value: avgUptime ? `${avgUptime} %` : '-',
    },
  };
};
