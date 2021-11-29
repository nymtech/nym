import React, { useContext } from 'react'
import { Alert, Grid } from '@mui/material'
import { Box } from '@mui/system'
import { NymCard } from '../components'
import { Layout } from '../layouts'

import { ClientContext } from '../context/main'

export const Balance = () => {
  const { userBalance } = useContext(ClientContext)

  return (
    <Layout>
      <NymCard title="Balance" data-testid="check-balance">
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
