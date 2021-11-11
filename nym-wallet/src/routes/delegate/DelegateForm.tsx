import React, { useContext } from 'react'
import {
  Button,
  CircularProgress,
  FormControl,
  Grid,
  InputAdornment,
  TextField,
  Theme,
  useTheme,
} from '@material-ui/core'
import { useForm } from 'react-hook-form'
import { EnumNodeType, TFee } from '../../types'
import { yupResolver } from '@hookform/resolvers/yup'
import { validationSchema } from './validationSchema'
import { Alert } from '@material-ui/lab'
import { ClientContext } from '../../context/main'
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
  onSuccess: (message?: string) => void
}) => {
  const theme = useTheme<Theme>()
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

  const { getBalance } = useContext(ClientContext)

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
        onSuccess(
          `Successfully delegated ${data.amount} punk to ${res.target_address}`
        )
        getBalance.fetchBalance()
      })
      .catch((e) => {
        console.log(e)
        onError(e)
      })
  }

  return (
    <FormControl fullWidth>
      <div style={{ padding: theme.spacing(3, 5) }}>
        <Grid container spacing={3}>
          <Grid container item xs={12} justifyContent="space-between">
            <Grid item>
              <Alert severity="info" data-testid="fee-amount">
                {`A fee of ${fees.mixnode.amount} PUNK will apply to this transaction`}
              </Alert>
            </Grid>
          </Grid>
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
                endAdornment: (
                  <InputAdornment position="end">punks</InputAdornment>
                ),
              }}
            />
          </Grid>
        </Grid>
      </div>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-end',
          borderTop: `1px solid ${theme.palette.grey[200]}`,
          background: theme.palette.grey[100],
          padding: theme.spacing(2),
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
      </div>
    </FormControl>
  )
}
