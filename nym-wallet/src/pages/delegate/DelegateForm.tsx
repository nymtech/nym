import React, { useContext } from 'react'
import { Box, Button, CircularProgress, FormControl, Grid, InputAdornment, TextField, Typography } from '@mui/material'
import { yupResolver } from '@hookform/resolvers/yup'
import { useForm } from 'react-hook-form'
import { EnumNodeType } from '../../types'
import { validationSchema } from './validationSchema'
import { ClientContext } from '../../context/main'
import { delegate, majorToMinor } from '../../requests'
import { checkHasEnoughFunds } from '../../utils'
import { Fee } from '../../components'

type TDelegateForm = {
  nodeType: EnumNodeType
  identity: string
  amount: string
}

const defaultValues: TDelegateForm = {
  nodeType: EnumNodeType.mixnode,
  identity: '',
  amount: '',
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
    watch,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<TDelegateForm>({
    defaultValues,
    resolver: yupResolver(validationSchema),
  })

  const watchNodeType = watch('nodeType', defaultValues.nodeType)

  const { userBalance, currency } = useContext(ClientContext)

  const onSubmit = async (data: TDelegateForm) => {
    const hasEnoughFunds = await checkHasEnoughFunds(data.amount)
    if (!hasEnoughFunds) {
      return setError('amount', {
        message: 'Not enough funds in wallet',
      })
    }

    const amount = await majorToMinor(data.amount)

    await delegate({
      type: data.nodeType,
      identity: data.identity,
      amount,
    })
      .then((res) => {
        onSuccess({ amount: data.amount, address: res.target_address })
        userBalance.fetchBalance()
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

          <Grid item xs={12}>
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
          <Grid item>
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
          onClick={handleSubmit(onSubmit)}
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
