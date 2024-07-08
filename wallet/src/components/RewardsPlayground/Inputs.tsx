import React, { useCallback } from 'react';
import { yupResolver } from '@hookform/resolvers/yup';
import { Button, Grid, TextField, Typography } from '@mui/material';
import { useForm } from 'react-hook-form';
import { DefaultInputValues } from '@src/pages/bonding/node-settings/apy-playground';
import { inputValidationSchema } from './inputsValidationSchema';

export type InputFields = {
  label: string;
  name: 'profitMargin' | 'uptime' | 'bond' | 'delegations' | 'operatorCost' | 'uptime';
  isPercentage?: boolean;
}[];

export type CalculateArgs = {
  bond: string;
  delegations: string;
  uptime: string;
  profitMargin: string;
  operatorCost: string;
};

const inputFields: InputFields = [
  { label: 'Profit margin', name: 'profitMargin', isPercentage: true },
  { label: 'Operator cost', name: 'operatorCost' },
  { label: 'Bond', name: 'bond' },
  { label: 'Delegations', name: 'delegations' },
  { label: 'Uptime', name: 'uptime', isPercentage: true },
];

export const Inputs = ({
  onCalculate,
  defaultValues,
}: {
  onCalculate: (args: CalculateArgs) => Promise<void>;
  defaultValues: DefaultInputValues;
}) => {
  const handleCalculate = useCallback(
    async (args: CalculateArgs) => {
      onCalculate({
        bond: args.bond,
        delegations: args.delegations,
        uptime: args.uptime,
        profitMargin: args.profitMargin,
        operatorCost: args.operatorCost,
      });
    },
    [onCalculate],
  );

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm({
    resolver: yupResolver(inputValidationSchema),
    defaultValues,
  });

  return (
    <Grid container spacing={3}>
      {inputFields.map((field) => (
        <Grid item xs={12} lg={2} key={field.name}>
          <TextField
            {...register(field.name)}
            fullWidth
            label={field.label}
            name={field.name}
            error={Boolean(errors[field.name])}
            helperText={errors[field.name]?.message}
            InputProps={{
              endAdornment: <Typography sx={{ color: 'grey.600' }}>{field.isPercentage ? '%' : 'NYM'}</Typography>,
            }}
            InputLabelProps={{ shrink: true }}
          />
        </Grid>
      ))}{' '}
      <Grid item xs={12} lg={2}>
        <Button
          variant="contained"
          disableElevation
          onClick={handleSubmit(handleCalculate)}
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
