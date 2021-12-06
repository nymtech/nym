import React, { useEffect, useState } from 'react'
import { Card, CircularProgress, Divider, Grid, Theme, Typography } from '@mui/material'
import { useFormContext } from 'react-hook-form'
import { getGasFee } from '../../requests'

export const SendReview = ({ transferFee }: { transferFee?: string }) => {
  const { getValues } = useFormContext()

  const values = getValues()

  return (
    <Card
      variant="outlined"
      sx={{
        width: '100%',
        p: [3, 2],
        m: [3, 0],
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
          <SendReviewField title="Amount" subtitle={values.amount} />
        </Grid>
        <Grid item xs={12}>
          <Divider light />
        </Grid>
        <Grid item xs={12}>
          <SendReviewField title="Transfer fee" subtitle={transferFee + ' PUNK'} info />
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
