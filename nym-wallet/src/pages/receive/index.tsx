import React, { useContext } from 'react'
import QRCode from 'qrcode.react'
import { Alert, Box, Stack, Typography } from '@mui/material'
import { ClientAddress, CopyToClipboard, NymCard } from '../../components'
import { Layout } from '../../layouts'
import { ClientContext } from '../../context/main'
import { ArrowBack } from '@mui/icons-material'
import { splice } from '../../utils'

export const Receive = () => {
  const { clientDetails, currency } = useContext(ClientContext)

  return (
    <Layout>
      <NymCard title={`Receive ${currency?.major}`} Icon={ArrowBack}>
        <Stack spacing={3} alignItems="center">
          <Alert severity="info" data-testid="receive-nym" sx={{ width: '100%' }}>
            You can receive tokens by providing this address to the sender
          </Alert>
          <Box>
            <ClientAddress withCopy />
          </Box>

          {clientDetails && <QRCode data-testid="qr-code" value={clientDetails?.client_address} />}
        </Stack>
      </NymCard>
    </Layout>
  )
}
