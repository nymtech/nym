import React, { useContext } from 'react'
import { Alert, Button, Grid, Link } from '@mui/material'
import { Box } from '@mui/system'
import { OpenInNew } from '@mui/icons-material'
import { NymCard } from '../components'
import { Layout } from '../layouts'

import { ClientContext, urls } from '../context/main'

export const Balance = () => {
  const { userBalance, clientDetails } = useContext(ClientContext)

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
              <Box data-testid="refresh-success" sx={{ p: 2 }}>
                {'The current balance is ' +
                  userBalance.balance?.printable_balance}
              </Box>
            )}
          </Grid>
          <Grid item>
            <Link
              sx={{ pl: 1 }}
              href={`${urls.blockExplorer}/account/${clientDetails?.client_address}`}
              target="_blank"
            >
              <Button endIcon={<OpenInNew />}>Last transactions</Button>
            </Link>
          </Grid>
        </Grid>
      </NymCard>
    </Layout>
  )
}
