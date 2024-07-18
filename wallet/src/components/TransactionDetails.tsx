import React from 'react';
import { Card, Grid, Typography } from '@mui/material';

export type TTransactionDetails = Array<{ primary: string; secondary: string }>;

export const TransactionDetails: FCWithChildren<{ details: TTransactionDetails }> = ({ details }) => (
  <Card variant="outlined" sx={{ width: '100%', p: 2 }}>
    {details.map(({ primary, secondary }, i) => (
      <Grid container sx={{ mt: i !== 0 ? 1 : 0 }} key={primary}>
        <Grid item sm={4} md={3} lg={2}>
          <Typography sx={{ color: (theme) => theme.palette.grey[600] }}>{primary}</Typography>
        </Grid>
        <Grid item>
          <Typography data-testid="to-address">{secondary}</Typography>
        </Grid>
      </Grid>
    ))}
  </Card>
);
