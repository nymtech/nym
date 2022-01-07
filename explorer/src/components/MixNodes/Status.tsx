import { Typography } from '@mui/material';
import * as React from 'react';
import { Theme, useTheme } from '@mui/material/styles';
import { MixnodeRowType } from '../MixNodes';
import { getMixNodeIcon } from '../Icons';
import { MixnodeStatus } from '../../typeDefs/explorer-api';

interface MixNodeStatusProps {
  mixNodeRow: MixnodeRowType;
}

export const MixNodeStatus: React.FC<MixNodeStatusProps> = ({ mixNodeRow }) => {
  const theme = useTheme();
  const Icon = React.useMemo(
    () => getMixNodeIcon(mixNodeRow.status),
    [mixNodeRow.status],
  );
  const color = React.useMemo(
    () => getMixNodeStatusColor(theme, mixNodeRow),
    [mixNodeRow.status, theme],
  );

  return (
    <Typography color={color} display="flex" alignItems="center">
      <Icon />
      <Typography ml={1} component="span" color="inherit">
        {`${mixNodeRow.status[0].toUpperCase()}${mixNodeRow.status.slice(1)}`}
      </Typography>
    </Typography>
  );
};

export const getMixNodeStatusColor = (theme: Theme, row: MixnodeRowType) => {
  let color;
  switch (row.status) {
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
export const getMixNodeStatusText = (row: MixnodeRowType) => {
  switch (row.status) {
    case MixnodeStatus.active:
      return 'active';
    case MixnodeStatus.standby:
      return 'on standby';
    default:
      return 'inactive';
  }
};
