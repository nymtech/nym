import { Box, Button, Modal, SxProps, Typography } from '@mui/material';
import React from 'react';
import { modalStyle } from '../Modals/styles';

type Props = {
  message?: string;
  error?: string;
  sx?: SxProps;
  backdropProps?: any;
  open: boolean;
  onClose?: () => void;
  children?: React.ReactNode;
};

export const ErrorModal = ({ message, error, open, onClose, sx, backdropProps, children }: Props) => (
  <Modal open={open} onClose={onClose} BackdropProps={backdropProps}>
    <Box sx={{ ...modalStyle, ...sx }} textAlign="center">
      <Typography color={(theme) => theme.palette.error.main} mb={1}>
        {message}
      </Typography>
      <Typography my={5} color="text.primary">
        {error}
      </Typography>
      {children}
      <Button variant="contained" onClick={onClose}>
        Close
      </Button>
    </Box>
  </Modal>
);
