import React, { useEffect } from 'react'
import { Alert, Button, CircularProgress, Grid } from '@mui/material'
import { Refresh } from '@mui/icons-material'
import { NymCard } from '../components'
import { Layout } from '../layouts'
import { useGetBalance } from '../hooks/useGetBalance'

export const Balance = () => {
  const { balance, isLoading, error, fetchBalance } = useGetBalance()

  useEffect(fetchBalance, [])

  const RefreshAction = () => (
    <Button
      variant="contained"
      size="small"
      color="primary"
      type="submit"
      data-testid="refresh-button"
      onClick={fetchBalance}
      disabled={isLoading}
      disableElevation
      startIcon={<Refresh />}
      endIcon={isLoading && <CircularProgress size={20} />}
      sx={{ mr: 2 }}
    >
      Refresh
    </Button>
  )

  return (
    <Layout>
      <NymCard title="Check Balance" data-testid="check-balance">
        <Grid container direction="column" spacing={2}>
          <Grid item>
            {error && (
              <Alert
                severity="error"
                data-testid="error-refresh"
                action={<RefreshAction />}
                sx={{ p: 2 }}
              >
                {error}
              </Alert>
            )}
            {!error && (
              <Alert
                severity="success"
                data-testid="refresh-success"
                sx={{ p: [2, 3] }}
                action={<RefreshAction />}
              >
                {'The current balance is ' + balance?.printable_balance}
              </Alert>
            )}
          </Grid>
        </Grid>
      </NymCard>
    </Layout>
  )
}
