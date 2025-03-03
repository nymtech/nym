import { useTheme } from '@mui/material';
import { MixnodeStatus } from '@src/typeDefs/explorer-api';

export const useGetMixNodeStatusColor = (status: MixnodeStatus) => {
  const theme = useTheme();

  switch (status) {
    case MixnodeStatus.active:
      return theme.palette.nym.networkExplorer.mixnodes.status.active;

    case MixnodeStatus.standby:
      return theme.palette.nym.networkExplorer.mixnodes.status.standby;

    default:
      return theme.palette.nym.networkExplorer.mixnodes.status.inactive;
  }
};
