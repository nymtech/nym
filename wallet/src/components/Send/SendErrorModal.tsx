import { SxProps, Typography } from '@mui/material';
import { SimpleModal } from '../Modals/SimpleModal';

export const SendErrorModal = ({
  onClose,
  sx,
  backdropProps,
  error,
}: {
  onClose: () => void;
  sx?: SxProps;
  backdropProps?: object;
  error?: string;
}) => (
  <SimpleModal
    open
    hideCloseIcon
    displayErrorIcon
    onOk={async () => onClose()}
    header="Send"
    subHeader="An error occurred while sending. Please try again"
    okLabel="Close"
    sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', ...sx }}
    headerStyles={{
      width: '100%',
      mb: 3,
      textAlign: 'center',
      color: 'error.main',
      fontSize: 16,
      textTransform: 'capitalize',
    }}
    subHeaderStyles={{ textAlign: 'center', color: 'text.primary', fontSize: 14, fontWeight: 400 }}
    backdropProps={backdropProps}
  >
    <Typography variant="body2">{error}</Typography>
  </SimpleModal>
);
