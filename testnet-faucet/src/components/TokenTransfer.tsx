import { Typography } from '@mui/material'

export const TokenTransfer = ({
  address,
  amount,
}: {
  address: string
  amount: string
}) => {
  return (
    <>
      <Typography
        component="span"
        variant="h5"
        sx={{ textDecoration: 'underline' }}
      >
        {amount} upunks
      </Typography>{' '}
      <Typography component="span" variant="h5">
        have been transfered to address
      </Typography>{' '}
      <Typography
        component="span"
        variant="h5"
        sx={{ textDecoration: 'underline' }}
      >
        {address}
      </Typography>
    </>
  )
}
