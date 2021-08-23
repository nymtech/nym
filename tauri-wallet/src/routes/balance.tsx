import React, { useContext } from 'react'
import { Button, Grid } from '@material-ui/core'
import { Refresh } from '@material-ui/icons'
import { Layout, NymCard, Page } from '../components'
import { NoClientError } from '../components/NoClientError'
import { Confirmation } from '../components/Confirmation'
import { ClientContext } from '../context/main'
import { Alert } from '@material-ui/lab'
import { theme } from '../theme'

export const Balance = () => {
  const { client } = useContext(ClientContext)
  return (
    <Page>
      <Layout>
        <NymCard title="Check Balance">
          {client === null ? (
            <NoClientError />
          ) : (
            <Grid container direction="column" spacing={2}>
              <Grid item>
                <Confirmation
                  isLoading={false}
                  error={null}
                  progressMessage="Checking balance..."
                  SuccessMessage={
                    <Alert
                      severity="success"
                      style={{ padding: theme.spacing(2, 3) }}
                      action={
                        <Button
                          variant="contained"
                          size="small"
                          color="primary"
                          type="submit"
                          onClick={() => {}}
                          disabled={false}
                          disableElevation
                          startIcon={<Refresh />}
                        >
                          Refresh
                        </Button>
                      }
                    >
                      {'The current balance is ' + client.balance}
                    </Alert>
                  }
                  failureMessage="Failed to check the account balance!"
                />
              </Grid>
            </Grid>
          )}
        </NymCard>
      </Layout>
    </Page>
  )
}
