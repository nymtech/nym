import React from 'react';
import { Box, Button, Modal, SxProps, Typography } from '@mui/material';
import { modalStyle } from './styles';

export const ErrorModal: React.FC<{
  open: boolean;
  message?: string;
  sx?: SxProps;
  backdropProps?: object;
  onClose: () => void;
}> = ({ children, open, message, sx, backdropProps, onClose }) => (
  <Modal open={open} onClose={onClose} BackdropProps={backdropProps}>
    <Box sx={{ ...modalStyle, ...sx }} textAlign="center">
      <Typography color={(theme) => theme.palette.error.main} mb={1}>
        Oh no! Something went wrong...
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
