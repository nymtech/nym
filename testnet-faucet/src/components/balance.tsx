import { useContext } from 'react'
import { Card, CardHeader, Typography } from '@mui/material'
import { GlobalContext } from '../context'
import { CancelOutlined, CheckCircleOutline } from '@mui/icons-material'

export const Balance = () => {
  const { tokensAreAvailable, balance } = useContext(GlobalContext)
  console.log(balance)
  return (
    <Card
      sx={{
        background: 'transparent',
        border: (theme) => `1px solid ${theme.palette.common.white}`,
        p: 2,
      }}
    >
      <CardHeader
        title={
          <Typography
            component="span"
            variant="h6"
            data-testid="nymt-balance-message"
          >
            {tokensAreAvailable
              ? 'Tokens are available'
              : 'Tokens are not currently available'}
          </Typography>
        }
        action={
          tokensAreAvailable ? (
            <CheckCircleOutline fontSize="large" color="success" />
          ) : (
            <CancelOutlined fontSize="large" color="error" />
          )
        }
      />
    </Card>
  )
}
