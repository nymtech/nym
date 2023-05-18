import React from 'react';
import { Box, CircularProgress, Modal, Stack, Typography, SxProps } from '@mui/material';

const modalStyle: SxProps = {
  position: 'absolute',
  top: '50%',
  left: '50%',
  transform: 'translate(-50%, -50%)',
  width: 300,
  bgcolor: 'background.paper',
  boxShadow: 24,
  borderRadius: '16px',
  p: 4,
};

export const LoadingModal = ({ sx, backdropProps }: { sx?: SxProps; backdropProps?: object }) => (
  <Modal open BackdropProps={backdropProps}>
    <Box sx={{ border: (t) => `1px solid ${t.palette.grey[500]}`, ...modalStyle, ...sx }} textAlign="center">
      <Stack spacing={4} direction="row" alignItems="center">
        <CircularProgress />
        <Typography sx={{ color: 'text.primary' }}>Please wait...</Typography>
      </Stack>
    </Box>
  </Modal>
);
