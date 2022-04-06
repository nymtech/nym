import { ColumnsType } from '../DetailTable';

export const delegatorsInfoColumns: ColumnsType[] = [
    {
      field: 'estimated_reward',
      title: 'Estimated Reward',
      flex: 1,
      headerAlign: 'left',
    },
    {
      field: 'active_set_probability',
      title: 'Active Set Probability',
      flex: 1,
      headerAlign: 'left',
    },
    {
      field: 'stake_saturation',
      title: 'Stake Saturation',
      flex: 1,
      headerAlign: 'left',
    },
    {
      field: 'profit_margin',
      title: 'Profit Margin',
      flex: 1,
      headerAlign: 'left',
    }
]