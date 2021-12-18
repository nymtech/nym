import React, { useContext } from 'react'
import QRCode from 'qrcode.react'
import { Alert, Box, Card, Grid, Typography, useMediaQuery } from '@mui/material'
import { CopyToClipboard, NymCard } from '../../components'
import { Layout } from '../../layouts'
import { ClientContext } from '../../context/main'

export const Receive = () => {
  const { clientDetails } = useContext(ClientContext)
  const matches = useMediaQuery('(min-width:769px)')

  return (
    <Layout>
      <NymCard title="Receive Nym">
        <Grid container direction="column" spacing={1}>
          <Grid item>
            <Alert severity="info" data-testid="receive-nym">
              You can receive tokens by providing this address to the sender
            </Alert>
          </Grid>
          <Grid item>
            <Card
              sx={{
                m: [1, 0],
                display: 'flex',
                justifyContent: 'space-between',
                flexWrap: 'wrap',
                p: 3,
              }}
              variant="outlined"
            >
              <Grid container direction="column" spacing={4} alignItems="center">
                <Grid item>
                  <Typography
                    data-testid="client-address"
                    variant={matches ? 'h5' : 'subtitle1'}
                    sx={{
                      wordBreak: 'break-word',
                      mr: 1,
                    }}
                    component="span"
                  >
                    {clientDetails?.client_address}
                  </Typography>
                  <CopyToClipboard text={clientDetails?.client_address || ''} />
                </Grid>
                <Grid item>
                  <Box
                    sx={{
                      display: 'flex',
                      justifyContent: 'center',
                      mb: 2,
                    }}
                    component="div"
                  >
                    {clientDetails && <QRCode data-testid="qr-code" value={clientDetails.client_address} />}
                  </Box>
                </Grid>
              </Grid>
            </Card>
          </Grid>
        </Grid>
      </NymCard>
    </Layout>
  )
}
