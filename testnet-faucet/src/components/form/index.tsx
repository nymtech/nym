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
    formState: { errors },
  } = useForm({ resolver: yupResolver(validationSchema) })

  console.log(errors)

  const { requestTokens, loadingState, tokenTransfer, error } =
    useContext(GlobalContext)

  const onSubmit: SubmitHandler<TFormData> = async (data) => {
    const uminorcurrency = getCoinValue(data.amount)
    await requestTokens({
      address: data.address,
      utokens: uminorcurrency.toString(),
      majorcurrency: data.amount,
    })
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
        sx={{ mb: 1 }}
        helperText={errors?.address?.message}
        error={!!errors.address}
        data-testid="address"
      />
      <TextField
        label="Amount (tokens)"
        fullWidth
        {...register('amount')}
        sx={{ mb: 1 }}
        helperText={errors?.amount?.message}
        error={!!errors.amount}
        data-testid={'token-amounts'}
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
