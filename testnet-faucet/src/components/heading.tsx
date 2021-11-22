import { Typography, useMediaQuery } from '@mui/material'
import { Box } from '@mui/system'

export const Header = () => {
  const matches = useMediaQuery('(min-width: 500px)')
  return (
    <Box sx={{ mb: 5, mt: 3 }}>
      <Typography variant="h4" sx={{ fontWeight: 'light' }} data-testid="token-faucet">
        Nym token faucet
      </Typography>
      {matches && (
        <Typography color="primary" variant="h3" sx={{ fontWeight: 'light' }}>
          Tokens to your address
        </Typography>
      )}
    </Box>
  )
}
