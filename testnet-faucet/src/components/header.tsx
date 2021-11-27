import { Grid, Typography, useMediaQuery } from '@mui/material'
import { Box } from '@mui/system'
import { Balance } from './balance'

export const Header = () => {
  const matches = useMediaQuery('(min-width: 500px)')
  return (
    <Box sx={{ mb: 3, mt: 3 }}>
      <Grid container spacing={1}>
        <Grid item xs={12} md={6}>
          <Typography
            variant="h4"
            sx={{ fontWeight: 'light' }}
            data-testid="token-faucet"
          >
            Nym token faucet
          </Typography>
          {matches && (
            <Typography
              color="primary"
              variant="h3"
              sx={{ fontWeight: 'light' }}
            >
              Tokens to your address
            </Typography>
          )}
        </Grid>
        <Grid item xs={12} md={6}>
          <Balance />
        </Grid>
      </Grid>
    </Box>
  )
}
