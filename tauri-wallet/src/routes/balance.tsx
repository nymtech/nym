import React, { useContext } from 'react'
import { Button, Grid } from '@material-ui/core'
import { Refresh } from '@material-ui/icons'
import { Layout, NymCard, Page } from '../components'
import { ClientContext } from '../context/main'
import { Alert } from '@material-ui/lab'
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
    >
      Refresh
    </Button>
  )

  return (
    <Page>
      <Layout>
        <NymCard title="Check Balance">
          <Grid container direction="column" spacing={2}>
            <Grid item>
              {balanceError && (
                <Alert severity="error" action={<RefreshAction />}>
                  {balanceError}
                </Alert>
              )}
              {!balanceError && (
                <Alert
                  severity="success"
                  style={{ padding: theme.spacing(2, 3) }}
                  action={<RefreshAction />}
                >
                  {'The current balance is ' + balance?.amount}
                </Alert>
              )}
            </Grid>
          </Grid>
        </NymCard>
      </Layout>
    </Page>
  )
}
