import React from 'react'
import { Card, Divider, Grid, Typography } from '@mui/material'
import { useFormContext } from 'react-hook-form'

export const SendReview = ({ transferFee }: { transferFee?: string }) => {
  const { getValues } = useFormContext()

  const values = getValues()

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
          <SendReviewField title="From" subtitle={values.from} />
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
          <SendReviewField title="Amount" subtitle={values.amount + ' punk'} />
        </Grid>
        <Grid item xs={12}>
          <Divider light />
        </Grid>
        <Grid item xs={12}>
          <SendReviewField title="Transfer fee" subtitle={transferFee + ' punk'} info />
        </Grid>
      </Grid>
    </Card>
  )
}

export const SendReviewField = ({ title, subtitle, info }: { title: string; subtitle?: string; info?: boolean }) => {
  return (
    <>
      <Typography sx={{ color: info ? 'nym.info' : '' }}>{title}</Typography>
      <Typography data-testid={title} sx={{ color: info ? 'nym.info' : '', wordBreak: 'break-all' }}>
        {subtitle}
      </Typography>
    </>
  )
}
