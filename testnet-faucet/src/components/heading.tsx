import { Typography } from '@mui/material'
import { Box } from '@mui/system'

export const Heading = () => {
  return (
    <Box sx={{ mb: 5, mt: 3 }}>
      <Typography variant="h4" sx={{ fontWeight: 'light' }}>
        Nym token faucet
      </Typography>
      <Typography color="primary" variant="h3" sx={{ fontWeight: 'light' }}>
        Tokens to your address
      </Typography>
    </Box>
  )
}
