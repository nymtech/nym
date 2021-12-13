import React, { useContext } from 'react'
import { Box, Button, CircularProgress, FormControl, Grid, InputAdornment, TextField, Typography } from '@mui/material'
import { useForm } from 'react-hook-form'
import { EnumNodeType, TFee } from '../../types'
import { yupResolver } from '@hookform/resolvers/yup'
import { validationSchema } from './validationSchema'
import { ClientContext, MAJOR_CURRENCY } from '../../context/main'
import { delegate, majorToMinor } from '../../requests'
import { checkHasEnoughFunds } from '../../utils'

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
  fees,
  onError,
  onSuccess,
}: {
  fees: TFee
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

  const { userBalance } = useContext(ClientContext)

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
      <Box sx={{ padding: [3, 5] }}>
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

          <Grid item xs={12} lg={6}>
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
                endAdornment: <InputAdornment position="end">{MAJOR_CURRENCY}</InputAdornment>,
              }}
            />
          </Grid>
          <Grid item xs={12}>
            <Typography sx={{ color: 'nym.info' }}>
              Fee for this transaction: {`${fees.mixnode.amount}  ${MAJOR_CURRENCY}`}
            </Typography>
          </Grid>
        </Grid>
      </Box>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-end',
          borderTop: (theme) => `1px solid ${theme.palette.grey[200]}`,
          bgcolor: 'grey.100',
          padding: 2,
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
        >
          Delegate stake
        </Button>
      </Box>
    </FormControl>
  )
}
