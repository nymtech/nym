import React, { useContext } from 'react'
import { Button, Grid } from '@material-ui/core'
import { Alert } from '@material-ui/lab'
import { Refresh } from '@material-ui/icons'
import { NymCard } from '../components'
import { Layout } from '../layouts'
import { ClientContext } from '../context/main'
import { theme } from '../theme'

export const Balance = () => {
  const { balance, balanceError, getBalance } = useContext(ClientContext)

  const RefreshAction = () => (
    <Button
      variant="contained"
      size="small"
      color="primary"
      type="submit"
      onClick={getBalance}
      disabled={false}
      disableElevation
      startIcon={<Refresh />}
      style={{ marginRight: theme.spacing(2) }}
    >
      Refresh
    </Button>
  )

  return (
    <Layout>
      <NymCard title="Check Balance">
        <Grid container direction="column" spacing={2}>
          <Grid item>
            {balanceError && (
              <Alert
                severity="error"
                action={<RefreshAction />}
                style={{ padding: theme.spacing(2) }}
              >
                {balanceError}
              </Alert>
            )}
            {!balanceError && (
              <Alert
                severity="success"
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
