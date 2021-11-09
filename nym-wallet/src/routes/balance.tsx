import React, { useEffect } from 'react'
import { Button, CircularProgress, Grid } from '@material-ui/core'
import { Alert } from '@material-ui/lab'
import { Refresh } from '@material-ui/icons'
import { NymCard } from '../components'
import { Layout } from '../layouts'
import { theme } from '../theme'
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
      style={{ marginRight: theme.spacing(2) }}
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
                style={{ padding: theme.spacing(2) }}
              >
                {error}
              </Alert>
            )}
            {!error && (
              <Alert
                severity="success"
                data-testid="refresh-success"
                style={{ padding: theme.spacing(2, 3) }}
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
