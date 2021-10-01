import React from 'react';
import { Box, Grid, Typography } from '@mui/material';
import { MixnodesDataGrid } from 'src/components/Mixnodes-DataGrid';

export const PageMixnodes: React.FC = () => {
  return (
    <>
      <Box component="main" sx={{ flexGrow: 1 }}>
        <Grid container spacing={0}>
          <Grid item xs={12}>
            <Typography sx={{ marginLeft: 3 }}>Mixnodes</Typography>
          </Grid>
          <Grid item xs={11}>
            <MixnodesDataGrid />
          </Grid>
        </Grid>
      </Box>
    </>
  );
};
