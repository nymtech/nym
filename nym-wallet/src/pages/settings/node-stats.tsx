import React, { useContext } from 'react';
import { OpenInNew } from '@mui/icons-material';
import { Button, Link, Stack, Typography } from '@mui/material';
import { urls, AppContext } from '../../context/main';

export const NodeStats = ({ mixnodeId }: { mixnodeId?: string }) => {
  const { network } = useContext(AppContext);
  return (
    <Stack spacing={2} sx={{ p: 4 }}>
      <Typography>All your node stats are available on the link below</Typography>
      <Link href={`${urls(network).networkExplorer}/network-components/mixnode/${mixnodeId}`} target="_blank">
        <Button endIcon={<OpenInNew />}>Network Explorer</Button>
      </Link>
    </Stack>
  );
};
