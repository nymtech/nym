import React, { useContext } from 'react'
import QRCode from 'qrcode.react'
import { Alert, Box, Stack, Typography } from '@mui/material'
import { CopyToClipboard, NymCard } from '../../components'
import { Layout } from '../../layouts'
import { ClientContext } from '../../context/main'
import { ArrowBack } from '@mui/icons-material'

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
            <Typography
              data-testid="client-address"
              variant="subtitle1"
              sx={{
                wordBreak: 'break-word',
                mr: 1,
              }}
              component="span"
            >
              Your address: {clientDetails?.client_address}
            </Typography>
            <CopyToClipboard text={clientDetails?.client_address || ''} iconButton />
          </Box>

          {clientDetails && <QRCode data-testid="qr-code" value={clientDetails?.client_address} />}
        </Stack>
      </NymCard>
    </Layout>
  )
}
