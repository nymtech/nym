import { useContext } from 'react'
import { CardHeader, Typography } from '@mui/material'
import { CancelOutlined, CheckCircleOutline } from '@mui/icons-material'
import { GlobalContext } from '../context'

export const Balance = () => {
  const { tokensAreAvailable } = useContext(GlobalContext)

  return (
    <CardHeader
      title={
        <Typography
          variant="h6"
          data-testid="nymt-balance-message"
          sx={{
            color: tokensAreAvailable ? 'success.main' : 'error.main',
            fontWeight: 'bold',
          }}
        >
          {tokensAreAvailable
            ? 'Tokens are available'
            : 'Tokens are not currently available'}
        </Typography>
      }
      avatar={
        tokensAreAvailable ? (
          <CheckCircleOutline fontSize="large" color="success" />
        ) : (
          <CancelOutlined fontSize="large" color="error" />
        )
      }
    />
  )
}
