import React, { useContext } from 'react';
import { Card, Divider, Grid, Typography } from '@mui/material';
import { useFormContext } from 'react-hook-form';
import { AppContext } from '../../context/main';

const SendReviewField = ({ title, subtitle, info }: { title: string; subtitle?: string; info?: boolean }) => (
  <>
    <Typography sx={{ color: info ? 'nym.fee' : '' }}>{title}</Typography>
    <Typography data-testid={title} sx={{ color: info ? 'nym.fee' : '', wordBreak: 'break-all' }}>
      {subtitle}
    </Typography>
  </>
);

export const SendReview = ({ transferFee }: { transferFee?: string }) => {
  const { getValues } = useFormContext();
  const { clientDetails } = useContext(AppContext);

  const values = getValues();

  return (
    <Card
      variant="outlined"
      sx={{
        width: '100%',
        py: 3,
        px: 2,
        my: 3,
        mx: 0,
      }}
    >
      <Grid container spacing={2}>
        <Grid item xs={12}>
          <SendReviewField title="From" subtitle={clientDetails?.client_address} />
        </Grid>
        <Grid item xs={12}>
          <Divider light />
        </Grid>
        <Grid item xs={12}>
          <SendReviewField title="To" subtitle={values.to} />
        </Grid>
        <Grid item xs={12}>
          <Divider light />
        </Grid>
        <Grid item xs={12}>
          <SendReviewField title="Amount" subtitle={`${values.amount.amount} ${clientDetails?.denom}`} />
        </Grid>
        <Grid item xs={12}>
          <Divider light />
        </Grid>
        <Grid item xs={12}>
          <SendReviewField title="Transfer fee" subtitle={`${transferFee} ${clientDetails?.denom}`} info />
        </Grid>
      </Grid>
    </Card>
  );
};
