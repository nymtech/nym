import React, { useEffect, useState } from 'react'
import {
  Card,
  CircularProgress,
  Divider,
  Grid,
  Theme,
  Typography,
} from '@mui/material'
import { useFormContext } from 'react-hook-form'
import { getGasFee } from '../../requests'

export const SendReview = () => {
  const { getValues } = useFormContext()
  const [transferFee, setTransferFee] = useState<string>()
  const [isLoadingFee, setIsLoadingFee] = useState(true)

  const values = getValues()

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
      sx={{
        width: '100%',
        p: 2,
        m: [3, 0],
      }}
    >
      {isLoadingFee ? (
        <CircularProgress size={48} />
      ) : (
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
  return (
    <>
      <Typography style={{ color: 'grey[600]' }}>{title}</Typography>
      <Typography data-testid={title} style={{ wordBreak: 'break-all' }}>
        {subtitle}
      </Typography>
    </>
  )
}
