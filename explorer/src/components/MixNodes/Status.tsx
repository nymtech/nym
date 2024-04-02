import { Typography } from '@mui/material';
import * as React from 'react';
import { getMixNodeIcon } from '@src/components/Icons';
import { MixnodeStatus } from '@src/typeDefs/explorer-api';
import { useGetMixNodeStatusColor } from '@src/hooks/useGetMixnodeStatusColor';

interface MixNodeStatusProps {
  status: MixnodeStatus;
}
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

export const MixNodeStatus: FCWithChildren<MixNodeStatusProps> = ({ status }) => {
  const Icon = React.useMemo(() => getMixNodeIcon(status), [status]);
  const color = useGetMixNodeStatusColor(status);

  return (
    <Typography color={color} display="flex" alignItems="center">
      <Icon />
      <Typography ml={1} component="span" color="inherit">
        {`${status[0].toUpperCase()}${status.slice(1)}`}
      </Typography>
    </Typography>
  );
};
