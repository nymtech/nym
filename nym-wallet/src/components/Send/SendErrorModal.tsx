import React from 'react';
import { SxProps } from '@mui/material';
import { SimpleModal } from '../Modals/SimpleModal';

export const SendErrorModal = ({
  onClose,
  sx,
  backdropProps,
}: {
  onClose: () => void;
  sx?: SxProps;
  backdropProps?: {};
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
  />
);
