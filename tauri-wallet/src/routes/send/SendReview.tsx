import React, { useContext } from 'react'
import { Card, Divider, Grid, Theme, Typography } from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import { useFormContext } from 'react-hook-form'
import { ClientContext } from '../../context/main'

export const SendReview = () => {
  const { gasPrice } = useContext(ClientContext)
  const { getValues } = useFormContext()

  const values = getValues()

  const theme: Theme = useTheme()

  return (
    <Card
      variant="outlined"
      style={{
        width: '100%',
        padding: theme.spacing(2),
        margin: theme.spacing(3, 0),
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
          <SendReviewField
            title="Transfer fee"
            subtitle={gasPrice?.amount + ' PUNK'}
          />
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
  subtitle?: string
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
