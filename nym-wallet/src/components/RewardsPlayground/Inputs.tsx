import React from 'react';
import { yupResolver } from '@hookform/resolvers/yup';
import { Button, Grid, TextField, Typography } from '@mui/material';
import { useForm } from 'react-hook-form';
import { inputValidationSchema } from './inputsValidationSchema';

export type InputValues = {
  label: string;
  name: 'profitMargin' | 'uptime' | 'bond' | 'delegations' | 'operatorCost';
  isPercentage?: boolean;
}[];

export const Inputs = ({
  inputValues,
  onCalculate,
}: {
  inputValues: InputValues;
  onCalculate: () => Promise<void>;
}) => {
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm({
    resolver: yupResolver(inputValidationSchema),
    defaultValues: { profitMargin: '', uptime: '', bond: '', delegations: '', operatorCost: '' },
  });

  return (
    <Grid container spacing={3}>
      {inputValues.map((input) => (
        <Grid item xs={12} lg={2}>
          <TextField
            {...register(input.name)}
            fullWidth
            label={input.label}
            name={input.name}
            error={Boolean(errors[input.name])}
            helperText={errors[input.name]?.message}
            InputProps={{
              endAdornment: <Typography sx={{ color: 'grey.600' }}>{input.isPercentage ? '%' : 'NYM'}</Typography>,
            }}
          />
        </Grid>
      ))}{' '}
      <Grid item xs={12} lg={2}>
        <Button
          variant="contained"
          disableElevation
          onClick={handleSubmit(onCalculate)}
          size="large"
          fullWidth
          disabled={Boolean(Object.keys(errors).length)}
        >
          Calculate
        </Button>
      </Grid>
    </Grid>
  );
};
