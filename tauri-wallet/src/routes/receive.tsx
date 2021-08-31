import React, { useContext } from 'react'
import { Card, CardContent, Grid, Typography } from '@material-ui/core'
import { Alert } from '@material-ui/lab'
import { CopyToClipboard, Layout, NymCard, Page } from '../components'
import { useMediaQuery } from '@material-ui/core'
import { theme } from '../theme'
import { ClientContext } from '../context/main'

export const Receive = () => {
  const { clientDetails } = useContext(ClientContext)
  const matches = useMediaQuery('(min-width:769px)')

  return (
    <Page>
      <Layout>
        <NymCard title="Receive Nym">
          <Grid container direction="column" spacing={1}>
            <Grid item>
              <Alert severity="info">
                You can receive tokens by providing this address to the sender
              </Alert>
            </Grid>
            <Grid item>
              <Card
                style={{
                  margin: theme.spacing(1, 0),
                  display: 'flex',
                  justifyContent: 'space-between',
                  flexWrap: 'wrap',
                  padding: theme.spacing(3),
                }}
                variant="outlined"
              >
                <Typography
                  variant={matches ? 'h5' : 'subtitle1'}
                  style={{ wordBreak: 'break-word' }}
                >
                  {clientDetails?.client_address}
                </Typography>
                <CopyToClipboard text={clientDetails?.client_address || ''} />
              </Card>
            </Grid>
          </Grid>
        </NymCard>
      </Layout>
    </Page>
  )
}
