import React from 'react'
import { Button, Grid, Stack, Typography } from '@mui/material'

export const WelcomeContent = ({ onComplete }: { onComplete: () => void }) => {
  return (
    <>
      <Typography sx={{ color: 'common.white', fontWeight: 600 }}>Welcome to NYM</Typography>
      <Typography variant="caption" sx={{ color: 'grey.800', textTransform: 'uppercase', letterSpacing: 4 }}>
        Next generation of privacy
      </Typography>
      <Stack spacing={3} sx={{width: 300}}>
        <Button fullWidth variant="contained" color="primary" disableElevation size="large" onClick={onComplete}>
          Create Account
        </Button>

        <Button
          fullWidth
          variant="outlined"
          size="large"
          sx={{ color: 'common.white', border: '1px solid white', '&:hover': { border: '1px solid white' } }}
          disableRipple
        >
          Use Existing Account
        </Button>
      </Stack>
    </>
  )
}
