import React, { useContext, useEffect, useState } from 'react'
import {
  Card,
  CircularProgress,
  Divider,
  Grid,
  Theme,
  Typography,
} from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import { useFormContext } from 'react-hook-form'
import { ClientContext } from '../../context/main'
import { getGasFee } from '../../requests'

export const SendReview = () => {
  const { getValues } = useFormContext()
  const [transferFee, setTransferFee] = useState<string>()
  const [isLoadingFee, setIsLoadingFee] = useState(true)

  const values = getValues()

  const theme: Theme = useTheme()

  useEffect(() => {
    const getFee = async () => {
      const fee = await getGasFee('Send')
      setTransferFee(fee.amount)
      setIsLoadingFee(false)
    }
    getFee()
  }, [])

  return (
    <Card
      variant="outlined"
      style={{
        width: '100%',
        padding: theme.spacing(2),
        margin: theme.spacing(3, 0),
      }}
    >
      {isLoadingFee ? (
        <CircularProgress size={48} />
      ) : (
        <Grid container spacing={2}>
          <Grid item xs={12}>
            <SendReviewField title="From" subtitle={values.from}/>
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
              subtitle={transferFee + ' PUNK'}
            />
          </Grid>
        </Grid>
      )}
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
      <Typography data-testid={title} style={{ wordBreak: 'break-all' }}>{subtitle}</Typography>
    </>
  )
}
