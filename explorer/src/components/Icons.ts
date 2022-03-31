import CheckCircleOutlineIcon from '@mui/icons-material/CheckCircleOutline';
import PauseCircleOutlineIcon from '@mui/icons-material/PauseCircleOutline';
import CircleOutlinedIcon from '@mui/icons-material/CircleOutlined';
import { MixnodeStatus } from '../typeDefs/explorer-api';

export const Icons = {
  Mixnodes: {
    Status: {
      Active: CheckCircleOutlineIcon,
      Standby: PauseCircleOutlineIcon,
      Inactive: CircleOutlinedIcon,
    },
  },
};

export const getMixNodeIcon = (value: any) => {
  if (value && typeof value === 'string') {
    switch (value) {
      case MixnodeStatus.active:
        return Icons.Mixnodes.Status.Active;
      case MixnodeStatus.standby:
        return Icons.Mixnodes.Status.Standby;
      default:
        return Icons.Mixnodes.Status.Inactive;
    }
  }
  return Icons.Mixnodes.Status.Inactive;
};
