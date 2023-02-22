import React from 'react';
import { Box, Typography } from '@mui/material';

export const ExperimentalWarning = () => (
  <Box sx={{ color: 'grey.600' }}>
    <Typography fontSize="12px" textAlign="center">
      This is experimental software.
    </Typography>
    <Typography fontSize="12px" textAlign="center">
      Do not rely on it for strong anonymity (yet).
    </Typography>
  </Box>
);
