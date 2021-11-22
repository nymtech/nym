import { Card, CardHeader, Typography } from '@mui/material'
import CheckCircleOutlineIcon from '@mui/icons-material/CheckCircleOutline'

export const TokenTransfer = ({
  address,
  amount,
}: {
  address: string
  amount: string
}) => {
  return (
    <Card
      sx={{
        background: 'transparent',
        border: (theme) => `1px solid ${theme.palette.common.white}`,
        p: 2,
        overflow: 'auto',
      }}
    >
      <CardHeader
        title={
          <>
            <Typography component="span" variant="h5">
              Successfully transferred {amount} PUNKS to
            </Typography>{' '}
            <Typography
              component="span"
              variant="h5"
              sx={{ textDecoration: 'underline' }}
              data-testid="success-sent-message"
            >
              {address}
            </Typography>
          </>
        }
      />
    </Card>
  )
}
