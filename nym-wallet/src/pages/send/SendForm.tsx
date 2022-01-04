import React, { useContext } from 'react'
import { Grid, InputAdornment, TextField, Typography } from '@mui/material'
import { useFormContext } from 'react-hook-form'
import { ClientContext, MAJOR_CURRENCY } from '../../context/main'

export const SendForm = ({ transferFee }: { transferFee?: string }) => {
  const {
    register,
    formState: { errors },
  } = useFormContext()
  const { clientDetails } = useContext(ClientContext)

  return (
    <Grid container spacing={3}>
      <Grid item xs={12}>
        <Typography variant="caption">Your address</Typography>
        <Typography>{clientDetails?.client_address}</Typography>
      </Grid>

      <Grid item xs={12}>
        <TextField
          {...register('to')}
          required
          variant="outlined"
          id="to"
          name="to"
          label="To"
          fullWidth
          autoFocus
          error={!!errors.to}
          helperText={errors.to?.message}
        />
      </Grid>
      <Grid item xs={12}>
        <TextField
          {...register('amount')}
          required
          variant="outlined"
          id="amount"
          name="amount"
          label="Amount"
          fullWidth
          error={!!errors.amount}
          helperText={errors.amount?.message}
          InputProps={{
            endAdornment: <InputAdornment position="end">{MAJOR_CURRENCY}</InputAdornment>,
          }}
        />
      </Grid>
      <Grid item xs={12}>
        <Typography sx={{ color: 'nym.fee' }}>
          Fee for this transaction: {`${transferFee} ${MAJOR_CURRENCY}`}
        </Typography>
      </Grid>
    </Grid>
  )
}
