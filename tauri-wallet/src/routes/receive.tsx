import React from 'react'
import { Card, CardContent, Grid, Typography } from '@material-ui/core'
import { CopyToClipboard, Layout, NymCard, Page } from '../components'
import { useMediaQuery } from '@material-ui/core'

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
                <Typography variant="subtitle1" noWrap={false}>
                  You can receive tokens by providing this address to the sender
                </Typography>
              </Grid>
              <Grid item>
                <Card>
                  <CardContent>
                    <div
                      style={{
                        display: 'flex',
                        justifyContent: 'space-between',
                        flexWrap: 'wrap',
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
