import { Button, TextField, useMediaQuery } from '@mui/material'
import { Box } from '@mui/system'
import { useForm, SubmitHandler } from 'react-hook-form'
import { useValidatorClient } from './useValidtorClient'

type TFormData = {
  address: string
  amount: string
}

export const Form = () => {
  const matches = useMediaQuery('(max-width:500px)')

  const { register, handleSubmit, setValue } = useForm()

  const onSubmit: SubmitHandler<TFormData> = (data) => {
    console.log(data)
    resetForm()
  }

  const resetForm = () => {
    setValue('address', '')
    setValue('amount', '')
  }

  const { getBalance } = useValidatorClient()

  return (
    <>
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
      <Box display="flex" justifyContent="flex-end" flexWrap="wrap">
        <Button
          size="large"
          variant="outlined"
          sx={matches ? { mb: 1 } : { mr: 1 }}
          fullWidth={matches}
          onClick={async () => {
            const balance = await getBalance()
            console.log(balance)
          }}
        >
          Check Balance
        </Button>
        <Button
          size="large"
          variant="contained"
          fullWidth={matches}
          onClick={handleSubmit(onSubmit)}
        >
          Request Tokens
        </Button>
      </Box>
    </>
  )
}
