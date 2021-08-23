import { Card, Divider, Grid, Theme, Typography } from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import React, { useContext } from 'react'
import { ClientContext } from '../../context/main'

export const SendReview = ({
  recipientAddress,
  amount,
}: {
  recipientAddress: string
  amount: string
}) => {
  const { client } = useContext(ClientContext)
  return (
    <Grid container spacing={2}>
      <Grid item xs={12}>
        <SendReviewField title="From" subtitle={client.address} />
      </Grid>
      <Grid item xs={12}>
        <Divider light />
      </Grid>
      <Grid item xs={12}>
        <SendReviewField title="To" subtitle={recipientAddress} />
      </Grid>
      <Grid item xs={12}>
        <Divider light />
      </Grid>
      <Grid item xs={12}>
        <SendReviewField title="Amount" subtitle={amount} />
      </Grid>
    </Grid>
  )
}

const SendReviewField = ({
  title,
  subtitle,
}: {
  title: string
  subtitle: string
}) => {
  const theme: Theme = useTheme()
  return (
    <div style={{ marginBottom: theme.spacing(2) }}>
      <Typography>{title}</Typography>
      <Typography variant="h6" style={{ wordBreak: 'break-all' }}>
        {subtitle}
      </Typography>
    </div>
  )
}
