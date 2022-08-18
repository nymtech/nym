import React, { useContext } from 'react';
import { AppContext } from 'src/context';
import { Box, Stack, Typography, SxProps, Dialog, DialogTitle, DialogContent } from '@mui/material';
import QRCode from 'qrcode.react';
import { ClientAddress } from '../ClientAddress';
import { ModalListItem } from '../Modals/ModalListItem';
import { Close as CloseIcon } from '@mui/icons-material';

export const ReceiveModal = ({ onClose }: { onClose: () => void; sx?: SxProps; backdropProps?: object }) => {
  const { clientDetails } = useContext(AppContext);
  return (
    <Dialog open maxWidth="sm" fullWidth onClose={onClose}>
      <DialogTitle>
        <Box sx={{ mt: 1, display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <Typography fontSize={20} fontWeight={600}>
            Receive
          </Typography>
          <CloseIcon onClick={onClose} cursor="pointer" />
        </Box>
      </DialogTitle>
      <DialogContent sx={{ p: 0 }}>
        <Box sx={{ px: 3 }}>
          <ModalListItem label="Your address:" value={<ClientAddress withCopy showEntireAddress />} />
        </Box>
        <Stack alignItems="center" sx={{ px: 0, py: 3, mt: 3, bgcolor: 'rgba(251, 110, 78, 5%)' }}>
          <Box sx={{ border: (t) => `1px solid ${t.palette.nym.highlight}`, bgcolor: 'white', borderRadius: 2, p: 3 }}>
            {clientDetails && <QRCode data-testid="qr-code" value={clientDetails?.client_address} />}
          </Box>
        </Stack>
      </DialogContent>
    </Dialog>
  );
};
