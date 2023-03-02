import { ColumnsType } from '../../DetailTable';

export const EconomicsInfoColumns: ColumnsType[] = [
  {
    field: 'estimatedTotalReward',
    title: 'Estimated Total Reward',
    width: 325,
    tooltipInfo:
      'Estimated node reward (total for the operator and delegators) in the current epoch. There are roughly 24 epochs in a day.',
  },
  {
    field: 'estimatedOperatorReward',
    title: 'Estimated Operator Reward',
    width: 350,
    tooltipInfo:
      "Estimated operator's reward (including PM and Operating Cost) in the current epoch. There are roughly 24 epochs in a day.",
  },
  {
    field: 'selectionChance',
    title: 'Active Set Probability',
    width: 290,
    tooltipInfo:
      'Probability of getting selected in the reward set (active and standby nodes) in the next epoch. The more your stake, the higher the chances to be selected.',
  },
  {
    field: 'stakeSaturation',
    title: 'Stake Saturation',
    width: 290,
    tooltipInfo:
      'Level of stake saturation for this node. Nodes receive more rewards the higher their saturation level, up to 100%. Beyond 100% no additional rewards are granted. The current stake saturation level is 730k NYM, computed as S/K where S is target amount of tokens staked in the network and K is the number of nodes in the reward set.',
  },
  {
    field: 'profitMargin',
    title: 'Profit Margin',
    width: 275,
    tooltipInfo:
      'Percentage of the delegators rewards that the operator takes as fee before rewards are distributed to the delegators.',
  },
  {
    field: 'operatingCost',
    title: 'Operating Cost',
    width: 290,
    tooltipInfo:
      'Monthly operational cost of running this node. This cost is set by the operator and it influences how the rewards are split between the operator and delegators.',
  },
  {
    field: 'avgUptime',
    title: 'Average uptime',
    tooltipInfo:
      "Node's routing score is relative to that of the network. Each time a node is tested, the test packets have to go through the full path of the network (a gateway + 3 nodes). If a node in the path drop packets it will affect the score of other nodes in the test.",
  },
];
