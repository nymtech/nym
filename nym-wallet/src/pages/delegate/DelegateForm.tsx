import React, { useEffect, useContext } from 'react'
import { Box, Button, CircularProgress, FormControl, Grid, InputAdornment, TextField, Typography } from '@mui/material'
import { yupResolver } from '@hookform/resolvers/yup'
import { useForm } from 'react-hook-form'
import { DelegationResult, EnumNodeType, TDelegateArgs } from '../../types'
import { validationSchema } from './validationSchema'
import { ClientContext } from '../../context/main'
import { delegate, majorToMinor, vestingDelegateToMixnode } from '../../requests'
import { Fee, TokenPoolSelector } from '../../components'

type TDelegateForm = {
  identity: string
  amount: string
  tokenPool: string
  type: EnumNodeType
}

const defaultValues: TDelegateForm = {
  identity: '',
  amount: '',
  tokenPool: 'balance',
  type: EnumNodeType.mixnode,
}

export const DelegateForm = ({
  onError,
  onSuccess,
}: {
  onError: (message?: string) => void
  onSuccess: (details: { amount: string; address: string }) => void
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
  })

  const { userBalance, currency, clientDetails } = useContext(ClientContext)

  useEffect(() => {
    reset()
  }, [clientDetails])

  const onSubmit = async (data: TDelegateForm, cb: (data: TDelegateArgs) => Promise<DelegationResult>) => {
    const amount = await majorToMinor(data.amount)

    await cb({
      type: data.type,
      identity: data.identity,
      amount,
    })
      .then(async (res) => {
        if (data.tokenPool === 'balance') {
          await userBalance.fetchBalance()
        } else {
          await userBalance.fetchTokenAllocation()
        }
        onSuccess({ amount: data.amount, address: res.target_address })
      })
      .catch((e) => {
        console.log(e)
        onError(e)
      })
  }

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
              <TokenPoolSelector onSelect={(pool) => setValue('tokenPool', pool)} />
            </Grid>
          )}

          <Grid item xs={6}>
            <TextField
              {...register('amount')}
              required
              variant="outlined"
              id="amount"
              name="amount"
              label="Amount to delegate"
              fullWidth
              error={!!errors.amount}
              helperText={errors?.amount?.message}
              InputProps={{
                endAdornment: <InputAdornment position="end">{currency?.major}</InputAdornment>,
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
            onSubmit(data, data.tokenPool === 'balance' ? delegate : vestingDelegateToMixnode),
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
  )
}
