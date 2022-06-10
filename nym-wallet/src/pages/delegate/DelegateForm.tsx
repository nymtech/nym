import React, { useContext, useEffect } from 'react';
import { Box, Button, CircularProgress, FormControl, Grid, InputAdornment, TextField } from '@mui/material';
import { yupResolver } from '@hookform/resolvers/yup';
import { useForm } from 'react-hook-form';
import { EnumNodeType, MajorCurrencyAmount, TransactionExecuteResult } from '@nymproject/types';
import { TDelegateArgs } from '../../types';
import { validationSchema } from './validationSchema';
import { AppContext } from '../../context/main';
import { delegateToMixnode, vestingDelegateToMixnode } from '../../requests';
import { Fee, TokenPoolSelector } from '../../components';
import { Console } from '../../utils/console';

type TDelegateForm = {
  identity: string;
  amount: MajorCurrencyAmount;
  tokenPool: string;
  type: EnumNodeType;
};

const defaultValues: TDelegateForm = {
  identity: '',
  amount: { amount: '', denom: 'NYM' },
  tokenPool: 'balance',
  type: EnumNodeType.mixnode,
};

export const DelegateForm = ({
  onError,
  onSuccess,
}: {
  onError: (message?: string) => void;
  onSuccess: (details: { amount: string; result: TransactionExecuteResult }) => void;
}) => {
  const {
    register,
    handleSubmit,
    setValue,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<TDelegateForm>({
    defaultValues,
    resolver: yupResolver(validationSchema),
  });

  const { userBalance, clientDetails } = useContext(AppContext);

  useEffect(() => {
    reset();
  }, [clientDetails]);

  const onSubmit = async (data: TDelegateForm, cb: (data: TDelegateArgs) => Promise<TransactionExecuteResult>) => {
    await cb({
      identity: data.identity,
      amount: { ...data.amount, denom: clientDetails!.denom },
    })
      .then(async (result) => {
        if (data.tokenPool === 'balance') {
          await userBalance.fetchBalance();
        } else {
          await userBalance.fetchTokenAllocation();
        }
        onSuccess({ amount: data.amount.amount, result });
      })
      .catch((e) => {
        Console.error(e as string);
        onError(e);
      });
  };

  return (
    <FormControl fullWidth>
      <Box sx={{ p: 3 }}>
        <Grid container spacing={3}>
          <Grid item xs={12}>
            <TextField
              {...register('identity')}
              required
              variant="outlined"
              id="identity"
              name="identity"
              label="Mixnode identity"
              fullWidth
              error={!!errors.identity}
              helperText={errors?.identity?.message}
            />
          </Grid>

          {userBalance.originalVesting && (
            <Grid item xs={6}>
              <TokenPoolSelector onSelect={(pool) => setValue('tokenPool', pool)} disabled={false} />
            </Grid>
          )}

          <Grid item xs={6}>
            <TextField
              {...register('amount.amount')}
              required
              variant="outlined"
              id="amount"
              name="amount.amount"
              label="Amount to delegate"
              fullWidth
              error={!!errors.amount?.amount}
              helperText={errors?.amount?.amount?.message}
              InputProps={{
                endAdornment: <InputAdornment position="end">{clientDetails?.denom}</InputAdornment>,
              }}
            />
          </Grid>
          <Grid item xs={12}>
            <Fee feeType="DelegateToMixnode" />
          </Grid>
        </Grid>
      </Box>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-end',
          p: 3,
          pt: 0,
        }}
      >
        <Button
          onClick={handleSubmit((data) =>
            onSubmit(data, data.tokenPool === 'balance' ? delegateToMixnode : vestingDelegateToMixnode),
          )}
          disabled={isSubmitting}
          data-testid="delegate-button"
          variant="contained"
          color="primary"
          type="submit"
          disableElevation
          endIcon={isSubmitting && <CircularProgress size={20} />}
          size="large"
        >
          Delegate stake
        </Button>
      </Box>
    </FormControl>
  );
};
