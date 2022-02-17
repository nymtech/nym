import { Grid, Typography } from '@mui/material'
import { Box } from '@mui/system'
import { Balance } from './balance'

export const Header = () => {
  return (
    <Box sx={{ mb: 3, mt: 3 }}>
      <Grid
        container
        spacing={1}
        alignItems="center"
        justifyContent="space-between"
      >
        <Grid item xs={12} md={8}>
          <Typography
            variant="h3"
            sx={{ fontWeight: 'light' }}
            data-testid="token-faucet"
          >
            Nym testnet Sandbox faucet
          </Typography>
        </Grid>
        <Grid container item xs={12} md={4} justifyContent="flex-end">
          <Balance />
        </Grid>
      </Grid>
    </Box>
  )
}
