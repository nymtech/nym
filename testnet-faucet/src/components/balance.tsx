import { Card, CardHeader, Typography } from '@mui/material'
import AttachMoneyIcon from '@mui/icons-material/AttachMoney'
import { Box } from '@mui/system'

export const Balance = ({ balance }: { balance: string }) => {
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
          <Typography variant="h5">
            The total number of available tokens is currently{' '}
            <Typography
              component="span"
              variant="h5"
              sx={{ textDecoration: 'underline' }}
              data-testid="punk-balance-message"
            >
              {balance} PUNKS
            </Typography>
          </Typography>
        }
      />
    </Card>
  )
}
