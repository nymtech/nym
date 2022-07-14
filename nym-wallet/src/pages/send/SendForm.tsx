import React, { useContext } from 'react';
import { useFormContext } from 'react-hook-form';
import { Grid, TextField } from '@mui/material';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { AppContext } from '../../context/main';

export const SendForm = () => {
  const {
    register,
    setValue,
    formState: { errors },
  } = useFormContext();
  const { clientDetails } = useContext(AppContext);

  return (
    <Grid container spacing={3}>
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
        <CurrencyFormField
          required
          fullWidth
          placeholder="Amount"
          onChanged={(val) => setValue('amount', val, { shouldValidate: true })}
          validationError={errors.amount?.amount?.message}
          denom={clientDetails?.mix_denom}
        />
      </Grid>
    </Grid>
  );
};
