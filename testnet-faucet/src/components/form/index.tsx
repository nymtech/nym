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
import { majorToMinor } from '../../utils'
import { EnumRequestType, GlobalContext } from '../../context'
import { TokenTransferComplete } from '../token-transfer'

type TFormData = {
  address: string
  amount: string
}

export const Form = ({ withInputField }: { withInputField?: boolean }) => {
  const matches = useMediaQuery('(max-width:500px)')

  const {
    register,
    handleSubmit,
    setValue,
    formState: { errors, isSubmitting },
  } = useForm({
    resolver: yupResolver(validationSchema),
    defaultValues: { address: '', amount: '101' },
  })

  const { requestTokens, loadingState, error, tokensAreAvailable } =
    useContext(GlobalContext)

  const resetForm = () => {
    setValue('address', '')
    setValue('amount', '101')
  }

  const onSubmit: SubmitHandler<TFormData> = async (data) => {
    const unymts = majorToMinor(data.amount)
    await requestTokens({
      address: data.address,
      unymts: unymts.toString(),
      nymts: data.amount,
    })
    resetForm()
  }

  return (
    <Box>
      <TextField
        label="Enter your wallet address"
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
        sx={{ mb: 2, display: withInputField ? 'block' : 'none' }}
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
          disabled={loadingState.isLoading || !tokensAreAvailable}
          data-testid="request-token-button"
        >
          Request 101 NYMT
        </Button>
      </Box>
      {error && <Alert severity="error">{error}</Alert>}
      <TokenTransferComplete />
    </Box>
  )
}
