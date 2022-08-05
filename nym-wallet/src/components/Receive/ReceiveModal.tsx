import React, { useContext } from 'react';
import { AppContext } from 'src/context';
import { Box, Stack, Typography, SxProps } from '@mui/material';
import QRCode from 'qrcode.react';
import { SimpleModal } from '../Modals/SimpleModal';
import { ClientAddress } from '../ClientAddress';

export const ReceiveModal = ({
  onClose,
  open,
  sx,
  backdropProps,
}: {
  onClose: () => void;
  open: boolean;
  sx?: SxProps;
  backdropProps?: object;
}) => {
  const { clientDetails } = useContext(AppContext);
  return (
    <SimpleModal
      header="Receive"
      okLabel="Ok"
      onClose={onClose}
      open={open}
      sx={{ width: 'small', ...sx }}
      backdropProps={backdropProps}
    >
      <Stack spacing={3} sx={{ mt: 1.6 }}>
        <Stack direction="row" alignItems="center" gap={4}>
          <Typography>Your address:</Typography>
          <ClientAddress withCopy showEntireAddress />
        </Stack>
        <Stack alignItems="center">
          <Box sx={{ border: (t) => `1px solid ${t.palette.nym.highlight}`, borderRadius: 2, p: 2 }}>
            {clientDetails && <QRCode data-testid="qr-code" value={clientDetails?.client_address} />}
          </Box>
        </Stack>
      </Stack>
    </SimpleModal>
  );
};
