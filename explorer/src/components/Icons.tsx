import * as React from 'react';
import CheckCircleOutlineIcon from '@mui/icons-material/CheckCircleOutline';
import PauseCircleOutlineIcon from '@mui/icons-material/PauseCircleOutline';
import CircleOutlinedIcon from '@mui/icons-material/CircleOutlined';
import { MixnodeStatus } from '../typeDefs/explorer-api';

export const Icons = {
  mixnodes: {
    status: {
      active: CheckCircleOutlineIcon,
      standby: PauseCircleOutlineIcon,
      inactive: CircleOutlinedIcon,
    },
  },
};

export const getMixNodeIcon = (value: any) => {
  if (value && typeof value === 'string') {
    switch (value) {
      case MixnodeStatus.active:
        return Icons.mixnodes.status.active;
      case MixnodeStatus.standby:
        return Icons.mixnodes.status.standby;
      default:
        return Icons.mixnodes.status.inactive;
    }
  }
  return Icons.mixnodes.status.inactive;
};
