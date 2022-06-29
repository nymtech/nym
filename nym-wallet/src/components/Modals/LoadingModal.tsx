import React from 'react';
import { Box, CircularProgress, Modal, Stack, Typography } from '@mui/material';

const modalStyle = {
  position: 'absolute' as 'absolute',
  top: '50%',
  left: '50%',
  transform: 'translate(-50%, -50%)',
  width: 500,
  bgcolor: 'background.paper',
  boxShadow: 24,
  borderRadius: '16px',
  p: 4,
};

export const LoadingModal = () => (
  <Modal open>
    <Box sx={modalStyle} textAlign="center">
      <Stack spacing={4} direction="row" alignItems="center">
        <CircularProgress />
        <Typography>Please wait...</Typography>
      </Stack>
    </Box>
  </Modal>
);
