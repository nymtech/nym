import React from 'react';
import { Box, CircularProgress, Modal, Stack, Typography } from '@mui/material';
import { SxProps } from '@mui/system';

const modalStyle = {
  position: 'absolute',
  top: '50%',
  left: '50%',
  transform: 'translate(-50%, -50%)',
  width: 500,
  bgcolor: 'background.paper',
  boxShadow: 24,
  borderRadius: '16px',
  p: 4,
};

export const LoadingModal: React.FC<{
  sx?: SxProps;
  backdropProps?: object;
}> = ({ sx, backdropProps }) => (
  <Modal open BackdropProps={backdropProps}>
    <Box sx={sx ? { ...modalStyle, ...sx } : { ...modalStyle }} textAlign="center">
      <Stack spacing={4} direction="row" alignItems="center">
        <CircularProgress />
        <Typography sx={{ color: 'text.primary' }}>Please wait...</Typography>
      </Stack>
    </Box>
  </Modal>
);
