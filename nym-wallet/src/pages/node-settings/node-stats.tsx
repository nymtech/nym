import React, { useContext } from 'react';
import { Stack, Typography } from '@mui/material';
import { TauriLink as Link } from 'src/components/TauriLinkWrapper';

import { urls, AppContext } from '../../context/main';

export const NodeStats = ({ identityKey }: { identityKey?: string }) => {
  const { network } = useContext(AppContext);
  return (
    <Stack spacing={2} sx={{ p: 4 }}>
      <Typography>All your node stats are available on the link below</Typography>
      <Link href={`${urls(network).networkExplorer}/nodes/${identityKey}`} target="_blank" text="Network Explorer" />
    </Stack>
  );
};
