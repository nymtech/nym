import React from 'react'
import { Grid, InputAdornment, TextField } from '@material-ui/core'
import { useFormContext } from 'react-hook-form'

export const SendForm = () => {
  const {
    register,
    formState: { errors },
  } = useFormContext()

  return (
    <Grid container spacing={3}>
      <Grid item xs={12}>
        <TextField
          {...register('from')}
          required
          variant="outlined"
          id="from"
          name="from"
          label="From"
          fullWidth
          disabled={true}
        />
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
      <Grid item xs={12} sm={6}>
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
            endAdornment: <InputAdornment position="end">punks</InputAdornment>,
          }}
        />
      </Grid>
    </Grid>
  )
}
