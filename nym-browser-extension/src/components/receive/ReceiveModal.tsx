import React from 'react';
import { Card, CardContent, Dialog, DialogContent, DialogTitle, IconButton, Stack, Typography } from '@mui/material';
import { QRCodeSVG } from 'qrcode.react';
import { useAppContext } from 'src/context';
import { ClientAddress } from '@nymproject/react/client-address/ClientAddress';
import { Close } from '@mui/icons-material';

export const ReceiveModal = ({ open, onClose }: { open: boolean; onClose: () => void }) => {
  const { client } = useAppContext();
  return (
    <Dialog open={open} onClose={onClose} maxWidth="xl" fullWidth>
      <DialogTitle>
        <Stack direction="row" justifyContent="space-between">
          <Typography fontWeight={700}>Receive</Typography>
          <IconButton size="small" onClick={onClose} sx={{ padding: 0 }}>
            <Close fontSize="small" />
          </IconButton>
        </Stack>
      </DialogTitle>
      <DialogContent>
        <Stack gap={1} alignItems="center">
          <Card elevation={3} sx={{ my: 2, width: 200 }}>
            <CardContent sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <QRCodeSVG value={client?.address || ''} />
            </CardContent>
          </Card>
          <Typography variant="body2" fontWeight={700}>
            Your Nym address
          </Typography>
          <ClientAddress address={client?.address || ''} withCopy smallIcons />
        </Stack>
      </DialogContent>
    </Dialog>
  );
};
