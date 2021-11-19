import { Typography } from '@mui/material'

export const Balance = ({ balance }: { balance: string }) => {
  return (
    <Typography variant="h5">
      The total number of tokens available is currently{' '}
      <Typography
        component="span"
        variant="h5"
        sx={{ textDecoration: 'underline' }}
      >
        {balance} upunks
      </Typography>
    </Typography>
  )
}
