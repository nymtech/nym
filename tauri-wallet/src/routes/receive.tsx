import React from 'react'
import { Card, CardContent, Grid, Typography } from '@material-ui/core'
import { Alert } from '@material-ui/lab'
import { CopyToClipboard, Layout, NymCard, Page } from '../components'
import { useMediaQuery } from '@material-ui/core'
import { theme } from '../theme'

export const Receive = () => {
  const matches = useMediaQuery('(min-width:769px)')
  const address = 'Example address here'
  return (
    <Page>
      <Layout>
        <>
          <NymCard title="Receive Nym">
            <Grid container direction="column" spacing={1}>
              <Grid item>
                <Alert severity="info">
                  You can receive tokens by providing this address to the sender
                </Alert>
              </Grid>
              <Grid item>
                <Card style={{ margin: theme.spacing(3, 0) }}>
                  <CardContent>
                    <div
                      style={{
                        display: 'flex',
                        justifyContent: 'space-between',
                        flexWrap: 'wrap',
                        padding: theme.spacing(1),
                      }}
                    >
                      <Typography
                        variant={matches ? 'h5' : 'subtitle1'}
                        style={{ wordBreak: 'break-word' }}
                      >
                        {address}
                      </Typography>
                      <CopyToClipboard text={address} />
                    </div>
                  </CardContent>
                </Card>
              </Grid>
            </Grid>
          </NymCard>
        </>
      </Layout>
    </Page>
  )
}
