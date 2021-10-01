import React from 'react';
import { Typography } from '@mui/material';
import { MixnodesDataGrid } from 'src/components/Mixnodes-DataGrid';

export const PageMixnodes: React.FC = () => {
  return (
    <>
      <Typography sx={{ marginBottom: 1 }} variant="h5">
        Mixnodes
      </Typography>
      <MixnodesDataGrid />
    </>
  );
};
