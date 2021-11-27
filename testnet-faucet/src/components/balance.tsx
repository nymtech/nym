import { useContext } from 'react'
import {
  Card,
  CardHeader,
  CircularProgress,
  IconButton,
  Typography,
} from '@mui/material'
import RefreshIcon from '@mui/icons-material/Refresh'
import { GlobalContext, EnumRequestType } from '../context'

export const Balance = () => {
  const { balance, loadingState, getBalance } = useContext(GlobalContext)

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
          <Typography variant="h6">
            The current faucet balance is{' '}
            <Typography
              component="span"
              variant="h6"
              data-testid="punk-balance-message"
            >
              {balance} PUNKS
            </Typography>
          </Typography>
        }
        action={
          loadingState.isLoading &&
          loadingState.requestType === EnumRequestType.balance ? (
            <CircularProgress size={12} />
          ) : (
            <IconButton onClick={getBalance}>
              <RefreshIcon />
            </IconButton>
          )
        }
      />
    </Card>
  )
}
