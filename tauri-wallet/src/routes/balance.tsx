import React, { useContext } from 'react'
import { Button, Grid } from '@material-ui/core'
import { Refresh } from '@material-ui/icons'
import { Layout, NymCard, Page } from '../components'
import { NoClientError } from '../components/NoClientError'
import { Confirmation } from '../components/Confirmation'
import { ClientContext } from '../context/main'

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
                  successMessage={''}
                  failureMessage="Failed to check the account balance!"
                />
              </Grid>
              <Grid item>
                <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
                  <Button
                    variant="contained"
                    color="primary"
                    type="submit"
                    onClick={() => {}}
                    disabled={false}
                    startIcon={<Refresh />}
                  >
                    Refresh
                  </Button>
                </div>
              </Grid>
            </Grid>
          )}
        </NymCard>
      </Layout>
    </Page>
  )
}
