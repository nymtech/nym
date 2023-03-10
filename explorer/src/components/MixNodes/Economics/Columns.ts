import { ColumnsType } from '../../DetailTable';

export const EconomicsInfoColumns: ColumnsType[] = [
  {
    field: 'estimatedTotalReward',
    title: 'Estimated Total Reward',
    width: '15%',
    tooltipInfo:
      'Estimated node reward (total for the operator and delegators) in the current epoch. There are roughly 24 epochs in a day.',
  },
  {
    field: 'estimatedOperatorReward',
    title: 'Estimated Operator Reward',
    width: '15%',
    tooltipInfo:
      "Estimated operator's reward (including PM and Operating Cost) in the current epoch. There are roughly 24 epochs in a day.",
  },
  {
    field: 'selectionChance',
    title: 'Active Set Probability',
    width: '12.5%',
    tooltipInfo:
      'Probability of getting selected in the reward set (active and standby nodes) in the next epoch. The more your stake, the higher the chances to be selected.',
  },
  {
    field: 'profitMargin',
    title: 'Profit Margin',
    width: '12.5%',
    tooltipInfo:
      'Percentage of the delegators rewards that the operator takes as fee before rewards are distributed to the delegators.',
  },
  {
    field: 'operatingCost',
    title: 'Operating Cost',
    width: '10%',
    tooltipInfo:
      'Monthly operational cost of running this node. This cost is set by the operator and it influences how the rewards are split between the operator and delegators.',
  },
  {
    field: 'nodePerformance',
    title: 'Routing Score',
    width: '10%',
    tooltipInfo:
      "Mixnode's most recent score (measured in the last 15 minutes). Routing score is relative to that of the network. Each time a gateway is tested, the test packets have to go through the full path of the network (gateway + 3 nodes). If a node in the path drop packets it will affect the score of the gateway and other nodes in the test.",
  },
  {
    field: 'avgUptime',
    title: 'Avg. Score',
    tooltipInfo: "Mixnode's average routing score in the last 24 hour",
  },
];
