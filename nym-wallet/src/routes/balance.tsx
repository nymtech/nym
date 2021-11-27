import React, { useContext } from 'react'
import {
  Alert,
  Button,
  CircularProgress,
  Grid,
  IconButton,
} from '@mui/material'
import { Box } from '@mui/system'
import { Refresh } from '@mui/icons-material'
import { NymCard } from '../components'
import { Layout } from '../layouts'

import { ClientContext } from '../context/main'

export const Balance = () => {
  const { userBalance } = useContext(ClientContext)

  const RefreshAction = () => (
    <IconButton
      disabled={userBalance.isLoading}
      onClick={userBalance.fetchBalance}
    >
      {userBalance.isLoading ? <CircularProgress size={20} /> : <Refresh />}
    </IconButton>
  )

  return (
    <Layout>
      <NymCard
        title="Balance"
        data-testid="check-balance"
        Action={<RefreshAction />}
      >
        <Grid container direction="column" spacing={2}>
          <Grid item>
            {userBalance.error && (
              <Alert severity="error" data-testid="error-refresh" sx={{ p: 2 }}>
                {userBalance.error}
              </Alert>
            )}
            {!userBalance.error && (
              <Box data-testid="refresh-success" sx={{ p: [2, 3] }}>
                {'The current balance is ' +
                  userBalance.balance?.printable_balance}
              </Box>
            )}
          </Grid>
        </Grid>
      </NymCard>
    </Layout>
  )
}
