import React from 'react';
import { Grid } from '@mui/material';
import { NymCard } from '../../components';

export const BondedGatewayCard = () => (
  <NymCard title="Balance">
    <Grid container direction="column" spacing={2}>
      <Grid item>bonded gateway data table</Grid>
    </Grid>
  </NymCard>
);
