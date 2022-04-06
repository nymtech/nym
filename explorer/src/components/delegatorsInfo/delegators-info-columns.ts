import { ColumnsType } from '../DetailTable';

export const delegatorsInfoColumns: ColumnsType[] = [
    {
      field: 'estimated_reward',
      title: 'Estimated Reward',
      flex: 1,
      headerAlign: 'left',
      tooltipInfo: 'Estimated reward per epoch for this profit margin if your node is selected in the active set.',
    },
    {
      field: 'active_set_probability',
      title: 'Active Set Probability',
      flex: 1,
      headerAlign: 'left',
      tooltipInfo: 'Probability of getting selected in the reward set (active and standby nodes) in the next epoch. The more your stake, the higher the chances to be selected.',
    },
    {
      field: 'stake_saturation',
      title: 'Stake Saturation',
      flex: 1,
      headerAlign: 'left',
      tooltipInfo: 'Level of stake saturation for this node. Nodes receive more rewards the higher their saturation level, up to 100%. Beyond 100% no additional rewards are granted. The current stake saturation level is: 1 million NYM, computed as S/K where S is  total amount of tokens available to stakeholders and K is the number of nodes in the reward set.',
    },
    {
      field: 'profit_margin',
      title: 'Profit Margin',
      flex: 1,
      headerAlign: 'left',
    }
]