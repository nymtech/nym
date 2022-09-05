import React, { useContext } from 'react';
import { AppContext } from 'src/context';
import { Box, Stack, Typography, SxProps, Dialog, DialogTitle, DialogContent, Paper } from '@mui/material';
import QRCode from 'qrcode.react';
import { ClientAddress } from '../ClientAddress';
import { ModalListItem } from '../Modals/ModalListItem';
import { Close as CloseIcon } from '@mui/icons-material';

export const ReceiveModal = ({ onClose }: { onClose: () => void; sx?: SxProps; backdropProps?: object }) => {
  const { clientDetails, mode } = useContext(AppContext);
  return (
    <Dialog open maxWidth="sm" fullWidth onClose={onClose} PaperComponent={Paper}>
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
        <Stack
          alignItems="center"
          sx={{ px: 0, py: 3, mt: 3, bgcolor: mode === 'light' ? 'rgba(251, 110, 78, 5%)' : 'nym.background.dark' }}
        >
          <Box
            sx={{
              border: (t) => `1px solid ${mode === 'light' ? t.palette.nym.highlight : t.palette.nym.text.grey} `,
              bgcolor: mode === 'light' ? 'white' : 'nym.background.main',
              borderRadius: 2,
              p: 3,
            }}
          >
            {clientDetails && <QRCode data-testid="qr-code" value={clientDetails?.client_address} />}
          </Box>
        </Stack>
      </DialogContent>
    </Dialog>
  );
};
