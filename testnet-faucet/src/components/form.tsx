import {
  Alert,
  Button,
  CircularProgress,
  TextField,
  useMediaQuery,
} from '@mui/material'
import { Box } from '@mui/system'
import { useContext } from 'react'
import { useForm, SubmitHandler } from 'react-hook-form'
import { EnumRequestType, GlobalContext } from '../context'
import { Balance } from './Balance'
import { TokenTransfer } from './TokenTransfer'

type TFormData = {
  address: string
  amount: string
}

export const Form = () => {
  const matches = useMediaQuery('(max-width:500px)')

  const { register, handleSubmit, setValue } = useForm()

  const {
    getBalance,
    requestTokens,
    loadingState,
    balance,
    tokenTransfer,
    error,
  } = useContext(GlobalContext)

  const onSubmit: SubmitHandler<TFormData> = async (data) => {
    await requestTokens({ address: data.address, amount: data.amount })
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
      />
      <TextField
        label="Amount"
        fullWidth
        {...register('amount')}
        sx={{ mb: 1 }}
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
          variant="outlined"
          sx={matches ? { mb: 1 } : { mr: 1 }}
          fullWidth={matches}
          endIcon={
            loadingState.requestType === EnumRequestType.balance && (
              <CircularProgress size={20} color="inherit" />
            )
          }
          disabled={loadingState.isLoading}
          onClick={async () => {
            await getBalance()
          }}
        >
          Check Balance
        </Button>
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
        >
          Request Tokens
        </Button>
      </Box>
      {balance && <Balance balance={balance} />}
      {error && <Alert severity="error">{error}</Alert>}
      {tokenTransfer && (
        <TokenTransfer
          address="punk1s63y29jf8f3ft64z0vh80g3c76ty8lnyr74eur"
          amount="1000"
        />
      )}
    </Box>
  )
}
