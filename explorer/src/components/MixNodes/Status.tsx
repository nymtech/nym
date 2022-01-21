import { Typography } from '@mui/material';
import * as React from 'react';
import { Theme, useTheme } from '@mui/material/styles';
import { MixnodeRowType } from '.';
import { getMixNodeIcon } from '../Icons';
import { MixnodeStatus } from '../../typeDefs/explorer-api';

interface MixNodeStatusProps {
  status: MixnodeStatus;
}

export const MixNodeStatus: React.FC<MixNodeStatusProps> = ({ status }) => {
  const theme = useTheme();
  const Icon = React.useMemo(() => getMixNodeIcon(status), [status]);
  const color = React.useMemo(() => getMixNodeStatusColor(theme, status), [status, theme]);

  return (
    <Typography color={color} display="flex" alignItems="center">
      <Icon />
      <Typography ml={1} component="span" color="inherit">
        {`${status[0].toUpperCase()}${status.slice(1)}`}
      </Typography>
    </Typography>
  );
};

export const getMixNodeStatusColor = (theme: Theme, status: MixnodeStatus) => {
  let color;
  switch (status) {
    case MixnodeStatus.active:
      color = theme.palette.nym.networkExplorer.mixnodes.status.active;
      break;
    case MixnodeStatus.standby:
      color = theme.palette.nym.networkExplorer.mixnodes.status.standby;
      break;
    default:
      color = theme.palette.nym.networkExplorer.mixnodes.status.inactive;
      break;
  }
  return color;
};

// TODO: should be done with i18n
export const getMixNodeStatusText = (status: MixnodeStatus) => {
  switch (status) {
    case MixnodeStatus.active:
      return 'active';
    case MixnodeStatus.standby:
      return 'on standby';
    default:
      return 'inactive';
  }
};
