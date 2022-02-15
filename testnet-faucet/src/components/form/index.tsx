import { useContext } from 'react'
import {
  Alert,
  Button,
  CircularProgress,
  TextField,
  useMediaQuery,
} from '@mui/material'
import { Box } from '@mui/system'
import { yupResolver } from '@hookform/resolvers/yup'
import { useForm, SubmitHandler } from 'react-hook-form'
import { validationSchema } from './validationSchema'
import { getCoinValue } from '../../utils'
import { EnumRequestType, GlobalContext } from '../../context'
import { TokenTransfer } from '../token-transfer'

type TFormData = {
  address: string
  amount: string
}

export const Form = () => {
  const matches = useMediaQuery('(max-width:500px)')

  const {
    register,
    handleSubmit,
    setValue,
    formState: { errors, isSubmitting },
  } = useForm({ resolver: yupResolver(validationSchema) })

  const { requestTokens, loadingState, tokenTransfer, error } =
    useContext(GlobalContext)

  const onSubmit: SubmitHandler<TFormData> = async (data) => {
    if (+data.amount < 101) {
      const nymts = getCoinValue(data.amount)
      await requestTokens({
        address: data.address,
        unymts: nymts.toString(),
        nymts: data.amount,
      })
    }
    resetForm()
  }

  const resetForm = () => {
    setValue('address', '')
    setValue('amount', '')
  }

  return (
    <Box>
      <TextField
        label="Address"
        fullWidth
        {...register('address')}
        sx={{ mb: 2 }}
        helperText={errors?.address?.message}
        error={!!errors.address}
        data-testid="address"
        disabled={isSubmitting}
      />
      <TextField
        label="Amount (max 101 NYMT)"
        fullWidth
        {...register('amount')}
        sx={{ mb: 2 }}
        helperText={errors?.amount?.message}
        error={!!errors.amount}
        data-testid={'punk-amounts'}
        disabled={isSubmitting}
      />
      <Box
        sx={{
          mb: 5,
          display: 'flex',
          justifyContent: 'flex-end',
          flexWrap: 'wrap',
        }}
      >
        <Button
          size="large"
          variant="contained"
          fullWidth={matches}
          onClick={handleSubmit(onSubmit)}
          endIcon={
            loadingState.requestType === EnumRequestType.tokens && (
              <CircularProgress size={20} color="inherit" />
            )
          }
          disabled={loadingState.isLoading}
          data-testid="request-token-button"
        >
          Request Tokens
        </Button>
      </Box>
      {error && <Alert severity="error">{error}</Alert>}
      {tokenTransfer && (
        <TokenTransfer
          address={tokenTransfer.address}
          amount={tokenTransfer.amount}
        />
      )}
    </Box>
  )
}
