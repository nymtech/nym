import React from 'react';
import { Box, CircularProgress, Modal, Stack, Typography, SxProps } from '@mui/material';
import { modalStyle } from './SimpleModal';

export const LoadingModal: FCWithChildren<{
  text?: string;
  sx?: SxProps;
  backdropProps?: object;
}> = ({ sx, text = 'Please wait...' }) => (
  <Modal open>
    <Box sx={{ ...modalStyle(), ...sx }} textAlign="center">
      <Stack spacing={4} direction="row" alignItems="center">
        <CircularProgress />
        <Typography sx={{ color: 'text.primary' }}>{text}</Typography>
      </Stack>
    </Box>
  </Modal>
);
