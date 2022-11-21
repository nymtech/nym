import React from 'react';
import { Box } from '@mui/material';
import { CustomTitleBar } from './CustomTitleBar';

export const AppWindowFrame: React.FC = ({ children }) => (
  <Box
    sx={{
      display: 'grid',
      borderRadius: '12px',
      gridTemplateRows: '40px 1fr',
      bgcolor: 'nym.background.dark',
      height: '100vh',
    }}
  >
    <CustomTitleBar />
    <Box style={{ padding: '16px' }}>{children}</Box>
  </Box>
);
