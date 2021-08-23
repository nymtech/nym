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
  const theme: Theme = useTheme()
  return (
    <Card
      variant="outlined"
      style={{ width: '100%', padding: theme.spacing(2) }}
    >
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
    </Card>
  )
}

export const SendReviewField = ({
  title,
  subtitle,
}: {
  title: string
  subtitle: string
}) => {
  const theme: Theme = useTheme()
  return (
    <>
      <Typography style={{ color: theme.palette.grey[600] }}>
        {title}
      </Typography>
      <Typography style={{ wordBreak: 'break-all' }}>{subtitle}</Typography>
    </>
  )
}
