import React from 'react';
import { Box, Button, Modal, SxProps, Typography } from '@mui/material';
import { modalStyle } from '../../../../nym-wallet/src/components/Modals/styles';

export const ErrorModal: FCWithChildren<{
  open: boolean;
  title?: string;
  message?: string;
  sx?: SxProps;
  backdropProps?: object;
  onClose: () => void;
  children?: React.ReactNode;
}> = ({ children, open, title, message, sx, backdropProps, onClose }) => (
  <Modal open={open} onClose={onClose} BackdropProps={backdropProps}>
    <Box sx={{ border: (t) => `1px solid #fff`, ...modalStyle, ...sx }} textAlign="center">
      <Typography color={(theme) => theme.palette.error.main} mb={1}>
        {title || 'Oh no! Something went wrong...'}
      </Typography>
      <Typography my={5} color="text.primary">
        {message}
      </Typography>
      {children}
      <Button variant="contained" onClick={onClose}>
        Close
      </Button>
    </Box>
  </Modal>
);
