import { useContext } from 'react';
import { AppContext } from '@src/context';
import { Box, Stack, SxProps } from '@mui/material';
import QRCode from 'qrcode.react';
import { ClientAddress } from '@nymproject/react';
import { ModalListItem } from '../Modals/ModalListItem';
import { SimpleModal } from '../Modals/SimpleModal';

export const ReceiveModal = ({
  onClose,
  sx,
  backdropProps,
}: {
  onClose: () => void;
  sx?: SxProps;
  backdropProps?: object;
}) => {
  const { clientDetails } = useContext(AppContext);
  return (
    <SimpleModal
      header="Receive"
      open
      onClose={onClose}
      okLabel=""
      sx={sx}
      backdropProps={backdropProps}
      subHeaderStyles={{ mb: 0 }}
    >
      <Stack gap={3} sx={{ position: 'relative', top: '32px' }}>
        <ModalListItem
          label="Your address"
          value={
            clientDetails?.client_address && (
              <ClientAddress address={clientDetails?.client_address} withCopy showEntireAddress />
            )
          }
        />
        <Stack
          alignItems="center"
          sx={{
            position: 'relative',
            left: '-32px',
            width: '598px',
            py: 4,
            bgcolor: (t) => (t.palette.mode === 'dark' ? t.palette.background.default : 'rgba(251, 110, 78, 5%)'),
            borderRadius: '0px 0px 16px 16px',
          }}
        >
          <Box
            sx={{
              border: (t) =>
                t.palette.mode === 'dark'
                  ? `1px solid ${t.palette.nym.nymWallet.modal.border}`
                  : `1px solid ${t.palette.nym.highlight}`,
              bgcolor: (t) => (t.palette.mode === 'dark' ? 'transparent' : 'white'),
              borderRadius: 2,
              p: 3,
            }}
          >
            {clientDetails && <QRCode data-testid="qr-code" value={clientDetails?.client_address} />}
          </Box>
        </Stack>
      </Stack>
    </SimpleModal>
  );
};
