import React, { useState } from 'react'
import { Box } from '@mui/system'
import { Alert, Button, Grid, Stack, Typography } from '@mui/material'
import { NymLogo } from '../../components'
import { WordTile } from './components/word-tile'

export const Welcome = () => {
  const [page, setPage] = useState('welcome')
  return (
    <Box
      sx={{
        height: '100vh',
        width: '100vw',
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        overflow: 'auto',
        bgcolor: 'nym.background.dark',
      }}
    >
      <Box
        sx={{
          width: '100%',
          display: 'flex',
          justifyContent: 'center',
          margin: 'auto',
        }}
      >
        <Stack spacing={3} alignItems="center" sx={{ width: 400 }}>
          <NymLogo />
          {page === 'welcome' && (
            <>
              <Typography sx={{ color: 'common.white', fontWeight: 600 }}>Welcome to NYM</Typography>
              <Typography variant="caption" sx={{ color: 'grey.800', textTransform: 'uppercase', letterSpacing: 4 }}>
                Next generation of privacy
              </Typography>
              <Grid container direction="column" spacing={2}>
                <Grid item>
                  <Button
                    fullWidth
                    variant="contained"
                    color="primary"
                    disableElevation
                    size="large"
                    onClick={() => setPage('create account')}
                  >
                    Create Account
                  </Button>
                </Grid>
                <Grid item>
                  <Button
                    fullWidth
                    variant="outlined"
                    size="large"
                    sx={{ color: 'common.white', border: '1px solid white', '&:hover': { border: '1px solid white' } }}
                    disableRipple
                  >
                    Use Existing Account
                  </Button>
                </Grid>
              </Grid>
            </>
          )}

          {page === 'create account' && (
            <>
              <Alert icon={false} severity="info" sx={{ bgcolor: '#18263B', color: '#50ABFF', width: 625 }}>
                Please store your mnemonic in a safe place. This is the only way to access your wallet!
              </Alert>
              <WordTile />
              <Button
                variant="contained"
                color="primary"
                disableElevation
                size="large"
                onClick={() => setPage('create account')}
              >
                Verify mnemonic
              </Button>
            </>
          )}
        </Stack>
      </Box>
    </Box>
  )
}
