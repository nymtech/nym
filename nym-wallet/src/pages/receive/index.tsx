import React, { useContext } from 'react';
import QRCode from 'qrcode.react';
import { Alert, Box, Stack } from '@mui/material';
import { ClientAddress, NymCard } from '../../components';
import { ClientContext } from '../../context/main';
import { PageLayout } from '../../layouts';

export const Receive = () => {
  const { clientDetails, currency } = useContext(ClientContext);

  return (
    <PageLayout>
      <NymCard title={`Receive ${currency?.major}`}>
        <Stack spacing={3} alignItems="center">
          <Alert severity="info" data-testid="receive-nym" sx={{ width: '100%' }}>
            You can receive tokens by providing this address to the sender
          </Alert>
          <Box>
            <ClientAddress withCopy showEntireAddress />
          </Box>
          {clientDetails && <QRCode data-testid="qr-code" value={clientDetails?.client_address} />}
        </Stack>
      </NymCard>
    </PageLayout>
  );
};
