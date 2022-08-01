import React, { useContext } from 'react';
import { AppContext } from 'src/context';
import { Box, Stack, Typography } from '@mui/material';
import QRCode from 'qrcode.react';
import { SimpleModal } from '../Modals/SimpleModal';
import { ClientAddress } from '../ClientAddress';

export const ReceiveModal = ({
  onClose,
  hasStorybookStyles,
  open,
}: {
  onClose: () => void;
  hasStorybookStyles?: {};
  open: boolean;
}) => {
  const { clientDetails } = useContext(AppContext);
  return (
    <SimpleModal header="Receive" okLabel="Ok" onClose={onClose} open={open} onOk={async () => onClose()} hideOkButton>
      <Stack spacing={3}>
        <Stack direction="row" alignItems="center" gap={4}>
          <Typography>Your address:</Typography>
          <ClientAddress withCopy showEntireAddress />
        </Stack>
        {clientDetails && <QRCode data-testid="qr-code" value={clientDetails?.client_address} />}
      </Stack>
    </SimpleModal>
  );
};
